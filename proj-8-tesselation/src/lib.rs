#![feature(array_zip)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{components::camera, metal::*, metal_types::*, *};
use shader_bindings::*;
use std::{f32::consts::PI, simd::f32x2};

const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 5., PI / 16.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

impl Default for Space {
    #[inline]
    fn default() -> Self {
        Self {
            matrix_world_to_projection: f32x4x4::identity(),
            matrix_screen_to_world: f32x4x4::identity(),
            position_world: float4 { xyzw: [0.; 4] },
        }
    }
}

struct Delegate {
    camera_space: Space,
    camera: camera::Camera,
    command_queue: CommandQueue,
    device: Device,
    light_space: Space,
    light: camera::Camera,
    needs_render: bool,
    pipeline_state: RenderPipelineState,
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        Self {
            camera_space: Default::default(),
            camera: camera::Camera::new(INITIAL_CAMERA_ROTATION, ModifierKeys::empty(), false, 0.),
            command_queue: device.new_command_queue(),
            light_space: Default::default(),
            light: camera::Camera::new(INITIAL_LIGHT_ROTATION, ModifierKeys::CONTROL, true, 1.),
            needs_render: false,
            pipeline_state: {
                let p = create_pipeline(
                    &device,
                    &library,
                    &mut new_basic_render_pipeline_descriptor(DEFAULT_PIXEL_FORMAT, None, false),
                    "Plane",
                    None,
                    (&"main_vertex", VertexBufferIndex::LENGTH as _),
                    Some((&"main_fragment", FragBufferIndex::LENGTH as _)),
                );
                debug_assert_argument_buffer_size::<{ FragBufferIndex::CameraSpace as _ }, Space>(
                    &p,
                    FunctionType::Fragment,
                );
                debug_assert_argument_buffer_size::<{ FragBufferIndex::LightSpace as _ }, Space>(
                    &p,
                    FunctionType::Fragment,
                );
                p.pipeline_state
            },
            device,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.needs_render = false;

        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        command_buffer.set_label("Renderer Command Buffer");

        // Render Plane
        {
            let encoder = command_buffer
                .new_render_command_encoder(new_basic_render_pass_descriptor(render_target, None));
            encoder.set_label("Render Plane");
            encoder.set_render_pipeline_state(&self.pipeline_state);
            encode_vertex_bytes::<Space>(
                encoder,
                VertexBufferIndex::CameraSpace as _,
                &self.camera_space,
            );
            encode_fragment_bytes::<Space>(
                encoder,
                FragBufferIndex::CameraSpace as _,
                &self.camera_space,
            );
            encode_fragment_bytes::<Space>(
                encoder,
                FragBufferIndex::LightSpace as _,
                &self.light_space,
            );
            encoder.draw_primitives(MTLPrimitiveType::TriangleStrip, 0, 4);
            encoder.end_encoding();
        }
        command_buffer
    }

    #[inline]
    fn on_event(&mut self, event: UserEvent) {
        for (space, obj) in [
            (&mut self.camera_space, &mut self.camera),
            (&mut self.light_space, &mut self.light),
        ] {
            if let Some(update) = obj.on_event(event) {
                *space = Space {
                    matrix_world_to_projection: update.matrix_world_to_projection,
                    matrix_screen_to_world: update.matrix_screen_to_world,
                    position_world: update.position_world.into(),
                };
                self.needs_render = true;
            }
        }
    }

    #[inline(always)]
    fn needs_render(&self) -> bool {
        self.needs_render
    }

    fn device(&self) -> &Device {
        &self.device
    }
}

impl Delegate {}

pub fn run() {
    launch_application::<Delegate>("Project 8 - Tesselation");
}
