#![feature(portable_simd)]
#![feature(slice_as_chunks)]
mod shader_bindings;
use metal_app::metal::*;
use metal_app::*;
use shader_bindings::*;
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, f32x4},
};
use tobj::LoadOptions;

struct Delegate {
    camera_distance_offset: f32,
    camera_distance: f32,
    camera_rotation_offset: f32x2,
    camera_rotation: f32x2,
    mins_maxs: [packed_float4; 2],
    num_vertices: usize,
    render_pipeline_state: RenderPipelineState,
    screen_size: f32x2,
    use_perspective: bool,
    vertex_buffer_positions: Buffer,
}

impl RendererDelgate for Delegate {
    fn new(device: Device, _command_queue: &CommandQueue) -> Self {
        let teapot_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
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
            mins_maxs,
            num_vertices: positions.len() / 3,
            render_pipeline_state: {
                let library = device
                    .new_library_with_data(include_bytes!(concat!(
                        env!("OUT_DIR"),
                        "/shaders.metallib"
                    )))
                    .expect("Failed to import shader metal lib.");
                let base_pipeline_desc = RenderPipelineDescriptor::new();

                // Setup Target Color Attachment
                {
                    let desc = unwrap_option_dcheck(
                        base_pipeline_desc.color_attachments().object_at(0 as u64),
                        "Failed to access color attachment on pipeline descriptor",
                    );
                    desc.set_blending_enabled(false);
                    desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
                }

                create_pipeline(
                    &device,
                    &library,
                    &base_pipeline_desc,
                    "Render Pipeline",
                    &"main_vertex",
                    VertexBufferIndex_VertexBufferIndex_LENGTH,
                    "main_fragment",
                    0,
                )
            },
            use_perspective: true,
            vertex_buffer_positions: allocate_new_buffer_with_data(
                &device,
                "Vertex Buffer Positions",
                &positions,
            ),
            screen_size: f32x2::splat(0.),
        }
    }

    #[inline]
    fn draw(&mut self, command_queue: &CommandQueue, drawable: &MetalDrawableRef) {
        let command_buffer = command_queue.new_command_buffer();
        command_buffer.set_label("Renderer Command Buffer");
        let encoder = command_buffer.new_render_command_encoder({
            let clear_color: MTLClearColor = MTLClearColor::new(0.0, 0.0, 0.0, 0.0);
            let desc = RenderPassDescriptor::new();
            let attachment = unwrap_option_dcheck(
                desc.color_attachments().object_at(0),
                "Failed to access color attachment on render pass descriptor",
            );
            attachment.set_texture(Some(drawable.texture()));
            attachment.set_load_action(MTLLoadAction::Clear);
            attachment.set_clear_color(clear_color);
            attachment.set_store_action(MTLStoreAction::Store);
            desc
        });
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex_VertexBufferIndex_MaxPositionValue,
            &self.mins_maxs,
        );
        encoder.set_vertex_buffer(
            VertexBufferIndex_VertexBufferIndex_Positions as _,
            Some(&self.vertex_buffer_positions),
            0,
        );
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex_VertexBufferIndex_CameraRotation,
            &(self.camera_rotation + self.camera_rotation_offset),
        );
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex_VertexBufferIndex_CameraDistance,
            &(self.camera_distance + self.camera_distance_offset),
        );
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex_VertexBufferIndex_ScreenSize,
            &self.screen_size,
        );
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex_VertexBufferIndex_UsePerspective,
            &self.use_perspective,
        );
        encoder.set_render_pipeline_state(&self.render_pipeline_state);
        encoder.draw_primitives_instanced(MTLPrimitiveType::Point, 0, 1, self.num_vertices as _);
        encoder.end_encoding();
        command_buffer.present_drawable(drawable);
        command_buffer.commit();
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
            WindowResize { size, .. } => {
                self.screen_size = size;
            }
            _ => {}
        }
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 2 - Transformations");
}
