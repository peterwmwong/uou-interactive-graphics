use crate::UserEvent;
use metal::{
    DeviceRef, MTLPixelFormat, MTLStorageMode, MTLTextureUsage, Texture, TextureDescriptor,
    TextureRef,
};

pub struct DepthTexture {
    label: &'static str,
    format: MTLPixelFormat,
    texture: Option<Texture>,
    storage_mode: MTLStorageMode,
    usage: MTLTextureUsage,
}

impl DepthTexture {
    #[inline]
    pub fn new(label: &'static str, format: MTLPixelFormat) -> Self {
        Self {
            label,
            format,
            texture: None,
            storage_mode: MTLStorageMode::Memoryless,
            usage: MTLTextureUsage::RenderTarget,
        }
    }

    #[inline]
    pub fn texture(&self) -> &TextureRef {
        self.texture
            .as_deref()
            .expect("Failed to access Depth Texture")
    }

    #[inline]
    pub fn on_event(&mut self, event: UserEvent, device: &DeviceRef) -> bool {
        match event {
            UserEvent::WindowFocusedOrResized { size, .. } => {
                let desc = TextureDescriptor::new();
                desc.set_width(size[0] as _);
                desc.set_height(size[1] as _);
                desc.set_pixel_format(self.format);
                desc.set_storage_mode(self.storage_mode);
                desc.set_usage(self.usage);
                let depth_texture = device.new_texture(&desc);
                depth_texture.set_label(self.label);
                self.texture = Some(depth_texture);
                true
            }
            _ => false,
        }
    }
}
