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
            new_render_pass_descriptor(
                Some((
                    render_target,
                    (
                        (rads / 2.0).cos().abs(),
                        (rads / 3.0).cos().abs(),
                        (rads / 4.0).cos().abs(),
                        0.,
                    ),
                    MTLLoadAction::Clear,
                    MTLStoreAction::Store,
                )),
                None,
                None,
            )
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
