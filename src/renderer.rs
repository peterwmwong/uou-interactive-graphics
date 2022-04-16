use crate::{
    shader_bindings::AttachmentIndex_AttachmentIndexColor,
    unwrap_helpers::{unwrap_option_dcheck, unwrap_result_dcheck},
};
use cocoa::appkit::CGFloat;
use core_graphics::display::CGSize;
use metal::*;
use objc::{rc::autoreleasepool, runtime::YES};
use std::simd::*;

pub type Unit = f32;
pub type Position = Simd<Unit, 2>;
pub type Size = Simd<Unit, 2>;
pub type Color = Simd<f32, 4>;

pub struct MetalRenderer {
    backing_scale_factor: Unit,
    command_queue: CommandQueue,
    pub(crate) layer: MetalLayer,
    render_pipeline_state: RenderPipelineState,
    screen_size: Size,
}

impl MetalRenderer {
    pub fn new(backing_scale_factor: Unit) -> MetalRenderer {
        let device = unwrap_option_dcheck(Device::system_default(), "No device found");
        let library = unwrap_result_dcheck(
            device.new_library_with_data(include_bytes!(concat!(
                env!("OUT_DIR"),
                "/shaders.metallib"
            ))),
            "Failed to import shader metal lib.",
        );
        Self {
            backing_scale_factor,
            command_queue: device.new_command_queue(),
            layer: {
                let layer = MetalLayer::new();
                layer.set_device(&device);
                layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
                layer.set_framebuffer_only(YES);
                layer.set_presents_with_transaction(false);
                layer
            },
            render_pipeline_state: {
                let pipeline_state_desc = RenderPipelineDescriptor::new();
                pipeline_state_desc.set_label("Render Pipeline");

                // Setup Vertex Shader
                {
                    let fun = unwrap_result_dcheck(
                        library.get_function(&"main_vertex", None),
                        "Failed to access vertex shader function from metal library",
                    );
                    pipeline_state_desc.set_vertex_function(Some(&fun));
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
                        pipeline_state_desc
                            .color_attachments()
                            .object_at(AttachmentIndex_AttachmentIndexColor as u64),
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
            screen_size: Simd::splat(0.0),
        }
    }

    pub(crate) fn render(&mut self, size: Size) {
        autoreleasepool(|| {
            self.ensure_drawable_size(size);

            let command_buffer = self.command_queue.new_command_buffer();
            command_buffer.set_label("Renderer Command Buffer");
            let Some(drawable) = self.layer.next_drawable() else { return };
            let encoder = command_buffer.new_render_command_encoder({
                let clear_color: MTLClearColor = MTLClearColor::new(0.0, 0.0, 0.0, 0.0);
                let desc = RenderPassDescriptor::new();
                let texture = drawable.texture();
                let attachment = unwrap_option_dcheck(
                    desc.color_attachments().object_at(0),
                    "Failed to access color attachment on render pass descriptor",
                );
                attachment.set_texture(Some(texture));
                attachment.set_load_action(MTLLoadAction::Clear);
                attachment.set_clear_color(clear_color);
                attachment.set_store_action(MTLStoreAction::Store);
                desc
            });
            encoder.set_render_pipeline_state(&self.render_pipeline_state);
            encoder.draw_primitives_instanced(MTLPrimitiveType::TriangleStrip, 0, 3, 1);
            encoder.end_encoding();
            command_buffer.present_drawable(drawable);
            command_buffer.commit();
        });
    }

    #[inline]
    fn ensure_drawable_size(&mut self, size: Size) {
        let size = size * Simd::splat(self.backing_scale_factor);
        if self.screen_size != size {
            self.screen_size = size;
            self.layer
                .set_drawable_size(CGSize::new(size[0] as CGFloat, size[1] as CGFloat));
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(7, 7);
    }
}
