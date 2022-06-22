#![feature(array_zip)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{metal::*, *};
use metal_build::metal_types::*;
use proj_4_textures::Delegate as Proj4Delegate;
use shader_bindings::*;
use std::{f32::consts::PI, ops::Neg, simd::f32x2};

const INITIAL_PLANE_TEXTURE_FILTER_MODE: TextureFilterMode = TextureFilterMode::Anistropic;
const INITIAL_CAMERA_DISTANCE: f32 = 0.5;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 32., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

const N: f32 = 0.001;
const F: f32 = 100000.0;
const NEAR_FIELD_MAJOR_AXIS: f32 = N / INITIAL_CAMERA_DISTANCE;
const PERSPECTIVE_MATRIX: f32x4x4 = f32x4x4::new(
    [N, 0., 0., 0.],
    [0., N, 0., 0.],
    [0., 0., N + F, -N * F],
    [0., 0., 1., 0.],
);

struct CheckerboardDelegate {
    command_queue: CommandQueue,
    pub device: Device,
    needs_render: bool,
    render_pipeline_state: RenderPipelineState,
}

impl RendererDelgate for CheckerboardDelegate {
    fn new(device: Device) -> Self {
        Self {
            command_queue: device.new_command_queue(),
            needs_render: true,
            render_pipeline_state: create_pipeline(
                &device,
                &device
                    .new_library_with_data(LIBRARY_BYTES)
                    .expect("Failed to import shader metal lib."),
                &new_basic_render_pipeline_descriptor(DEFAULT_PIXEL_FORMAT, None, false),
                "Checkerboard",
                None,
                &"checkerboard_vertex",
                0,
                &"checkerboard_fragment",
                0,
            )
            .pipeline_state,
            device,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.needs_render = false;
        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        let encoder = command_buffer
            .new_render_command_encoder(new_basic_render_pass_descriptor(render_target, None));
        // Render Plane
        {
            encoder.push_debug_group("Checkerboard");
            encoder.set_render_pipeline_state(&self.render_pipeline_state);
            encoder.draw_primitives(MTLPrimitiveType::TriangleStrip, 0, 4);
            encoder.pop_debug_group();
        }
        encoder.end_encoding();
        command_buffer
    }

    #[inline(always)]
    fn needs_render(&self) -> bool {
        self.needs_render
    }

    #[inline(always)]
    fn on_event(&mut self, event: UserEvent) {
        self.needs_render = matches!(event, UserEvent::WindowResize { .. });
    }

    fn device(&self) -> &Device {
        &self.device
    }
}

struct Delegate<R: RendererDelgate> {
    camera_distance: f32,
    camera_rotation: f32x2,
    command_queue: CommandQueue,
    matrix_model_to_world: f32x4x4,
    matrix_model_to_projection: f32x4x4,
    render_pipeline_state: RenderPipelineState,
    plane_renderer: R,
    plane_texture: Option<Texture>,
    plane_texture_filter_mode: TextureFilterMode,
    screen_size: f32x2,
    needs_render: bool,
}

impl<R: RendererDelgate> RendererDelgate for Delegate<R> {
    fn new(device: Device) -> Self {
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        let render_pipeline_state = {
            create_pipeline(
                &device,
                &library,
                &new_basic_render_pipeline_descriptor(DEFAULT_PIXEL_FORMAT, None, false),
                "Plane",
                None,
                &"main_vertex",
                VertexBufferIndex::LENGTH as _,
                &"main_fragment",
                FragBufferIndex::LENGTH as _,
            )
            .pipeline_state
        };
        let matrix_model_to_world =
            f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.) * f32x4x4::scale(0.5, 0.5, 0.5, 1.);
        let command_queue = device.new_command_queue();

        let mut delegate = Self {
            camera_distance: INITIAL_CAMERA_DISTANCE,
            camera_rotation: INITIAL_CAMERA_ROTATION,
            command_queue,
            matrix_model_to_projection: f32x4x4::identity(),
            matrix_model_to_world,
            needs_render: false,
            plane_renderer: R::new(device),
            plane_texture_filter_mode: INITIAL_PLANE_TEXTURE_FILTER_MODE,
            plane_texture: None,
            render_pipeline_state,
            screen_size: f32x2::default(),
            // plane_renderer: Proj4Delegate::<false>::new(device),
        };
        delegate.update_camera(
            delegate.screen_size,
            delegate.camera_rotation,
            delegate.camera_distance,
        );
        delegate.reset_needs_render();
        delegate
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.reset_needs_render();

        let plane_texture = self
            .plane_texture
            .as_ref()
            .expect("Failed to load Plane Texture");

        // If the Plane needs to render, use that command buffer so rendering is in sync.
        // Otherwise, create a new command buffer.
        let command_buffer = if self.plane_renderer.needs_render() {
            let command_buffer = self.plane_renderer.render(plane_texture);
            {
                let encoder = command_buffer.new_blit_command_encoder();
                encoder.set_label("Plane Texture Generate Mipmaps");
                encoder.generate_mipmaps(plane_texture);
                encoder.end_encoding();
            }
            command_buffer
        } else {
            self.command_queue
                .new_command_buffer_with_unretained_references()
        };
        command_buffer.set_label("Renderer Command Buffer");
        let encoder = command_buffer
            .new_render_command_encoder(new_basic_render_pass_descriptor(render_target, None));
        // Render Plane
        {
            encoder.push_debug_group("Plane");
            encoder.set_render_pipeline_state(&self.render_pipeline_state);
            encode_vertex_bytes(
                encoder,
                VertexBufferIndex::MatrixModelToProjection as _,
                &self.matrix_model_to_projection,
            );
            encoder.set_fragment_texture(FragBufferIndex::Texture as _, Some(plane_texture));
            encode_fragment_bytes(
                encoder,
                FragBufferIndex::TextureFilterMode as _,
                &self.plane_texture_filter_mode,
            );
            encoder.draw_primitives(MTLPrimitiveType::TriangleStrip, 0, 4);
            encoder.pop_debug_group();
        }
        encoder.end_encoding();
        command_buffer
    }

    #[inline]
    fn on_event(&mut self, event: UserEvent) {
        use MouseButton::*;
        use UserEvent::*;
        match event {
            MouseDrag {
                button,
                modifier_keys,
                drag_amount,
                ..
            } => {
                if modifier_keys.contains(ModifierKeys::ALT_OPTION) {
                    self.plane_renderer
                        .on_event(remove_modifier_keys(event, ModifierKeys::ALT_OPTION))
                } else if modifier_keys.is_empty() {
                    let mut camera_rotation = self.camera_rotation;
                    let mut camera_distance = self.camera_distance;
                    match button {
                        Left => {
                            camera_rotation += {
                                let adjacent = f32x2::splat(self.camera_distance);
                                let opposite = drag_amount / f32x2::splat(500.);
                                let &[x, y] = (opposite / adjacent).as_array();
                                f32x2::from_array([
                                    y.atan(), // Rotation on x-axis
                                    x.atan(), // Rotation on y-axis
                                ])
                            }
                        }
                        Right => camera_distance += -drag_amount[1] / 250.,
                    }
                    self.update_camera(self.screen_size, camera_rotation, camera_distance);
                }
            }
            KeyDown {
                key_code,
                modifier_keys,
            } => {
                if modifier_keys.contains(ModifierKeys::ALT_OPTION) {
                    self.plane_renderer
                        .on_event(remove_modifier_keys(event, ModifierKeys::ALT_OPTION))
                } else {
                    self.update_plane_texture_filter_mode(match key_code {
                        29 /* 0 */ => TextureFilterMode::Anistropic,
                        18 /* 1 */ => TextureFilterMode::Nearest,
                        19 /* 2 */ => TextureFilterMode::Linear,
                        20 /* 3 */ => TextureFilterMode::Mipmap,
                        21 /* 4 */ => TextureFilterMode::Anistropic,
                        _ => self.plane_texture_filter_mode
                    });
                }
            }
            WindowResize { size, .. } => {
                self.plane_renderer.on_event(event);
                self.update_plane_texture_size(size);
                self.update_camera(size, self.camera_rotation, self.camera_distance);
            }
            _ => {}
        }
    }

    #[inline(always)]
    fn needs_render(&self) -> bool {
        self.needs_render || self.plane_renderer.needs_render()
    }

    fn device(&self) -> &Device {
        self.plane_renderer.device()
    }
}

impl<R: RendererDelgate> Delegate<R> {
    #[inline(always)]
    fn device(&self) -> &Device {
        &self.plane_renderer.device()
    }

    #[inline(always)]
    fn reset_needs_render(&mut self) {
        self.needs_render = false;
    }

    #[inline]
    fn calc_matrix_camera_to_projection(&self, aspect_ratio: f32) -> f32x4x4 {
        let (w, h) = (NEAR_FIELD_MAJOR_AXIS, aspect_ratio * NEAR_FIELD_MAJOR_AXIS);
        let orthographic_matrix = {
            f32x4x4::new(
                [2. / w, 0., 0., 0.],
                [0., 2. / h, 0., 0.],
                // IMPORTANT: Metal's NDC coordinate space has a z range of [0.,1], **NOT [-1,1]** (OpenGL).
                [0., 0., 1. / (F - N), -N / (F - N)],
                [0., 0., 0., 1.],
            )
        };
        orthographic_matrix * PERSPECTIVE_MATRIX
    }

    #[inline]
    fn update_plane_texture_filter_mode(&mut self, mode: TextureFilterMode) {
        self.needs_render = mode != std::mem::replace(&mut self.plane_texture_filter_mode, mode);
    }

    fn update_plane_texture_size(&mut self, size: f32x2) {
        let plane_size = f32x2::splat(size.reduce_max());

        let desc = TextureDescriptor::new();
        desc.set_width(plane_size[0] as _);
        desc.set_height(plane_size[0] as _);
        // TODO: What is the optimal mip-map level count?
        desc.set_mipmap_level_count(6);
        desc.set_pixel_format(DEFAULT_PIXEL_FORMAT);
        desc.set_usage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
        let plane_texture = self.device().new_texture(&desc);
        plane_texture.set_label("Plane Texture");
        self.plane_texture = Some(plane_texture);
        self.plane_renderer
            .on_event(UserEvent::WindowResize { size: plane_size });
    }

    fn update_camera(&mut self, screen_size: f32x2, camera_rotation: f32x2, camera_distance: f32) {
        self.screen_size = screen_size;
        self.camera_rotation = camera_rotation;
        self.camera_distance = camera_distance;
        let &[rotx, roty] = self.camera_rotation.neg().as_array();
        let matrix_world_to_camera =
            f32x4x4::translate(0., 0., self.camera_distance) * f32x4x4::rotate(rotx, roty, 0.);
        let &[sx, sy, ..] = screen_size.as_array();
        let aspect_ratio = sy / sx;
        let matrix_world_to_projection =
            self.calc_matrix_camera_to_projection(aspect_ratio) * matrix_world_to_camera;
        self.matrix_model_to_projection = matrix_world_to_projection * self.matrix_model_to_world;
        self.needs_render = true;
    }
}

pub fn run() {
    const APP_NAME: &'static str = &"Project 5 - Render Buffers";
    // TODO: BUHAHHAHA... this cannot be good... Probably bloats binary size by generating code for
    // a whole application times 2.
    if std::env::args().len() >= 2 {
        launch_application::<Delegate<Proj4Delegate<false>>>(APP_NAME);
    } else {
        launch_application::<Delegate<CheckerboardDelegate>>(APP_NAME);
    }
}
