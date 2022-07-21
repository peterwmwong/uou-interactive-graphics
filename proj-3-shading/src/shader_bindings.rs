#![allow(deref_nullptr, non_upper_case_globals, non_snake_case)]
/**************************************************************************************************
 GENERATED FILE. DO NOT MODIFY.

 This file is generated by the `metal_build` crate, check you're `build.rs` for
 `metal_build::build()`.
 Structs and Enums are generated based on `shader_src/shader_bindings.h`.
***************************************************************************************************/
#[allow(unused_imports)]
use metal_app::{metal::*, metal_types::*, render_pipeline::*};
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
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
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

/******************
 Function Constants
*******************/

pub struct FunctionConstants {
    pub HasAmbient: bool,
    pub HasDiffuse: bool,
    pub OnlyNormals: bool,
    pub HasSpecular: bool,
}
impl FunctionConstantsFactory for FunctionConstants {
    #[inline]
    fn create_function_constant_values(&self) -> Option<FunctionConstantValues> {
        let fcv = FunctionConstantValues::new();
        fcv.set_constant_value_at_index((&self.HasAmbient as *const _) as _, bool::MTL_DATA_TYPE, 0);
        fcv.set_constant_value_at_index((&self.HasDiffuse as *const _) as _, bool::MTL_DATA_TYPE, 1);
        fcv.set_constant_value_at_index((&self.OnlyNormals as *const _) as _, bool::MTL_DATA_TYPE, 2);
        fcv.set_constant_value_at_index((&self.HasSpecular as *const _) as _, bool::MTL_DATA_TYPE, 3);
        Some(fcv)
    }
}

/****************
 Shader functions
*****************/

#[allow(non_camel_case_types)]
pub struct main_vertex_binds<'c> {
    pub geometry: BindOne<'c, Geometry>,
    pub model: BindOne<'c, ModelSpace>,
}
impl FunctionBinds for main_vertex_binds<'_> {
    #[inline]
    fn encode_binds<E: BindEncoder>(self, encoder: &RenderCommandEncoderRef) {
        E::encode_one(encoder, self.geometry, 0);
        E::encode_one(encoder, self.model, 1);
    }
}

#[allow(non_camel_case_types)]
pub struct main_vertex;
impl metal_app::render_pipeline::Function for main_vertex {
    const FUNCTION_NAME: &'static str = "main_vertex";
    type Binds<'c> = main_vertex_binds<'c>;
    type Type = VertexFunctionType;
    type FunctionConstantsType = FunctionConstants;
}

#[allow(non_camel_case_types)]
pub struct main_fragment_binds<'c> {
    pub camera: BindOne<'c, ProjectedSpace>,
    pub light_pos: BindOne<'c, float4>,
}
impl FunctionBinds for main_fragment_binds<'_> {
    #[inline]
    fn encode_binds<E: BindEncoder>(self, encoder: &RenderCommandEncoderRef) {
        E::encode_one(encoder, self.camera, 0);
        E::encode_one(encoder, self.light_pos, 1);
    }
}

#[allow(non_camel_case_types)]
pub struct main_fragment;
impl metal_app::render_pipeline::Function for main_fragment {
    const FUNCTION_NAME: &'static str = "main_fragment";
    type Binds<'c> = main_fragment_binds<'c>;
    type Type = FragmentFunctionType;
    type FunctionConstantsType = FunctionConstants;
}

#[allow(non_camel_case_types)]
pub struct light_vertex_binds<'c> {
    pub camera: BindOne<'c, ProjectedSpace>,
    pub light_pos: BindOne<'c, float4>,
}
impl FunctionBinds for light_vertex_binds<'_> {
    #[inline]
    fn encode_binds<E: BindEncoder>(self, encoder: &RenderCommandEncoderRef) {
        E::encode_one(encoder, self.camera, 0);
        E::encode_one(encoder, self.light_pos, 1);
    }
}

#[allow(non_camel_case_types)]
pub struct light_vertex;
impl metal_app::render_pipeline::Function for light_vertex {
    const FUNCTION_NAME: &'static str = "light_vertex";
    type Binds<'c> = light_vertex_binds<'c>;
    type Type = VertexFunctionType;
    type FunctionConstantsType = FunctionConstants;
}

#[allow(non_camel_case_types)]
pub struct light_fragment;
impl metal_app::render_pipeline::Function for light_fragment {
    const FUNCTION_NAME: &'static str = "light_fragment";
    type Binds<'c> = NoBinds;
    type Type = FragmentFunctionType;
    type FunctionConstantsType = FunctionConstants;
}
