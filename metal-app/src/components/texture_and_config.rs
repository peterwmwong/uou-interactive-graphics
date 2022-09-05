use crate::UserEvent;
use metal::{
    DeviceRef, MTLPixelFormat, MTLResourceOptions, MTLTextureUsage, Texture, TextureDescriptor,
    TextureRef,
};
use std::simd::u32x2;

pub(crate) struct TextureAndConfig {
    pub(crate) label: &'static str,
    pub(crate) format: MTLPixelFormat,
    pub(crate) texture: Option<Texture>,
    pub(crate) resource_options: MTLResourceOptions,
    pub(crate) usage: MTLTextureUsage,
}

impl TextureAndConfig {
    #[inline]
    pub(crate) fn texture(&self) -> &TextureRef {
        self.texture.as_deref().expect("Failed to access Texture")
    }

    #[inline]
    pub(crate) fn on_event<F: Fn(u32x2) -> Option<u32x2>>(
        &mut self,
        event: UserEvent,
        device: &DeviceRef,
        sizer: F,
    ) -> bool {
        match event {
            UserEvent::WindowFocusedOrResized { size, .. } => {
                if let Some(size) = sizer(unsafe { size.to_int_unchecked() }) {
                    let desc = TextureDescriptor::new();
                    desc.set_width(size[0] as _);
                    desc.set_height(size[1] as _);
                    desc.set_pixel_format(self.format);
                    desc.set_usage(self.usage);
                    desc.set_resource_options(self.resource_options);
                    let texture = device.new_texture(&desc);
                    texture.set_label(self.label);
                    self.texture = Some(texture);
                    return true;
                }
            }
            _ => {}
        }
        return false;
    }
}
