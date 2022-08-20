#![feature(array_zip)]
#![feature(portable_simd)]
use metal_app::metal::*;
use metal_app::*;
use std::time::Instant;

struct Delegate {
    now: Instant,
    command_queue: CommandQueue,
    device: Device,
}

impl RendererDelgate for Delegate {
    #[inline]
    fn new(device: Device) -> Self {
        Self {
            now: Instant::now(),
            command_queue: device.new_command_queue(),
            device,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        command_buffer.set_label("Renderer Command Buffer");
        let encoder = command_buffer.new_render_command_encoder({
            let rads = self.now.elapsed().as_secs_f32() * std::f32::consts::PI;
            let desc = RenderPassDescriptor::new();
            let a = desc
                .color_attachments()
                .object_at(0)
                .expect("Failed to access color attachment on render pass descriptor");
            a.set_clear_color(MTLClearColor::new(
                (rads / 2.0).cos().abs() as _,
                (rads / 3.0).cos().abs() as _,
                (rads / 4.0).cos().abs() as _,
                0.,
            ));
            a.set_load_action(MTLLoadAction::Clear);
            a.set_store_action(MTLStoreAction::Store);
            a.set_texture(Some(render_target));
            desc
        });
        encoder.end_encoding();
        command_buffer
    }

    fn device(&self) -> &Device {
        &self.device
    }
}

fn main() {
    launch_application::<Delegate>("Project 1 - Hello World");
}
