use crate::unwrap_helpers::unwrap_option_dcheck;
use cocoa::appkit::CGFloat;
use core_graphics::display::CGSize;
use metal::*;
use objc::{rc::autoreleasepool, runtime::YES};
use std::simd::*;

pub type Unit = f32;
pub type Size = Simd<Unit, 2>;

pub trait RendererDelgate {
    fn new(device: Device) -> Self;
    fn draw(
        &mut self,
        command_queue: &CommandQueue,
        drawable: &MetalDrawableRef,
        screen_size: Size,
    );
}

pub(crate) struct MetalRenderer<R: RendererDelgate> {
    backing_scale_factor: Unit,
    command_queue: CommandQueue,
    pub(crate) layer: MetalLayer,
    screen_size: Size,
    delegate: R,
}

impl<R: RendererDelgate> MetalRenderer<R> {
    pub fn new(backing_scale_factor: Unit) -> MetalRenderer<R> {
        let device = unwrap_option_dcheck(Device::system_default(), "No device found");
        let command_queue = device.new_command_queue();
        let layer = MetalLayer::new();
        layer.set_device(&device);
        layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        layer.set_framebuffer_only(YES);
        layer.set_presents_with_transaction(false);
        let delegate = R::new(device);
        Self {
            backing_scale_factor,
            command_queue,
            layer,
            delegate,
            screen_size: Simd::splat(0.0),
        }
    }

    pub(crate) fn render(&mut self, size: Size) {
        autoreleasepool(|| {
            let size = size * Simd::splat(self.backing_scale_factor);
            if self.screen_size != size {
                self.screen_size = size;
                self.layer
                    .set_drawable_size(CGSize::new(size[0] as CGFloat, size[1] as CGFloat));
            }
            if let Some(drawable) = self.layer.next_drawable() {
                self.delegate.draw(&self.command_queue, drawable, size);
            };
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(7, 7);
    }
}
