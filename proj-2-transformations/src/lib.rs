#![feature(portable_simd)]
mod shader_bindings;
use std::{os::raw::c_void, path::PathBuf};

use crate::shader_bindings::VertexBufferIndex_VertexBufferIndexPositions;
use metal_app::{
    allocate_new_buffer, launch_application, metal::*, unwrap_option_dcheck, unwrap_result_dcheck,
    RendererDelgate, Size,
};
use shader_bindings::VertexBufferIndex_VertexBufferIndexMaxPositionValue;
use tobj::LoadOptions;

struct Delegate {
    num_vertices: usize,
    // TODO: START HERE
    // TODO: START HERE
    // TODO: START HERE
    // - Instead of doing a basic normalization (divide by max), this should be Projection View Matrix (to Canonicalized View)
    max_position_value: f32,
    vertex_buffer_positions: Buffer,
    render_pipeline_state: RenderPipelineState,
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

        let &max_position_value = positions
            .iter()
            .reduce(|a, b| if a > b { a } else { b })
            .unwrap();

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
            max_position_value,
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
        }
    }

    fn draw(
        &mut self,
        command_queue: &CommandQueue,
        drawable: &MetalDrawableRef,
        _screen_size: Size,
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
            let max_value_ptr: *const f32 = &self.max_position_value;
            encoder.set_vertex_bytes(
                VertexBufferIndex_VertexBufferIndexMaxPositionValue as _,
                std::mem::size_of::<f32>() as _,
                max_value_ptr as *const c_void,
            );
        }
        encoder.set_vertex_buffer(
            VertexBufferIndex_VertexBufferIndexPositions as _,
            Some(&self.vertex_buffer_positions),
            0,
        );
        encoder.set_render_pipeline_state(&self.render_pipeline_state);
        encoder.draw_primitives_instanced(MTLPrimitiveType::Point, 0, 1, self.num_vertices as _);
        encoder.end_encoding();
        command_buffer.present_drawable(drawable);
        command_buffer.commit();
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 2 - Transformations");
}
