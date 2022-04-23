#![feature(portable_simd)]
#![feature(slice_as_chunks)]
mod shader_bindings;

use metal_app::{
    allocate_new_buffer_with_data, encode_vertex_bytes, f32x4x4, launch_application, metal::*,
    unwrap_option_dcheck, unwrap_result_dcheck, Position, RendererDelgate, Size, Unit, UserEvent,
};
use shader_bindings::{
    VertexBufferIndex_VertexBufferIndexCameraDistance,
    VertexBufferIndex_VertexBufferIndexCameraRotation, VertexBufferIndex_VertexBufferIndexIndices,
    VertexBufferIndex_VertexBufferIndexModelViewProjection,
    VertexBufferIndex_VertexBufferIndexPositions, VertexBufferIndex_VertexBufferIndexScreenSize,
    INITIAL_CAMERA_DISTANCE,
};
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, Simd},
};
use tobj::{LoadOptions, Mesh};

struct Delegate {
    camera_distance_offset: Unit,
    camera_distance: Unit,
    camera_rotation_offset: f32x2,
    camera_rotation: f32x2,
    model_matrix: f32x4x4,
    num_triangles: usize,
    projection_matrix: f32x4x4,
    render_pipeline_state: RenderPipelineState,
    vertex_buffer_indices: Buffer,
    vertex_buffer_positions: Buffer,
}

impl Delegate {
    // TODO: This doesn't allow for a full 360 degree rotation in one drag (atan is [-90, 90]).
    fn calc_rotation_offset(&self, down_position: Position, position: Position) -> Position {
        let adjacent = Simd::splat(self.camera_distance);
        let offsets = down_position - position;
        let ratio = offsets / adjacent;
        Simd::from_array([
            ratio[1].atan(), // Rotation on x-axis
            ratio[0].atan(), // Rotation on y-axis
        ])
    }
}

impl RendererDelgate for Delegate {
    fn new(device: metal_app::metal::Device) -> Self {
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
        let Mesh {
            positions, indices, ..
        } = model.mesh;

        debug_assert_eq!(
            indices.len() % 3,
            0,
            r#"`mesh.indices` should contain triples (triangle vertices). Model should have been loaded with `triangulate`, guaranteeing all faces have 3 vertices."#
        );
        debug_assert_eq!(
            positions.len() % 3,
            0,
            r#"`mesh.positions` should contain triples (3D position)"#
        );

        let (positions3, ..) = positions.as_chunks::<3>();
        let mut mins = Simd::splat(f32::MAX);
        let mut maxs = Simd::splat(f32::MIN);
        for &[x, y, z] in positions3 {
            let input = Simd::from_array([x, y, z, 0.0]);
            mins = mins.min(input);
            maxs = maxs.max(input);
        }
        let max_bound = mins.reduce_min().abs().max(maxs.reduce_max());

        let height_of_teapot = maxs[2] - mins[2];
        let model_matrix = {
            let r = f32x4x4::x_rotate(std::f32::consts::PI / 2.);
            let t = f32x4x4::translate(0., 0., -height_of_teapot / 2.0);
            r * t
        };

        let camera_distance = INITIAL_CAMERA_DISTANCE;
        // TODO: move this into the draw or in on_event
        // let view_matrix = f32x4x4::translate(0., 0., camera_distance, 0.)
        //     * f32x4x4::rotate(camera_rotation[0], camera_rotation[1], 0.);

        let n = 0.1;
        let f = 1000.0;
        let initial_screen_ratio = 1.0;
        let b = n * -max_bound / camera_distance;
        let t = n * max_bound / camera_distance;
        let l = initial_screen_ratio * b;
        let r = initial_screen_ratio * t;

        let projection_matrix = {
            let perspective_matrix = f32x4x4::new(
                [n, 0., 0., 0.],
                [0., n, 0., 0.],
                [0., 0., n + f, -n * f],
                [0., 0., 1., 0.],
            );
            let orthographic_matrix = {
                f32x4x4::new(
                    [2. / (r - l), 0., 0., -(r + l) / (r - l)],
                    [0., 2. / (t - b), 0., -(t + b) / (t - b)],
                    // IMPORTANT: Metal's NDC coordinate space has a z range of [0.,1], **NOT [-1,1]** (OpenGL).
                    [0., 0., 1. / (f - n), -n / (f - n)],
                    [0., 0., 0., 1.],
                )
            };
            orthographic_matrix * perspective_matrix
        };

        Self {
            camera_distance_offset: 0.0,
            camera_distance: INITIAL_CAMERA_DISTANCE,
            camera_rotation_offset: Simd::splat(0.0),
            camera_rotation: Simd::from_array([-PI / 6.0, 0.0]),
            model_matrix,
            num_triangles: indices.len() / 3,
            projection_matrix,
            render_pipeline_state: {
                let library = device
                    .new_library_with_data(include_bytes!(concat!(
                        env!("OUT_DIR"),
                        "/shaders.metallib"
                    )))
                    .expect("Failed to import shader metal lib.");

                let pipeline_state_desc = RenderPipelineDescriptor::new();
                pipeline_state_desc.set_label("Render Pipeline");

                // Setup Vertex Shader
                {
                    let fun = library
                        .get_function(&"main_vertex", None)
                        .expect("Failed to access vertex shader function from metal library");
                    pipeline_state_desc.set_vertex_function(Some(&fun));

                    let buffers = pipeline_state_desc
                        .vertex_buffers()
                        .expect("Failed to access vertex buffers");
                    for &buffer_index in &[
                        VertexBufferIndex_VertexBufferIndexIndices,
                        VertexBufferIndex_VertexBufferIndexPositions,
                        VertexBufferIndex_VertexBufferIndexModelViewProjection,
                        VertexBufferIndex_VertexBufferIndexScreenSize,
                        VertexBufferIndex_VertexBufferIndexCameraRotation,
                        VertexBufferIndex_VertexBufferIndexCameraDistance,
                    ] {
                        unwrap_option_dcheck(
                            buffers.object_at(buffer_index as _),
                            "Failed to access vertex buffer",
                        )
                        .set_mutability(MTLMutability::Immutable);
                    }
                }

                // Setup Fragment Shader
                {
                    let fun = unwrap_result_dcheck(
                        library.get_function(&"main_fragment", None),
                        "Failed to access fragment shader function from metal library",
                    );
                    pipeline_state_desc.set_fragment_function(Some(&fun));
                }

                // Setup Target Color Attachment
                {
                    let desc = &unwrap_option_dcheck(
                        pipeline_state_desc.color_attachments().object_at(0 as u64),
                        "Failed to access color attachment on pipeline descriptor",
                    );
                    desc.set_blending_enabled(true);

                    desc.set_rgb_blend_operation(MTLBlendOperation::Add);
                    desc.set_source_rgb_blend_factor(MTLBlendFactor::SourceAlpha);
                    desc.set_destination_rgb_blend_factor(MTLBlendFactor::OneMinusSourceAlpha);

                    desc.set_alpha_blend_operation(MTLBlendOperation::Add);
                    desc.set_source_alpha_blend_factor(MTLBlendFactor::SourceAlpha);
                    desc.set_destination_alpha_blend_factor(MTLBlendFactor::OneMinusSourceAlpha);

                    desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
                }

                unwrap_result_dcheck(
                    device.new_render_pipeline_state(&pipeline_state_desc),
                    "Failed to create render pipeline",
                )
            },
            vertex_buffer_indices: allocate_new_buffer_with_data(
                &device,
                "Vertex Buffer Indices",
                &indices,
            ),
            vertex_buffer_positions: allocate_new_buffer_with_data(
                &device,
                "Vertex Buffer Positions",
                &positions,
            ),
        }
    }

    fn draw(
        &mut self,
        command_queue: &CommandQueue,
        drawable: &MetalDrawableRef,
        screen_size: Size,
    ) {
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
        encoder.set_render_pipeline_state(&self.render_pipeline_state);
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex_VertexBufferIndexModelViewProjection,
            // TODO: START HERE
            // TODO: START HERE
            // TODO: START HERE
            // 1. The shader is receiving this matrix as columns instead of rows!!!
            // 2. This should be: projection_matrix * view_matrix * model_matrix
            &self.model_matrix,
        );
        encoder.set_vertex_buffer(
            VertexBufferIndex_VertexBufferIndexIndices as _,
            Some(&self.vertex_buffer_indices),
            0,
        );
        encoder.set_vertex_buffer(
            VertexBufferIndex_VertexBufferIndexPositions as _,
            Some(&self.vertex_buffer_positions),
            0,
        );
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex_VertexBufferIndexCameraRotation,
            &(self.camera_rotation + self.camera_rotation_offset),
        );
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex_VertexBufferIndexCameraDistance,
            &(self.camera_distance + self.camera_distance_offset),
        );
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex_VertexBufferIndexScreenSize,
            &screen_size,
        );
        encoder.draw_primitives_instanced(
            MTLPrimitiveType::TriangleStrip,
            0,
            3,
            self.num_triangles as _,
        );
        encoder.end_encoding();
        command_buffer.present_drawable(drawable);
        command_buffer.commit();
    }

    fn on_event(&mut self, event: UserEvent) {
        use metal_app::MouseButton::*;
        use UserEvent::*;
        fn calc_distance_offset(down_position: Position, position: Position) -> Unit {
            // Dragging up   => zooms in  (-offset)
            // Dragging down => zooms out (+offset)
            let screen_offset = position[1] - down_position[1];
            screen_offset / 8.0
        }
        match event {
            MouseDrag {
                button,
                position,
                down_position,
            } => match button {
                Left => {
                    self.camera_rotation_offset =
                        self.calc_rotation_offset(down_position, position);
                }
                Right => {
                    self.camera_distance_offset = calc_distance_offset(down_position, position);
                }
            },
            MouseUp {
                button,
                position,
                down_position,
            } => match button {
                Left => {
                    self.camera_rotation_offset = Simd::default();
                    self.camera_rotation += self.calc_rotation_offset(down_position, position);
                }
                Right => {
                    self.camera_distance_offset = 0.0;
                    self.camera_distance += calc_distance_offset(down_position, position);
                }
            },
            _ => return,
        }
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 3 - Shading");
}
