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
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Geometry {
    pub indices: ::std::os::raw::c_ulong,
    pub positions: ::std::os::raw::c_ulong,
    pub normals: ::std::os::raw::c_ulong,
    pub tx_coords: ::std::os::raw::c_ulong,
}
#[test]
fn bindgen_test_layout_Geometry() {
    assert_eq!(
        ::std::mem::size_of::<Geometry>(),
        32usize,
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
    fn test_field_normals() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Geometry>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).normals) as usize - ptr as usize
            },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(Geometry),
                "::",
                stringify!(normals)
            )
        );
    }
    test_field_normals();
    fn test_field_tx_coords() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Geometry>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).tx_coords) as usize - ptr as usize
            },
            24usize,
            concat!(
                "Offset of field: ",
                stringify!(Geometry),
                "::",
                stringify!(tx_coords)
            )
        );
    }
    test_field_tx_coords();
}
#[repr(C)]
#[repr(align(16))]
pub struct ModelSpace {
    pub matrix_model_to_projection: float4x4,
    pub matrix_normal_to_world: float3x3,
}
#[test]
fn bindgen_test_layout_ModelSpace() {
    assert_eq!(
        ::std::mem::size_of::<ModelSpace>(),
        112usize,
        concat!("Size of: ", stringify!(ModelSpace))
    );
    assert_eq!(
        ::std::mem::align_of::<ModelSpace>(),
        16usize,
        concat!("Alignment of ", stringify!(ModelSpace))
    );
    fn test_field_matrix_model_to_projection() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ModelSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).matrix_model_to_projection) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(ModelSpace),
                "::",
                stringify!(matrix_model_to_projection)
            )
        );
    }
    test_field_matrix_model_to_projection();
    fn test_field_matrix_normal_to_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ModelSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).matrix_normal_to_world) as usize - ptr as usize
            },
            64usize,
            concat!(
                "Offset of field: ",
                stringify!(ModelSpace),
                "::",
                stringify!(matrix_normal_to_world)
            )
        );
    }
    test_field_matrix_normal_to_world();
}
#[repr(C)]
#[repr(align(16))]
pub struct ProjectedSpace {
    pub matrix_world_to_projection: float4x4,
    pub matrix_screen_to_world: float4x4,
    pub position_world: float4,
}
#[test]
fn bindgen_test_layout_ProjectedSpace() {
    assert_eq!(
        ::std::mem::size_of::<ProjectedSpace>(),
        144usize,
        concat!("Size of: ", stringify!(ProjectedSpace))
    );
    assert_eq!(
        ::std::mem::align_of::<ProjectedSpace>(),
        16usize,
        concat!("Alignment of ", stringify!(ProjectedSpace))
    );
    fn test_field_matrix_world_to_projection() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ProjectedSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).matrix_world_to_projection) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(ProjectedSpace),
                "::",
                stringify!(matrix_world_to_projection)
            )
        );
    }
    test_field_matrix_world_to_projection();
    fn test_field_matrix_screen_to_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ProjectedSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).matrix_screen_to_world) as usize - ptr as usize
            },
            64usize,
            concat!(
                "Offset of field: ",
                stringify!(ProjectedSpace),
                "::",
                stringify!(matrix_screen_to_world)
            )
        );
    }
    test_field_matrix_screen_to_world();
    fn test_field_position_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ProjectedSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).position_world) as usize - ptr as usize
            },
            128usize,
            concat!(
                "Offset of field: ",
                stringify!(ProjectedSpace),
                "::",
                stringify!(position_world)
            )
        );
    }
    test_field_position_world();
}
#[repr(u8)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum ShadingMode {
    HasAmbient = 0,
    HasDiffuse = 1,
    OnlyNormals = 2,
    HasSpecular = 3,
}
#[repr(u32)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum VertexBufferIndex {
    Geometry = 0,
    Camera = 1,
    Model = 2,
    LENGTH = 3,
}
#[repr(u32)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum FragBufferIndex {
    Camera = 0,
    LightPosition = 1,
    MatrixEnvironment = 2,
    Darken = 3,
    LENGTH = 4,
}
#[repr(u32)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum FragTextureIndex {
    EnvTexture = 0,
}
