use super::texture_and_config::TextureAndConfig;
use crate::UserEvent;
use metal::{DeviceRef, MTLPixelFormat, MTLResourceOptions, MTLTextureUsage, TextureRef};

pub struct DepthTexture(TextureAndConfig);
impl DepthTexture {
    #[inline]
    pub fn new(label: &'static str, format: MTLPixelFormat) -> Self {
        Self(TextureAndConfig {
            label,
            format,
            texture: None,
            resource_options: MTLResourceOptions::StorageModePrivate,
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
