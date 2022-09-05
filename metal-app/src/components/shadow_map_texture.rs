use super::texture_and_config::TextureAndConfig;
use crate::{math_helpers::round_up_pow_of_2, UserEvent};
use metal::{DeviceRef, MTLPixelFormat, MTLResourceOptions, MTLTextureUsage, TextureRef};
use std::simd::u32x2;

const MAX_TEXTURE_SIZE: u16 = 16384;

pub struct ShadowMapTexture(TextureAndConfig);
impl ShadowMapTexture {
    #[inline]
    pub fn new(label: &'static str, format: MTLPixelFormat) -> Self {
        Self(TextureAndConfig {
            label,
            format,
            texture: None,
            resource_options: MTLResourceOptions::StorageModePrivate,
            usage: MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead,
        })
    }

    #[inline]
    pub fn texture(&self) -> &TextureRef {
        self.0.texture()
    }

    #[inline]
    pub fn on_event(&mut self, event: UserEvent, device: &DeviceRef) -> bool {
        let cur_wh: Option<(usize, usize)> = self
            .0
            .texture
            .as_ref()
            .map(|tx| (tx.width() as _, tx.height() as _));
        self.0.on_event(event, device, |wh| {
            if let Some(cur_wh) = cur_wh {
                #[inline(always)]
                fn is_shadow_map_correctly_sized(cur: usize, target: u32) -> bool {
                    ((target << 1)..=(target << 2)).contains(&(cur as _))
                }
                if is_shadow_map_correctly_sized(cur_wh.0 as _, wh[0])
                    && is_shadow_map_correctly_sized(cur_wh.1 as _, wh[1])
                {
                    return None;
                }
            }
            let new_wh = round_up_pow_of_2(wh << u32x2::splat(1));
            #[cfg(debug_assertions)]
            println!("Allocating new Shadow Map {new_wh:?}");
            Some(new_wh.min(u32x2::splat(MAX_TEXTURE_SIZE as _)))
        })
    }
}
