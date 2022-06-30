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

#[repr(C)]
#[repr(align(16))]
pub struct Space {
    pub matrix_world_to_projection: float4x4,
    pub matrix_screen_to_world: float4x4,
    pub position_world: float4,
}
#[test]
fn bindgen_test_layout_Space() {
    assert_eq!(
        ::std::mem::size_of::<Space>(),
        144usize,
        concat!("Size of: ", stringify!(Space))
    );
    assert_eq!(
        ::std::mem::align_of::<Space>(),
        16usize,
        concat!("Alignment of ", stringify!(Space))
    );
    fn test_field_matrix_world_to_projection() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Space>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).matrix_world_to_projection) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(Space),
                "::",
                stringify!(matrix_world_to_projection)
            )
        );
    }
    test_field_matrix_world_to_projection();
    fn test_field_matrix_screen_to_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Space>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).matrix_screen_to_world) as usize - ptr as usize
            },
            64usize,
            concat!(
                "Offset of field: ",
                stringify!(Space),
                "::",
                stringify!(matrix_screen_to_world)
            )
        );
    }
    test_field_matrix_screen_to_world();
    fn test_field_position_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Space>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).position_world) as usize - ptr as usize
            },
            128usize,
            concat!(
                "Offset of field: ",
                stringify!(Space),
                "::",
                stringify!(position_world)
            )
        );
    }
    test_field_position_world();
}
#[repr(u32)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum VertexBufferIndex {
    CameraSpace = 0,
    LENGTH = 1,
}
#[repr(u32)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum FragBufferIndex {
    CameraSpace = 0,
    LightSpace = 1,
    LENGTH = 2,
}
