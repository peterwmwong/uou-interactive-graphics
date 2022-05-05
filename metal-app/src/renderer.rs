use crate::unwrap_helpers::unwrap_option_dcheck;
use cocoa::appkit::CGFloat;
use core_graphics::display::CGSize;
use metal::*;
use objc::{rc::autoreleasepool, runtime::YES};
use std::{os::raw::c_ushort, simd::*};

#[derive(Copy, Clone, PartialEq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum MouseButton {
    Left,
    Right,
}

bitflags::bitflags! {
    pub struct ModifierKeys: u32 {
        const SHIFT    = 1 << 0;
        const CONTROL  = 1 << 1;
        const COMMAND  = 1 << 2;
        const FUNCTION = 1 << 3;
    }
}

#[derive(Copy, Clone, PartialEq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum UserEvent {
    #[non_exhaustive]
    MouseDown {
        button: MouseButton,
        modifier_keys: ModifierKeys,
        position: f32x2,
    },
    #[non_exhaustive]
    MouseUp {
        button: MouseButton,
        modifier_keys: ModifierKeys,
        position: f32x2,
    },
    #[non_exhaustive]
    MouseDrag {
        button: MouseButton,
        modifier_keys: ModifierKeys,
        position: f32x2,
        drag_amount: f32x2,
    },
    #[non_exhaustive]
    KeyDown { key_code: c_ushort },
    #[non_exhaustive]
    WindowResize { size: f32x2 },
}

pub trait RendererDelgate {
    fn new(device: Device) -> Self;

    fn draw(&mut self, command_queue: &CommandQueue, drawable: &MetalDrawableRef);

    #[inline]
    fn on_event(&mut self, _event: UserEvent) {}
}

pub(crate) struct MetalRenderer<R: RendererDelgate> {
    backing_scale_factor: f32,
    command_queue: CommandQueue,
    pub(crate) layer: MetalLayer,
    screen_size: f32x2,
    delegate: R,
}

impl<R: RendererDelgate> MetalRenderer<R> {
    #[inline]
    pub(crate) fn new(backing_scale_factor: f32) -> MetalRenderer<R> {
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
    pub(crate) fn update_size(&mut self, size: f32x2) {
        let size = size * Simd::splat(self.backing_scale_factor);
        if self.screen_size != size {
            self.layer
                .set_drawable_size(CGSize::new(size[0] as CGFloat, size[1] as CGFloat));
            self.screen_size = size;
            self.delegate.on_event(UserEvent::WindowResize { size });
        }
    }

    #[inline]
    pub(crate) fn render(&mut self) {
        autoreleasepool(|| {
            if let Some(drawable) = self.layer.next_drawable() {
                self.delegate.draw(&self.command_queue, drawable);
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
