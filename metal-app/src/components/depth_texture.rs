use super::texture_and_config::TextureAndConfig;
use crate::UserEvent;
use metal::{DeviceRef, MTLPixelFormat, MTLStorageMode, MTLTextureUsage, TextureRef};

pub struct DepthTexture(TextureAndConfig);
impl DepthTexture {
    #[inline]
    pub fn new(label: &'static str, format: MTLPixelFormat) -> Self {
        Self(TextureAndConfig {
            label,
            format,
            texture: None,
            storage_mode: MTLStorageMode::Memoryless,
            usage: MTLTextureUsage::RenderTarget,
        })
    }

    #[inline]
    pub fn new_with_storage_mode(
        label: &'static str,
        format: MTLPixelFormat,
        storage_mode: MTLStorageMode,
    ) -> Self {
        Self(TextureAndConfig {
            label,
            format,
            texture: None,
            storage_mode,
            usage: MTLTextureUsage::RenderTarget,
        })
    }

    #[inline]
    pub fn texture(&self) -> &TextureRef {
        self.0.texture()
    }

    #[inline]
    pub fn on_event(&mut self, event: UserEvent, device: &DeviceRef) -> bool {
        self.0.on_event(event, device, |s| Some(s))
    }
}
