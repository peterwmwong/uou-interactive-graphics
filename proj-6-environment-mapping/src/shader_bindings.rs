#![allow(deref_nullptr, non_upper_case_globals, non_snake_case)]
/**************************************************************************************************
 GENERATED FILE. DO NOT MODIFY.

 This file is generated by the `metal_build` crate, check you're `build.rs` for
 `metal_build::build()`.
 Structs and Enums are generated based on `shader_src/common.h`.
***************************************************************************************************/
#[allow(unused_imports)]
use metal_app::metal_types::*;
/* automatically generated by rust-bindgen 0.60.1 */

pub const MIRRORED_INSTANCE_ID: ::std::os::raw::c_ushort = 1;
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
pub struct World {
    pub matrix_model_to_projection: float4x4,
    pub matrix_model_to_world: float4x4,
    pub matrix_normal_to_world: float3x3,
    pub matrix_world_to_projection: float4x4,
    pub matrix_screen_to_world: float4x4,
    pub camera_position: float4,
    pub plane_y: f32,
}
#[test]
fn bindgen_test_layout_World() {
    assert_eq!(
        ::std::mem::size_of::<World>(),
        336usize,
        concat!("Size of: ", stringify!(World))
    );
    assert_eq!(
        ::std::mem::align_of::<World>(),
        16usize,
        concat!("Alignment of ", stringify!(World))
    );
    fn test_field_matrix_model_to_projection() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<World>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).matrix_model_to_projection) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(World),
                "::",
                stringify!(matrix_model_to_projection)
            )
        );
    }
    test_field_matrix_model_to_projection();
    fn test_field_matrix_model_to_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<World>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).matrix_model_to_world) as usize - ptr as usize
            },
            64usize,
            concat!(
                "Offset of field: ",
                stringify!(World),
                "::",
                stringify!(matrix_model_to_world)
            )
        );
    }
    test_field_matrix_model_to_world();
    fn test_field_matrix_normal_to_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<World>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).matrix_normal_to_world) as usize - ptr as usize
            },
            128usize,
            concat!(
                "Offset of field: ",
                stringify!(World),
                "::",
                stringify!(matrix_normal_to_world)
            )
        );
    }
    test_field_matrix_normal_to_world();
    fn test_field_matrix_world_to_projection() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<World>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).matrix_world_to_projection) as usize - ptr as usize
            },
            176usize,
            concat!(
                "Offset of field: ",
                stringify!(World),
                "::",
                stringify!(matrix_world_to_projection)
            )
        );
    }
    test_field_matrix_world_to_projection();
    fn test_field_matrix_screen_to_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<World>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).matrix_screen_to_world) as usize - ptr as usize
            },
            240usize,
            concat!(
                "Offset of field: ",
                stringify!(World),
                "::",
                stringify!(matrix_screen_to_world)
            )
        );
    }
    test_field_matrix_screen_to_world();
    fn test_field_camera_position() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<World>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).camera_position) as usize - ptr as usize
            },
            304usize,
            concat!(
                "Offset of field: ",
                stringify!(World),
                "::",
                stringify!(camera_position)
            )
        );
    }
    test_field_camera_position();
    fn test_field_plane_y() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<World>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).plane_y) as usize - ptr as usize
            },
            320usize,
            concat!(
                "Offset of field: ",
                stringify!(World),
                "::",
                stringify!(plane_y)
            )
        );
    }
    test_field_plane_y();
}
#[repr(u32)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum BGFragBufferIndex {
    World = 0,
    LENGTH = 1,
}
#[repr(u32)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum BGFragTextureIndex {
    CubeMapTexture = 0,
}
#[repr(u32)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum VertexBufferIndex {
    World = 0,
    Geometry = 1,
    LENGTH = 2,
}
#[repr(u32)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum FragBufferIndex {
    World = 0,
    LENGTH = 1,
}
#[repr(u32)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum FragTextureIndex {
    CubeMapTexture = 0,
    ModelTexture = 1,
}
