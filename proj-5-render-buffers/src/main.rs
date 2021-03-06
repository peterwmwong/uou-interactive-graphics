#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{components::Camera, metal::*, metal_types::*, pipeline::*, *};
use proj_4_textures::Delegate as Proj4Delegate;
use shader_bindings::*;
use std::{
    f32::consts::PI,
    simd::{f32x2, SimdFloat},
};

const INITIAL_PLANE_TEXTURE_FILTER_MODE: TextureFilterMode = TextureFilterMode::Anistropic;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 32., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

struct CheckerboardDelegate {
    command_queue: CommandQueue,
    pub device: Device,
    needs_render: bool,
    render_pipeline:
        RenderPipeline<1, checkerboard_vertex, checkerboard_fragment, (NoDepth, NoStencil)>,
}

impl RendererDelgate for CheckerboardDelegate {
    fn new(device: Device) -> Self {
        Self {
            command_queue: device.new_command_queue(),
            needs_render: true,
            render_pipeline: RenderPipeline::new(
                "Checkerboard",
                &device,
                &device
                    .new_library_with_data(LIBRARY_BYTES)
                    .expect("Failed to import shader metal lib."),
                [(DEFAULT_COLOR_FORMAT, BlendMode::NoBlend)],
                checkerboard_vertex,
                checkerboard_fragment,
                (NoDepth, NoStencil),
            ),
            device,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.needs_render = false;
        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        self.render_pipeline.new_pass(
            "Checkboard",
            command_buffer,
            [(
                render_target,
                (0., 0., 0., 0.),
                MTLLoadAction::Clear,
                MTLStoreAction::Store,
            )],
            NoDepth,
            NoStencil,
            NoDepthState,
            &[],
            |p| p.draw_primitives(MTLPrimitiveType::TriangleStrip, 0, 4),
        );
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
    m_model_to_world: f32x4x4,
    m_model_to_projection: f32x4x4,
    render_pipeline_state: RenderPipeline<1, main_vertex, main_fragment, (NoDepth, NoStencil)>,
    plane_renderer: R,
    plane_texture: Option<Texture>,
    plane_texture_filter_mode: TextureFilterMode,
    needs_render: bool,
}

impl<R: RendererDelgate> RendererDelgate for Delegate<R> {
    fn new(device: Device) -> Self {
        Self {
            camera: Camera::new_with_default_distance(
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            command_queue: device.new_command_queue(),
            m_model_to_projection: f32x4x4::identity(),
            m_model_to_world: f32x4x4::y_rotate(PI)
                * f32x4x4::x_rotate(PI / 2.)
                * f32x4x4::scale(0.5, 0.5, 0.5, 1.),
            needs_render: false,
            plane_texture_filter_mode: INITIAL_PLANE_TEXTURE_FILTER_MODE,
            plane_texture: None,
            render_pipeline_state: RenderPipeline::new(
                "Plane",
                &device,
                &device
                    .new_library_with_data(LIBRARY_BYTES)
                    .expect("Failed to import shader metal lib."),
                [(DEFAULT_COLOR_FORMAT, BlendMode::NoBlend)],
                main_vertex,
                main_fragment,
                (NoDepth, NoStencil),
            ),
            plane_renderer: R::new(device),
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
        self.render_pipeline_state.new_pass(
            "Plane",
            command_buffer,
            [(
                render_target,
                (0., 0., 0., 0.),
                MTLLoadAction::Clear,
                MTLStoreAction::Store,
            )],
            NoDepth,
            NoStencil,
            NoDepthState,
            &[],
            |p| {
                p.draw_primitives_with_bind(
                    main_vertex_binds {
                        m_model_to_projection: Bind::Value(&self.m_model_to_projection),
                    },
                    main_fragment_binds {
                        texture: BindTexture::Texture(plane_texture),
                        mode: Bind::Value(&self.plane_texture_filter_mode),
                    },
                    MTLPrimitiveType::TriangleStrip,
                    0,
                    4,
                )
            },
        );
        command_buffer
    }

    #[inline]
    fn on_event(&mut self, event: UserEvent) {
        use UserEvent::*;

        if let Some(u) = self.camera.on_event(event) {
            self.m_model_to_projection = u.m_world_to_projection * self.m_model_to_world;
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
        desc.set_pixel_format(DEFAULT_COLOR_FORMAT);
        desc.set_usage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
        let plane_texture = self.device().new_texture(&desc);
        plane_texture.set_label("Plane Texture");
        self.plane_texture = Some(plane_texture);
        self.plane_renderer
            .on_event(UserEvent::WindowFocusedOrResized { size: plane_size });
    }
}

fn main() {
    const APP_NAME: &'static str = &"Project 5 - Render Buffers";
    // TODO: BUHAHHAHA... this cannot be good... Probably bloats binary size by generating code for
    // a whole application times 2.
    if std::env::args().len() >= 2 {
        launch_application::<Delegate<Proj4Delegate<false>>>(APP_NAME);
    } else {
        launch_application::<Delegate<CheckerboardDelegate>>(APP_NAME);
    }
}
