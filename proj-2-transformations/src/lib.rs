#![feature(array_zip)]
#![feature(portable_simd)]
#![feature(slice_as_chunks)]
mod shader_bindings;
use metal_app::*;
use metal_app::{metal::*, metal_types::*};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, f32x4},
};
use tobj::LoadOptions;

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// - Use Model
// - Use Camera

struct Delegate {
    camera_distance_offset: f32,
    camera_distance: f32,
    camera_rotation_offset: f32x2,
    camera_rotation: f32x2,
    device: Device,
    command_queue: CommandQueue,
    mins_maxs: [packed_float4; 2],
    num_vertices: usize,
    render_pipeline: RenderPipelineState,
    screen_size: f32x2,
    use_perspective: bool,
    vertex_buffer_positions: Buffer,
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let teapot_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("common-assets")
            .join("teapot")
            .join("teapot.obj");
        let (mut models, ..) = tobj::load_obj(
            teapot_file,
            &LoadOptions {
                single_index: false,
                triangulate: true,
                ignore_points: true,
                ignore_lines: true,
            },
        )
        .expect("Failed to load OBJ file");

        let model = models
            .pop()
            .expect("Failed to parse model, expecting atleast one model (teapot)");
        let positions = model.mesh.positions;

        debug_assert_eq!(
            positions.len() % 3,
            0,
            r#"`mesh.positions` should contain triples (3D position)"#
        );

        let mins_maxs = {
            let (positions3, ..) = positions.as_chunks::<3>();
            let mut mins = f32x4::splat(f32::MAX);
            let mut maxs = f32x4::splat(f32::MIN);
            for &[x, y, z] in positions3 {
                let input = f32x4::from_array([x, y, z, 0.0]);
                mins = mins.min(input);
                maxs = maxs.max(input);
            }
            [mins.into(), maxs.into()]
        };

        Self {
            camera_distance_offset: 0.0,
            camera_distance: INITIAL_CAMERA_DISTANCE,
            camera_rotation_offset: f32x2::splat(0.0),
            camera_rotation: f32x2::from_array([-PI / 6.0, 0.0]),
            command_queue: device.new_command_queue(),
            mins_maxs,
            num_vertices: positions.len() / 3,
            render_pipeline: {
                let library = device
                    .new_library_with_data(include_bytes!(concat!(
                        env!("OUT_DIR"),
                        "/shaders.metallib"
                    )))
                    .expect("Failed to import shader metal lib.");
                create_render_pipeline(
                    &device,
                    &new_render_pipeline_descriptor(
                        "Render Pipeline",
                        &library,
                        Some((DEFAULT_PIXEL_FORMAT, false)),
                        None,
                        None,
                        Some((&"main_vertex", VertexBufferIndex::LENGTH as _)),
                        Some((&"main_fragment", 0)),
                    ),
                )
                .pipeline_state
            },
            use_perspective: true,
            vertex_buffer_positions: allocate_new_buffer_with_data(
                &device,
                "Vertex Buffer Positions",
                &positions,
            ),
            screen_size: f32x2::splat(0.),
            device,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        command_buffer.set_label("Renderer Command Buffer");
        let encoder = command_buffer
            .new_render_command_encoder(new_render_pass_descriptor(Some(render_target), None));
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex::MaxPositionValue as _,
            &self.mins_maxs,
        );
        encoder.set_vertex_buffer(
            VertexBufferIndex::Positions as _,
            Some(&self.vertex_buffer_positions),
            0,
        );
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex::CameraRotation as _,
            &(self.camera_rotation + self.camera_rotation_offset),
        );
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex::CameraDistance as _,
            &(self.camera_distance + self.camera_distance_offset),
        );
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex::ScreenSize as _,
            &self.screen_size,
        );
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex::UsePerspective as _,
            &self.use_perspective,
        );
        encoder.set_render_pipeline_state(&self.render_pipeline);
        encoder.draw_primitives(MTLPrimitiveType::Point, 0, self.num_vertices as _);
        encoder.end_encoding();
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        use MouseButton::*;
        use UserEvent::*;
        match event {
            MouseDrag {
                button,
                drag_amount,
                ..
            } => {
                match button {
                    Left => {
                        self.camera_rotation += {
                            let adjacent = f32x2::splat(self.camera_distance);
                            let offsets = drag_amount / f32x2::splat(4.);
                            let ratio = offsets / adjacent;
                            f32x2::from_array([
                                ratio[1].atan(), // Rotation on x-axis
                                ratio[0].atan(), // Rotation on y-axis
                            ])
                        }
                    }
                    Right => self.camera_distance += -drag_amount[1] / 8.0,
                }
            }
            KeyDown { key_code, .. } => {
                // "P" Key Code
                if key_code == 35 {
                    // Toggle between orthographic and perspective
                    self.use_perspective = !self.use_perspective;
                }
            }
            WindowFocusedOrResized { size, .. } => {
                self.screen_size = size;
            }
            _ => {}
        }
    }

    fn device(&self) -> &Device {
        &self.device
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 2 - Transformations");
}
