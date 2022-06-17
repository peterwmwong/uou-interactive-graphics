#![allow(deref_nullptr, non_upper_case_globals, non_snake_case)]
/**************************************************************************************************
 GENERATED FILE. DO NOT MODIFY.

 This file is generated by the `metal_build` crate, check you're `build.rs` for
 `metal_build::build()`.
 Structs and Enums are generated based on `shader_src/common.h`.
***************************************************************************************************/
/* automatically generated by rust-bindgen 0.59.2 */

#[repr(C)]
#[repr(align(4))]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct half2 {
    pub x: u16,
    pub y: u16,
}
#[test]
fn bindgen_test_layout_half2() {
    assert_eq!(
        ::std::mem::size_of::<half2>(),
        4usize,
        concat!("Size of: ", stringify!(half2))
    );
    assert_eq!(
        ::std::mem::align_of::<half2>(),
        4usize,
        concat!("Alignment of ", stringify!(half2))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<half2>())).x as *const _ as usize },
        0usize,
        concat!("Offset of field: ", stringify!(half2), "::", stringify!(x))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<half2>())).y as *const _ as usize },
        2usize,
        concat!("Offset of field: ", stringify!(half2), "::", stringify!(y))
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct packed_half2 {
    pub x: u16,
    pub y: u16,
}
#[test]
fn bindgen_test_layout_packed_half2() {
    assert_eq!(
        ::std::mem::size_of::<packed_half2>(),
        4usize,
        concat!("Size of: ", stringify!(packed_half2))
    );
    assert_eq!(
        ::std::mem::align_of::<packed_half2>(),
        2usize,
        concat!("Alignment of ", stringify!(packed_half2))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<packed_half2>())).x as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(packed_half2),
            "::",
            stringify!(x)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<packed_half2>())).y as *const _ as usize },
        2usize,
        concat!(
            "Offset of field: ",
            stringify!(packed_half2),
            "::",
            stringify!(y)
        )
    );
}
#[repr(C)]
#[repr(align(8))]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct half4 {
    pub x: u16,
    pub y: u16,
    pub z: u16,
    pub w: u16,
}
#[test]
fn bindgen_test_layout_half4() {
    assert_eq!(
        ::std::mem::size_of::<half4>(),
        8usize,
        concat!("Size of: ", stringify!(half4))
    );
    assert_eq!(
        ::std::mem::align_of::<half4>(),
        8usize,
        concat!("Alignment of ", stringify!(half4))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<half4>())).x as *const _ as usize },
        0usize,
        concat!("Offset of field: ", stringify!(half4), "::", stringify!(x))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<half4>())).y as *const _ as usize },
        2usize,
        concat!("Offset of field: ", stringify!(half4), "::", stringify!(y))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<half4>())).z as *const _ as usize },
        4usize,
        concat!("Offset of field: ", stringify!(half4), "::", stringify!(z))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<half4>())).w as *const _ as usize },
        6usize,
        concat!("Offset of field: ", stringify!(half4), "::", stringify!(w))
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct packed_half4 {
    pub x: u16,
    pub y: u16,
    pub z: u16,
    pub w: u16,
}
#[test]
fn bindgen_test_layout_packed_half4() {
    assert_eq!(
        ::std::mem::size_of::<packed_half4>(),
        8usize,
        concat!("Size of: ", stringify!(packed_half4))
    );
    assert_eq!(
        ::std::mem::align_of::<packed_half4>(),
        2usize,
        concat!("Alignment of ", stringify!(packed_half4))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<packed_half4>())).x as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(packed_half4),
            "::",
            stringify!(x)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<packed_half4>())).y as *const _ as usize },
        2usize,
        concat!(
            "Offset of field: ",
            stringify!(packed_half4),
            "::",
            stringify!(y)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<packed_half4>())).z as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(packed_half4),
            "::",
            stringify!(z)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<packed_half4>())).w as *const _ as usize },
        6usize,
        concat!(
            "Offset of field: ",
            stringify!(packed_half4),
            "::",
            stringify!(w)
        )
    );
}
#[repr(C)]
#[repr(align(8))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct float2 {
    pub x: f32,
    pub y: f32,
}
#[test]
fn bindgen_test_layout_float2() {
    assert_eq!(
        ::std::mem::size_of::<float2>(),
        8usize,
        concat!("Size of: ", stringify!(float2))
    );
    assert_eq!(
        ::std::mem::align_of::<float2>(),
        8usize,
        concat!("Alignment of ", stringify!(float2))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<float2>())).x as *const _ as usize },
        0usize,
        concat!("Offset of field: ", stringify!(float2), "::", stringify!(x))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<float2>())).y as *const _ as usize },
        4usize,
        concat!("Offset of field: ", stringify!(float2), "::", stringify!(y))
    );
}
#[repr(C)]
#[repr(align(16))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct float4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}
#[test]
fn bindgen_test_layout_float4() {
    assert_eq!(
        ::std::mem::size_of::<float4>(),
        16usize,
        concat!("Size of: ", stringify!(float4))
    );
    assert_eq!(
        ::std::mem::align_of::<float4>(),
        16usize,
        concat!("Alignment of ", stringify!(float4))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<float4>())).x as *const _ as usize },
        0usize,
        concat!("Offset of field: ", stringify!(float4), "::", stringify!(x))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<float4>())).y as *const _ as usize },
        4usize,
        concat!("Offset of field: ", stringify!(float4), "::", stringify!(y))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<float4>())).z as *const _ as usize },
        8usize,
        concat!("Offset of field: ", stringify!(float4), "::", stringify!(z))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<float4>())).w as *const _ as usize },
        12usize,
        concat!("Offset of field: ", stringify!(float4), "::", stringify!(w))
    );
}
#[repr(C)]
#[repr(align(4))]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ushort2 {
    pub x: ::std::os::raw::c_ushort,
    pub y: ::std::os::raw::c_ushort,
}
#[test]
fn bindgen_test_layout_ushort2() {
    assert_eq!(
        ::std::mem::size_of::<ushort2>(),
        4usize,
        concat!("Size of: ", stringify!(ushort2))
    );
    assert_eq!(
        ::std::mem::align_of::<ushort2>(),
        4usize,
        concat!("Alignment of ", stringify!(ushort2))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<ushort2>())).x as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(ushort2),
            "::",
            stringify!(x)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<ushort2>())).y as *const _ as usize },
        2usize,
        concat!(
            "Offset of field: ",
            stringify!(ushort2),
            "::",
            stringify!(y)
        )
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct packed_float2 {
    pub x: f32,
    pub y: f32,
}
#[test]
fn bindgen_test_layout_packed_float2() {
    assert_eq!(
        ::std::mem::size_of::<packed_float2>(),
        8usize,
        concat!("Size of: ", stringify!(packed_float2))
    );
    assert_eq!(
        ::std::mem::align_of::<packed_float2>(),
        4usize,
        concat!("Alignment of ", stringify!(packed_float2))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<packed_float2>())).x as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(packed_float2),
            "::",
            stringify!(x)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<packed_float2>())).y as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(packed_float2),
            "::",
            stringify!(y)
        )
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct packed_float4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}
#[test]
fn bindgen_test_layout_packed_float4() {
    assert_eq!(
        ::std::mem::size_of::<packed_float4>(),
        16usize,
        concat!("Size of: ", stringify!(packed_float4))
    );
    assert_eq!(
        ::std::mem::align_of::<packed_float4>(),
        4usize,
        concat!("Alignment of ", stringify!(packed_float4))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<packed_float4>())).x as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(packed_float4),
            "::",
            stringify!(x)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<packed_float4>())).y as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(packed_float4),
            "::",
            stringify!(y)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<packed_float4>())).z as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(packed_float4),
            "::",
            stringify!(z)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<packed_float4>())).w as *const _ as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(packed_float4),
            "::",
            stringify!(w)
        )
    );
}
#[repr(i32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum VertexBufferIndex {
    MatrixModelToProjection = 0,
    LENGTH = 1,
}
#[repr(i32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum FragBufferIndex {
    Texture = 0,
    LENGTH = 1,
}

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