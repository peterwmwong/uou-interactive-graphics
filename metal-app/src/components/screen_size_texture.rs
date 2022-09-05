use metal::{DeviceRef, MTLPixelFormat, MTLResourceOptions, MTLTextureUsage, TextureRef};

use crate::UserEvent;

use super::texture_and_config::TextureAndConfig;

pub struct ScreenSizeTexture(TextureAndConfig);

impl ScreenSizeTexture {
    pub const DEFAULT_DEPTH_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;

    #[inline]
    pub fn new(
        label: &'static str,
        format: MTLPixelFormat,
        resource_options: MTLResourceOptions,
        usage: MTLTextureUsage,
    ) -> Self {
        Self(TextureAndConfig {
            label,
            format,
            texture: None,
            resource_options,
            usage,
        })
    }

    #[inline]
    pub fn new_memoryless_render_target(label: &'static str, format: MTLPixelFormat) -> Self {
        Self::new(
            label,
            format,
            MTLResourceOptions::HazardTrackingModeUntracked
                | MTLResourceOptions::StorageModeMemoryless,
            MTLTextureUsage::RenderTarget,
        )
    }

    #[inline]
    pub fn new_depth() -> Self {
        Self::new_memoryless_render_target("Depth", Self::DEFAULT_DEPTH_FORMAT)
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
