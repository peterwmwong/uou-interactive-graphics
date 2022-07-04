#![allow(deref_nullptr, non_upper_case_globals, non_snake_case)]
/**************************************************************************************************
 GENERATED FILE. DO NOT MODIFY.

 This file is generated by the `metal_build` crate, check you're `build.rs` for
 `metal_build::build()`.
 Structs and Enums are generated based on `shader_src/shader_bindings.h`.
***************************************************************************************************/
#[allow(unused_imports)]
use metal_app::metal_types::*;
/* automatically generated by rust-bindgen 0.60.1 */

pub const INITIAL_CAMERA_DISTANCE: f32 = 50.0;
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Geometry {
    pub indices: ::std::os::raw::c_ulong,
    pub positions: ::std::os::raw::c_ulong,
}
#[test]
fn bindgen_test_layout_Geometry() {
    assert_eq!(
        ::std::mem::size_of::<Geometry>(),
        16usize,
        concat!("Size of: ", stringify!(Geometry))
    );
    assert_eq!(
        ::std::mem::align_of::<Geometry>(),
        8usize,
        concat!("Alignment of ", stringify!(Geometry))
    );
    fn test_field_indices() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Geometry>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).indices) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(Geometry),
                "::",
                stringify!(indices)
            )
        );
    }
    test_field_indices();
    fn test_field_positions() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Geometry>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).positions) as usize - ptr as usize
            },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(Geometry),
                "::",
                stringify!(positions)
            )
        );
    }
    test_field_positions();
}
#[repr(C)]
#[repr(align(16))]
pub struct VertexInput {
    pub mins: float4,
    pub maxs: float4,
    pub screen_size: float2,
    pub camera_rotation: float2,
    pub camera_distance: f32,
    pub use_perspective: bool,
}
#[test]
fn bindgen_test_layout_VertexInput() {
    assert_eq!(
        ::std::mem::size_of::<VertexInput>(),
        64usize,
        concat!("Size of: ", stringify!(VertexInput))
    );
    assert_eq!(
        ::std::mem::align_of::<VertexInput>(),
        16usize,
        concat!("Alignment of ", stringify!(VertexInput))
    );
    fn test_field_mins() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<VertexInput>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).mins) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(VertexInput),
                "::",
                stringify!(mins)
            )
        );
    }
    test_field_mins();
    fn test_field_maxs() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<VertexInput>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).maxs) as usize - ptr as usize
            },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(VertexInput),
                "::",
                stringify!(maxs)
            )
        );
    }
    test_field_maxs();
    fn test_field_screen_size() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<VertexInput>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).screen_size) as usize - ptr as usize
            },
            32usize,
            concat!(
                "Offset of field: ",
                stringify!(VertexInput),
                "::",
                stringify!(screen_size)
            )
        );
    }
    test_field_screen_size();
    fn test_field_camera_rotation() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<VertexInput>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).camera_rotation) as usize - ptr as usize
            },
            40usize,
            concat!(
                "Offset of field: ",
                stringify!(VertexInput),
                "::",
                stringify!(camera_rotation)
            )
        );
    }
    test_field_camera_rotation();
    fn test_field_camera_distance() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<VertexInput>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).camera_distance) as usize - ptr as usize
            },
            48usize,
            concat!(
                "Offset of field: ",
                stringify!(VertexInput),
                "::",
                stringify!(camera_distance)
            )
        );
    }
    test_field_camera_distance();
    fn test_field_use_perspective() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<VertexInput>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).use_perspective) as usize - ptr as usize
            },
            52usize,
            concat!(
                "Offset of field: ",
                stringify!(VertexInput),
                "::",
                stringify!(use_perspective)
            )
        );
    }
    test_field_use_perspective();
}
#[repr(i32)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum VertexBufferIndex {
    VertexInput = 0,
    Geometry = 1,
    LENGTH = 2,
}
