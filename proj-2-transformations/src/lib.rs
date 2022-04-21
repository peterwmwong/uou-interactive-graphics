#![feature(portable_simd)]
#![feature(slice_as_chunks)]
mod shader_bindings;
use std::{f32::consts::PI, os::raw::c_void, path::PathBuf, simd::Simd, time::Instant};

use crate::shader_bindings::VertexBufferIndex_VertexBufferIndexPositions;
use metal_app::{
    allocate_new_buffer, launch_application, metal::*, unwrap_option_dcheck, unwrap_result_dcheck,
    Position, RendererDelgate, Size, Unit, UserEvent,
};
use shader_bindings::{
    packed_float4, VertexBufferIndex_VertexBufferIndexAspectRatio,
    VertexBufferIndex_VertexBufferIndexCameraRotationDistance,
    VertexBufferIndex_VertexBufferIndexMaxPositionValue, VertexBufferIndex_VertexBufferIndexTime,
};
use tobj::LoadOptions;

struct Delegate {
    num_vertices: usize,
    camera_distance_offset: Unit,
    camera_rotation_offset: Simd<Unit, 2>,
    camera_rotation_distance: packed_float4,
    mins_maxs: [packed_float4; 2],
    vertex_buffer_positions: Buffer,
    render_pipeline_state: RenderPipelineState,
    now: Instant,
}

impl Delegate {
    fn calc_rotation_offset(&self, down_position: Position, position: Position) -> Position {
        let adjacent = Simd::splat(self.camera_rotation_distance.z);
        let offsets = position - down_position;
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
            camera_rotation_offset: Simd::splat(0.0),
            camera_rotation_distance: packed_float4 {
                x: -PI / 4.0,
                y: 0.0,
                z: -50.0,
                w: 1.0,
            },
            mins_maxs,
            num_vertices: positions.len() / 3,
            vertex_buffer_positions,
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
            now: Instant::now(),
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
        {
            let max_value_ptr: *const [packed_float4; 2] = &self.mins_maxs;
            encoder.set_vertex_bytes(
                VertexBufferIndex_VertexBufferIndexMaxPositionValue as _,
                std::mem::size_of::<[packed_float4; 2]>() as _,
                max_value_ptr as *const c_void,
            );
        }
        encoder.set_vertex_buffer(
            VertexBufferIndex_VertexBufferIndexPositions as _,
            Some(&self.vertex_buffer_positions),
            0,
        );
        {
            let mut cam_rot_pos = self.camera_rotation_distance;
            cam_rot_pos.x += self.camera_rotation_offset[0];
            cam_rot_pos.y += self.camera_rotation_offset[1];
            cam_rot_pos.z += self.camera_distance_offset;
            let camera_rotation_distance: *const packed_float4 = &cam_rot_pos;
            encoder.set_vertex_bytes(
                VertexBufferIndex_VertexBufferIndexCameraRotationDistance as _,
                std::mem::size_of::<packed_float4>() as _,
                camera_rotation_distance as *const c_void,
            );
        }
        {
            let aspect_ratio: *const f32 = &(screen_size[0] / screen_size[1]);
            encoder.set_vertex_bytes(
                VertexBufferIndex_VertexBufferIndexAspectRatio as _,
                std::mem::size_of::<f32>() as _,
                aspect_ratio as *const c_void,
            );
        }
        {
            let time: *const f32 = &self.now.elapsed().as_secs_f32();
            encoder.set_vertex_bytes(
                VertexBufferIndex_VertexBufferIndexTime as _,
                std::mem::size_of::<f32>() as _,
                time as *const c_void,
            );
        }
        encoder.set_render_pipeline_state(&self.render_pipeline_state);
        encoder.draw_primitives_instanced(MTLPrimitiveType::Point, 0, 1, self.num_vertices as _);
        encoder.end_encoding();
        command_buffer.present_drawable(drawable);
        command_buffer.commit();
    }

    fn on_event(&mut self, event: UserEvent) {
        fn calc_distance_offset(down_position: Position, position: Position) -> Unit {
            // Dragging up   => zooms in  (-offset)
            // Dragging down => zooms out (+offset)
            let screen_offset = down_position[1] - position[1];

            // TODO: Probably need some type of scaling, it zooms to fast... dragging up one pixel zooms to much.
            // - Simple ratio?
            // - Ratio based world space camera distance?
            screen_offset / 8.0
        }
        match event {
            UserEvent::MouseDrag {
                button,
                position,
                down_position,
            } => match button {
                metal_app::MouseButton::Left => {
                    self.camera_rotation_offset =
                        self.calc_rotation_offset(down_position, position);
                }
                metal_app::MouseButton::Right => {
                    self.camera_distance_offset = calc_distance_offset(down_position, position);
                }
            },
            UserEvent::MouseUp {
                button,
                position,
                down_position,
            } => match button {
                metal_app::MouseButton::Left => {
                    self.camera_rotation_offset = Simd::default();
                    let rot = self.calc_rotation_offset(down_position, position);
                    self.camera_rotation_distance.x += rot[0];
                    self.camera_rotation_distance.y += rot[1];
                }
                metal_app::MouseButton::Right => {
                    self.camera_distance_offset = 0.0;
                    self.camera_rotation_distance.z +=
                        calc_distance_offset(down_position, position);
                }
            },
            _ => return,
        }
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 2 - Transformations");
}
