#![feature(portable_simd)]
#![feature(slice_as_chunks)]
mod shader_bindings;
use std::{f32::consts::PI, path::PathBuf, simd::Simd};

use crate::shader_bindings::{
    VertexBufferIndex_VertexBufferIndexPositions, INITIAL_CAMERA_DISTANCE,
};
use metal_app::{
    allocate_new_buffer, encode_vertex_bytes, launch_application, metal::*, unwrap_option_dcheck,
    unwrap_result_dcheck, Position, RendererDelgate, Size, Unit, UserEvent,
};
use shader_bindings::{
    packed_float4, VertexBufferIndex_VertexBufferIndexCameraDistance,
    VertexBufferIndex_VertexBufferIndexCameraRotation,
    VertexBufferIndex_VertexBufferIndexMaxPositionValue,
    VertexBufferIndex_VertexBufferIndexScreenSize,
    VertexBufferIndex_VertexBufferIndexUsePerspective,
};
use tobj::LoadOptions;

struct Delegate {
    camera_distance_offset: Unit,
    camera_distance: Unit,
    camera_rotation_offset: Simd<Unit, 2>,
    camera_rotation: Simd<Unit, 2>,
    mins_maxs: [packed_float4; 2],
    num_vertices: usize,
    render_pipeline_state: RenderPipelineState,
    use_perspective: bool,
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
        let (mut models, ..) =
            tobj::load_obj(teapot_file, &LoadOptions::default()).expect("Failed to load OBJ file");

        debug_assert_eq!(
            models.len(),
            1,
            "Model object file (`teapot.obj`) should only contain one model."
        );

        let model = models.pop().expect("Failed to parse model");
        let positions = model.mesh.positions;

        debug_assert_eq!(
            positions.len() % 3,
            0,
            r#"`mesh.positions` should contain triples (3D position)"#
        );

        let mins_maxs = {
            let (positions3, ..) = positions.as_chunks::<3>();
            let mut mins = (f32::MAX, f32::MAX, f32::MAX);
            let mut maxs = (f32::MIN, f32::MIN, f32::MIN);
            for &[x, y, z] in positions3 {
                mins = (mins.0.min(x), mins.1.min(y), mins.2.min(z));
                maxs = (maxs.0.max(x), maxs.1.max(y), maxs.2.max(z));
            }
            [
                packed_float4::new(mins.0, mins.1, mins.2, 1.0),
                packed_float4::new(maxs.0, maxs.1, maxs.2, 1.0),
            ]
        };

        let (contents, vertex_buffer_positions) = allocate_new_buffer(
            &device,
            "Vertex Buffer Positions",
            std::mem::size_of::<f32>() * positions.len(),
        );
        unsafe {
            std::ptr::copy_nonoverlapping(
                positions.as_ptr(),
                contents as *mut f32,
                positions.len(),
            );
        }
        Self {
            camera_distance_offset: 0.0,
            camera_distance: INITIAL_CAMERA_DISTANCE,
            camera_rotation_offset: Simd::splat(0.0),
            camera_rotation: Simd::from_array([-PI / 6.0, 0.0]),
            mins_maxs,
            num_vertices: positions.len() / 3,
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
                    unwrap_option_dcheck(
                        buffers.object_at(VertexBufferIndex_VertexBufferIndexPositions as u64),
                        "Failed to access vertex buffer",
                    )
                    .set_mutability(MTLMutability::Immutable);
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
            use_perspective: true,
            vertex_buffer_positions,
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
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex_VertexBufferIndexMaxPositionValue,
            &self.mins_maxs,
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
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex_VertexBufferIndexUsePerspective,
            &self.use_perspective,
        );
        encoder.set_render_pipeline_state(&self.render_pipeline_state);
        encoder.draw_primitives_instanced(MTLPrimitiveType::Point, 0, 1, self.num_vertices as _);
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
            UserEvent::KeyDown { key_code } => {
                // Space Bar Key Code
                if key_code == 49 {
                    // Toggle between orthographic and perspective
                    self.use_perspective = !self.use_perspective;
                }
            }
            _ => return,
        }
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 2 - Transformations");
}
