#![feature(array_zip)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{components::Camera, metal::*, metal_types::*, *};
use proj_4_textures::Delegate as Proj4Delegate;
use shader_bindings::*;
use std::{f32::consts::PI, simd::f32x2};

const INITIAL_PLANE_TEXTURE_FILTER_MODE: TextureFilterMode = TextureFilterMode::Anistropic;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 32., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

struct CheckerboardDelegate {
    command_queue: CommandQueue,
    pub device: Device,
    needs_render: bool,
    render_pipeline: RenderPipelineState,
}

impl RendererDelgate for CheckerboardDelegate {
    fn new(device: Device) -> Self {
        Self {
            command_queue: device.new_command_queue(),
            needs_render: true,
            render_pipeline: create_render_pipeline(
                &device,
                &new_render_pipeline_descriptor(
                    "Checkerboard",
                    &device
                        .new_library_with_data(LIBRARY_BYTES)
                        .expect("Failed to import shader metal lib."),
                    Some((DEFAULT_PIXEL_FORMAT, false)),
                    None,
                    None,
                    Some((&"checkerboard_vertex", 0)),
                    Some((&"checkerboard_fragment", 0)),
                ),
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
            .new_render_command_encoder(new_render_pass_descriptor(Some(render_target), None));
        // Render Plane
        {
            encoder.push_debug_group("Checkerboard");
            encoder.set_render_pipeline_state(&self.render_pipeline);
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
        self.needs_render = matches!(event, UserEvent::WindowFocusedOrResized { .. });
    }

    fn device(&self) -> &Device {
        &self.device
    }
}

struct Delegate<R: RendererDelgate> {
    camera: Camera,
    command_queue: CommandQueue,
    matrix_model_to_world: f32x4x4,
    matrix_model_to_projection: f32x4x4,
    render_pipeline_state: RenderPipelineState,
    plane_renderer: R,
    plane_texture: Option<Texture>,
    plane_texture_filter_mode: TextureFilterMode,
    needs_render: bool,
}

impl<R: RendererDelgate> RendererDelgate for Delegate<R> {
    fn new(device: Device) -> Self {
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        let render_pipeline_state = {
            let p = create_render_pipeline(
                &device,
                &new_render_pipeline_descriptor(
                    "Plane",
                    &library,
                    Some((DEFAULT_PIXEL_FORMAT, false)),
                    None,
                    None,
                    Some((&"main_vertex", VertexBufferIndex::LENGTH as _)),
                    Some((&"main_fragment", FragBufferIndex::LENGTH as _)),
                ),
            );
            use debug_assert_pipeline_function_arguments::*;
            debug_assert_render_pipeline_function_arguments(
                &p,
                &[value_arg::<float4x4>(
                    VertexBufferIndex::MatrixModelToProjection as _,
                )],
                None,
            );
            p.pipeline_state
        };
        let matrix_model_to_world =
            f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.) * f32x4x4::scale(0.5, 0.5, 0.5, 1.);
        let command_queue = device.new_command_queue();

        Self {
            camera: Camera::new_with_default_distance(
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            command_queue,
            matrix_model_to_projection: f32x4x4::identity(),
            matrix_model_to_world,
            needs_render: false,
            plane_renderer: R::new(device),
            plane_texture_filter_mode: INITIAL_PLANE_TEXTURE_FILTER_MODE,
            plane_texture: None,
            render_pipeline_state,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.needs_render = false;

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
            .new_render_command_encoder(new_render_pass_descriptor(Some(render_target), None));
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
        use UserEvent::*;

        if let Some(update) = self.camera.on_event(event) {
            self.matrix_model_to_projection =
                update.matrix_world_to_projection * self.matrix_model_to_world;
            self.needs_render = true;
        }

        match event {
            MouseDrag { modifier_keys, .. } => {
                if modifier_keys.contains(ModifierKeys::ALT_OPTION) {
                    self.plane_renderer
                        .on_event(remove_modifier_keys(event, ModifierKeys::ALT_OPTION))
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
            WindowFocusedOrResized { size, .. } => {
                self.plane_renderer.on_event(event);
                self.update_plane_texture_size(size);
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
            .on_event(UserEvent::WindowFocusedOrResized { size: plane_size });
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
