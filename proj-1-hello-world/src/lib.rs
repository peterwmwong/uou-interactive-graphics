#![feature(portable_simd)]
use metal_app::metal::*;
use metal_app::*;
use std::time::Instant;

struct Delegate {
    now: Instant,
}

impl RendererDelgate for Delegate {
    #[inline]
    fn new(_device: Device, _command_queue: &CommandQueue) -> Self {
        Self {
            now: Instant::now(),
        }
    }

    #[inline]
    fn render<'a>(
        &mut self,
        command_queue: &'a CommandQueue,
        render_target: &TextureRef,
    ) -> &'a CommandBufferRef {
        let command_buffer = command_queue.new_command_buffer();
        command_buffer.set_label("Renderer Command Buffer");
        let encoder = command_buffer.new_render_command_encoder({
            let desc = RenderPassDescriptor::new();
            let attachment = unwrap_option_dcheck(
                desc.color_attachments().object_at(0),
                "Failed to access color attachment on render pass descriptor",
            );
            attachment.set_texture(Some(render_target));
            attachment.set_load_action(MTLLoadAction::Clear);
            let rads = self.now.elapsed().as_secs_f32() * std::f32::consts::PI;
            attachment.set_clear_color(MTLClearColor::new(
                (rads / 2.0).cos().abs() as f64,
                (rads / 3.0).cos().abs() as f64,
                (rads / 4.0).cos().abs() as f64,
                0.0,
            ));
            attachment.set_store_action(MTLStoreAction::Store);
            desc
        });
        encoder.end_encoding();
        command_buffer
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 1 - Hello World");
}
