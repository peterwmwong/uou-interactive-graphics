use crate::unwrap_helpers::unwrap_option_dcheck;
use cocoa::appkit::CGFloat;
use core_graphics::display::CGSize;
use metal::*;
use objc::{rc::autoreleasepool, runtime::YES};
use std::{os::raw::c_ushort, simd::*};

pub type Unit = f32;
// TODO: Rename to indicate 2D-ness
pub type Size = Simd<Unit, 2>;
pub type Position = Simd<Unit, 2>;

#[derive(Copy, Clone, PartialEq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum MouseButton {
    Left,
    Right,
}

#[derive(Copy, Clone, PartialEq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum UserEvent {
    MouseDown {
        button: MouseButton,
        position: Position,
    },
    MouseUp {
        button: MouseButton,
        position: Position,
        down_position: Position,
    },
    MouseDrag {
        button: MouseButton,
        position: Position,
        down_position: Position,
    },
    KeyDown {
        key_code: c_ushort,
    },
}

pub trait RendererDelgate {
    fn new(device: Device) -> Self;
    fn draw(
        &mut self,
        command_queue: &CommandQueue,
        drawable: &MetalDrawableRef,
        screen_size: Size,
    );
    fn on_event(&mut self, _event: UserEvent) {}
}

pub(crate) struct MetalRenderer<R: RendererDelgate> {
    backing_scale_factor: Unit,
    command_queue: CommandQueue,
    pub(crate) layer: MetalLayer,
    screen_size: Size,
    delegate: R,
}

impl<R: RendererDelgate> MetalRenderer<R> {
    #[inline]
    pub(crate) fn new(backing_scale_factor: Unit) -> MetalRenderer<R> {
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

    #[inline]
    pub(crate) fn update_size(&mut self, size: Size) {
        let size = size * Simd::splat(self.backing_scale_factor);
        if self.screen_size != size {
            self.screen_size = size;
            self.layer
                .set_drawable_size(CGSize::new(size[0] as CGFloat, size[1] as CGFloat));
        }
    }

    #[inline]
    pub(crate) fn render(&mut self) {
        autoreleasepool(|| {
            if let Some(drawable) = self.layer.next_drawable() {
                self.delegate
                    .draw(&self.command_queue, drawable, self.screen_size);
            };
        });
    }

    #[inline]
    pub(crate) fn on_event(&mut self, event: UserEvent) {
        self.delegate.on_event(event);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(7, 7);
    }
}
