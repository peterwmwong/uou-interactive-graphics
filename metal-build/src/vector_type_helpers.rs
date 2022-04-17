include!(concat!(
    env!("OUT_DIR"),
    "/rust-bindgen-only-vector-type-bindings.rs"
));
// APPEND THE FOLLOWING TO `shader_bindings.rs`

/**************************************************************************************************
 Helper methods and trait implementations make it easier to write and read vector types from Metal.
 See `metal-build/src/vector_type_helpers.rs`.
***************************************************************************************************/
use metal_app::half::f16;
use std::simd::Simd;

impl From<Simd<f32, 2>> for packed_half2 {
    #[inline]
    fn from(simd: Simd<f32, 2>) -> Self {
        packed_half2 {
            x: f16::from_f32(simd[0]).to_bits(),
            y: f16::from_f32(simd[1]).to_bits(),
        }
    }
}

impl From<Simd<f32, 4>> for packed_half4 {
    #[inline]
    fn from(simd: Simd<f32, 4>) -> Self {
        packed_half4 {
            x: f16::from_f32(simd[0]).to_bits(),
            y: f16::from_f32(simd[1]).to_bits(),
            z: f16::from_f32(simd[2]).to_bits(),
            w: f16::from_f32(simd[3]).to_bits(),
        }
    }
}

impl From<Simd<f32, 4>> for half4 {
    #[inline]
    fn from(simd: Simd<f32, 4>) -> Self {
        half4 {
            x: f16::from_f32(simd[0]).to_bits(),
            y: f16::from_f32(simd[1]).to_bits(),
            z: f16::from_f32(simd[2]).to_bits(),
            w: f16::from_f32(simd[3]).to_bits(),
        }
    }
}

impl float2 {
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<Simd<f32, 2>> for float2 {
    #[inline]
    fn from(simd: Simd<f32, 2>) -> Self {
        // TODO: Add some tests to verify this actually correct for whatever platfrom this is
        // running on.
        unsafe { std::mem::transmute(simd) }
    }
}

impl From<float2> for Simd<f32, 2> {
    #[inline]
    fn from(f: float2) -> Self {
        unsafe { std::mem::transmute(f) }
    }
}

impl ushort2 {
    #[inline]
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

impl From<Simd<u16, 2>> for ushort2 {
    #[inline]
    fn from(simd: Simd<u16, 2>) -> Self {
        unsafe { std::mem::transmute(simd) }
    }
}

impl From<ushort2> for Simd<u16, 2> {
    #[inline]
    fn from(f: ushort2) -> Self {
        unsafe { std::mem::transmute(f) }
    }
}

impl packed_float2 {
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<Simd<f32, 2>> for packed_float2 {
    #[inline]
    fn from(simd: Simd<f32, 2>) -> Self {
        unsafe { std::mem::transmute(simd) }
    }
}

impl From<packed_float2> for Simd<f32, 2> {
    #[inline]
    fn from(f: packed_float2) -> Self {
        unsafe { std::mem::transmute(f) }
    }
}

impl float4 {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

impl From<Simd<f32, 4>> for float4 {
    #[inline]
    fn from(simd: Simd<f32, 4>) -> Self {
        unsafe { std::mem::transmute(simd) }
    }
}

impl From<float4> for Simd<f32, 4> {
    #[inline]
    fn from(f: float4) -> Self {
        unsafe { std::mem::transmute(f) }
    }
}

impl packed_float4 {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

impl From<Simd<f32, 4>> for packed_float4 {
    #[inline]
    fn from(simd: Simd<f32, 4>) -> Self {
        unsafe { std::mem::transmute(simd) }
    }
}

impl From<packed_float4> for Simd<f32, 4> {
    #[inline]
    fn from(f: packed_float4) -> Self {
        unsafe { std::mem::transmute(f) }
    }
}
