#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::Camera,
    launch_application,
    metal::{MTLLoadAction::*, MTLStoreAction::*, *},
    metal_types::*,
    model_acceleration_structure::ModelAccelerationStructure,
    pipeline::*,
    MaxBounds, ModifierKeys, RendererDelgate, UserEvent, DEFAULT_COLOR_FORMAT,
};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    ops::Neg,
    path::PathBuf,
    simd::{f32x2, SimdFloat},
};

const INITIAL_CAMERA_DISTANCE: f32 = 1.;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([0., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

struct Delegate {
    camera: Camera,
    command_queue: CommandQueue,
    device: Device,
    model_accel_struct: ModelAccelerationStructure,
    needs_render: bool,
    pipeline: RenderPipeline<1, main_vertex, main_fragment, (NoDepth, NoStencil)>,
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let executable_name = std::env::args()
            .nth(0)
            .expect("Failed to access command line executable name");
        let model_file_path = std::env::args().nth(1).expect(&format!(
            "Usage: {executable_name} [Path to Wavefront OBJ file]"
        ));
        let model_file = PathBuf::from(model_file_path);
        let command_queue = device.new_command_queue();
        Self {
            camera: Camera::new(
                INITIAL_CAMERA_DISTANCE,
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            model_accel_struct: ModelAccelerationStructure::from_file(
                model_file,
                &device,
                &command_queue,
                |MaxBounds { center, size }, _| {
                    let [cx, cy, cz, _] = center.neg().to_array();
                    let scale = 1. / size.reduce_max();
                    f32x4x4::scale(scale, scale, scale, 1.)
                        * f32x4x4::y_rotate(PI)
                        * f32x4x4::x_rotate(PI / 2.)
                        * f32x4x4::translate(cx, cy, cz)
                },
            ),
            command_queue,
            needs_render: false,
            pipeline: RenderPipeline::new(
                "Pipeline",
                &device,
                &device.new_library_with_data(LIBRARY_BYTES).unwrap(),
                [(DEFAULT_COLOR_FORMAT, BlendMode::NoBlend)],
                main_vertex,
                main_fragment,
                (NoDepth, NoStencil),
            ),
            device,
        }
    }

    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.needs_render = false;
        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        self.pipeline.new_pass(
            "Render",
            command_buffer,
            [(render_target, (0., 0., 0., 1.), Clear, Store)],
            NoDepth,
            NoStencil,
            NoDepthState,
            &[&self.model_accel_struct.resource()],
            |p| {
                p.draw_primitives_with_binds(
                    NoBinds,
                    main_fragment_binds {
                        accelerationStructure: self.model_accel_struct.bind(),
                        camera: Bind::Value(&self.camera.projected_space),
                        m_model_to_worlds: BindMany::buffer(
                            &self.model_accel_struct.m_model_to_worlds_buffer,
                        ),
                    },
                    MTLPrimitiveType::Triangle,
                    0,
                    3,
                )
            },
        );
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        if self.camera.on_event(event) {
            self.needs_render = true;
        }

        use UserEvent::*;
        match event {
            KeyDown { key_code, .. } => {
                let translate_x = if key_code == UserEvent::KEY_CODE_RIGHT {
                    0.1
                } else if key_code == UserEvent::KEY_CODE_LEFT {
                    -0.1
                } else {
                    return;
                };
                self.model_accel_struct.update_model_to_world_matrix(
                    0,
                    f32x4x4::translate(translate_x, 0., 0.),
                    &self.command_queue,
                );
                self.needs_render = true;
            }
            _ => {}
        }
    }

    #[inline]
    fn needs_render(&self) -> bool {
        self.needs_render
    }

    #[inline]
    fn device(&self) -> &Device {
        &self.device
    }
}

fn main() {
    launch_application::<Delegate>("x-rt");
}
