use cocoa::appkit::CGFloat;
use core_graphics::display::CGSize;
use metal::*;
use objc::{rc::autoreleasepool, runtime::YES};
use std::{os::raw::c_ushort, simd::f32x2};

#[derive(Copy, Clone, PartialEq, Eq)]
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
        const ALT_OPTION = 1 << 4;
    }
}

#[derive(Copy, Clone, PartialEq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum UserEvent {
    MouseMoved {
        position: f32x2,
    },
    MouseDown {
        button: MouseButton,
        modifier_keys: ModifierKeys,
        position: f32x2,
    },
    MouseUp {
        button: MouseButton,
        modifier_keys: ModifierKeys,
        position: f32x2,
    },
    MouseDrag {
        button: MouseButton,
        modifier_keys: ModifierKeys,
        position: f32x2,
        drag_amount: f32x2,
    },
    KeyDown {
        key_code: c_ushort,
        modifier_keys: ModifierKeys,
    },
    WindowFocusedOrResized {
        size: f32x2,
    },
}

// TODO: Apply constants to all projects doing key capture w/magic values.
impl UserEvent {
    pub const KEY_CODE_DOWN: c_ushort = 125;
    pub const KEY_CODE_LEFT: c_ushort = 123;
    pub const KEY_CODE_RIGHT: c_ushort = 124;
    pub const KEY_CODE_UP: c_ushort = 126;
    pub const KEY_CODE_SPACEBAR: c_ushort = 49;
}

pub fn remove_modifier_keys(event: UserEvent, modifier_keys_to_remove: ModifierKeys) -> UserEvent {
    match event {
        UserEvent::MouseDown {
            button,
            modifier_keys,
            position,
        } => UserEvent::MouseDown {
            button,
            position,
            modifier_keys: modifier_keys.difference(modifier_keys_to_remove),
        },
        UserEvent::MouseUp {
            button,
            modifier_keys,
            position,
        } => UserEvent::MouseUp {
            button,
            position,
            modifier_keys: modifier_keys.difference(modifier_keys_to_remove),
        },
        UserEvent::MouseDrag {
            button,
            modifier_keys,
            position,
            drag_amount,
        } => UserEvent::MouseDrag {
            button,
            modifier_keys: modifier_keys.difference(modifier_keys_to_remove),
            position,
            drag_amount,
        },
        UserEvent::KeyDown {
            key_code,
            modifier_keys,
        } => UserEvent::KeyDown {
            key_code,
            modifier_keys: modifier_keys.difference(modifier_keys_to_remove),
        },
        e @ _ => e,
    }
}

pub trait RendererDelgate {
    fn new(device: Device) -> Self;

    fn device(&self) -> &Device;

    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef;

    #[inline]
    fn on_event(&mut self, _event: UserEvent) {}

    #[inline]
    fn needs_render(&self) -> bool {
        true
    }
}

pub(crate) struct MetalRenderer<R: RendererDelgate> {
    backing_scale_factor: f32,
    pub(crate) layer: MetalLayer,
    screen_size: f32x2,
    delegate: R,
}

unsafe impl<R: RendererDelgate> Send for MetalRenderer<R> {}

impl<R: RendererDelgate> MetalRenderer<R> {
    #[inline]
    pub(crate) fn new(backing_scale_factor: f32) -> MetalRenderer<R> {
        let device = Device::system_default().expect("No device found");
        let layer = MetalLayer::new();
        layer.set_device(&device);
        layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        layer.set_framebuffer_only(YES);
        layer.set_presents_with_transaction(false);
        Self {
            backing_scale_factor,
            delegate: R::new(device),
            layer,
            screen_size: f32x2::splat(0.0),
        }
    }

    #[inline]
    pub(crate) fn update_size(&mut self, size: f32x2) {
        let size = size * f32x2::splat(self.backing_scale_factor);
        if self.screen_size != size {
            self.layer
                .set_drawable_size(CGSize::new(size[0] as CGFloat, size[1] as CGFloat));
            self.screen_size = size;
            self.delegate
                .on_event(UserEvent::WindowFocusedOrResized { size });
        }
    }

    #[inline]
    pub(crate) fn needs_render(&mut self) -> bool {
        self.delegate.needs_render()
    }

    #[inline]
    pub(crate) fn render(&mut self) {
        autoreleasepool(|| {
            if let Some(drawable) = self.layer.next_drawable() {
                let command_buffer = self.delegate.render(drawable.texture());
                command_buffer.present_drawable(drawable);
                command_buffer.commit();
                // TODO: Implement Triple Buffering
                command_buffer.wait_until_completed();
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
