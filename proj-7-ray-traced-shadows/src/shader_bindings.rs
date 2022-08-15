#![allow(non_upper_case_globals, non_snake_case)]
/**************************************************************************************************
 GENERATED FILE. DO NOT MODIFY.

 This file is generated by the `metal_build` crate, check you're `build.rs` for
 `metal_build::build()`.
 Structs and Enums are generated based on `shader_src/shader_bindings.h` and `shader_src/shaders.metal`.
***************************************************************************************************/
#[allow(unused_imports)]
use metal_app::{metal::*, metal_types::*, pipeline::*};
/* automatically generated by rust-bindgen 0.60.1 */

#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
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
#[derive(Default, Copy, Clone, PartialEq)]
pub struct Material {
    pub ambient_texture: ::std::os::raw::c_ulong,
    pub diffuse_texture: ::std::os::raw::c_ulong,
    pub specular_texture: ::std::os::raw::c_ulong,
    pub specular_shineness: f32,
    pub ambient_amount: f32,
}
#[test]
fn bindgen_test_layout_Material() {
    assert_eq!(
        ::std::mem::size_of::<Material>(),
        32usize,
        concat!("Size of: ", stringify!(Material))
    );
    assert_eq!(
        ::std::mem::align_of::<Material>(),
        8usize,
        concat!("Alignment of ", stringify!(Material))
    );
    fn test_field_ambient_texture() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Material>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).ambient_texture) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(Material),
                "::",
                stringify!(ambient_texture)
            )
        );
    }
    test_field_ambient_texture();
    fn test_field_diffuse_texture() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Material>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).diffuse_texture) as usize - ptr as usize
            },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(Material),
                "::",
                stringify!(diffuse_texture)
            )
        );
    }
    test_field_diffuse_texture();
    fn test_field_specular_texture() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Material>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).specular_texture) as usize - ptr as usize
            },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(Material),
                "::",
                stringify!(specular_texture)
            )
        );
    }
    test_field_specular_texture();
    fn test_field_specular_shineness() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Material>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).specular_shineness) as usize - ptr as usize
            },
            24usize,
            concat!(
                "Offset of field: ",
                stringify!(Material),
                "::",
                stringify!(specular_shineness)
            )
        );
    }
    test_field_specular_shineness();
    fn test_field_ambient_amount() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Material>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).ambient_amount) as usize - ptr as usize
            },
            28usize,
            concat!(
                "Offset of field: ",
                stringify!(Material),
                "::",
                stringify!(ambient_amount)
            )
        );
    }
    test_field_ambient_amount();
}
#[repr(C)]
#[repr(align(16))]
#[derive(Copy, Clone)]
pub struct ModelSpace {
    pub m_model_to_projection: float4x4,
    pub m_normal_to_world: float3x3,
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
    fn test_field_m_model_to_projection() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ModelSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).m_model_to_projection) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(ModelSpace),
                "::",
                stringify!(m_model_to_projection)
            )
        );
    }
    test_field_m_model_to_projection();
    fn test_field_m_normal_to_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ModelSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).m_normal_to_world) as usize - ptr as usize
            },
            64usize,
            concat!(
                "Offset of field: ",
                stringify!(ModelSpace),
                "::",
                stringify!(m_normal_to_world)
            )
        );
    }
    test_field_m_normal_to_world();
}
impl Default for ModelSpace {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
#[repr(C)]
#[repr(align(16))]
#[derive(Copy, Clone)]
pub struct ProjectedSpace {
    pub m_world_to_projection: float4x4,
    pub m_screen_to_world: float4x4,
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
    fn test_field_m_world_to_projection() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ProjectedSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).m_world_to_projection) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(ProjectedSpace),
                "::",
                stringify!(m_world_to_projection)
            )
        );
    }
    test_field_m_world_to_projection();
    fn test_field_m_screen_to_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ProjectedSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).m_screen_to_world) as usize - ptr as usize
            },
            64usize,
            concat!(
                "Offset of field: ",
                stringify!(ProjectedSpace),
                "::",
                stringify!(m_screen_to_world)
            )
        );
    }
    test_field_m_screen_to_world();
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
impl Default for ProjectedSpace {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}

/****************
 Shader functions
*****************/

#[allow(non_camel_case_types)]
pub struct main_vertex_binds<'c> {
    pub model: Bind<'c, ModelSpace>,
    pub geometry: Bind<'c, Geometry>,
}
impl Binds for main_vertex_binds<'_> {
    const SKIP: Self = Self {
        model: Bind::Skip,
        geometry: Bind::Skip,
    };

    #[inline(always)]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder) {
        self.model.bind::<F>(encoder, 0);
        self.geometry.bind::<F>(encoder, 1);
    }
}

#[allow(non_camel_case_types)]
pub struct main_vertex;
impl metal_app::pipeline::function::Function for main_vertex {
    const FUNCTION_NAME: &'static str = "main_vertex";
    type Binds<'c> = main_vertex_binds<'c>;
}
impl PipelineFunction<VertexFunctionType> for main_vertex {}

#[allow(non_camel_case_types)]
pub struct main_fragment_binds<'c> {
    pub camera: Bind<'c, ProjectedSpace>,
    pub light_pos: Bind<'c, float4>,
    pub material: Bind<'c, Material>,
    pub accel_struct: BindAccelerationStructure<'c>,
}
impl Binds for main_fragment_binds<'_> {
    const SKIP: Self = Self {
        camera: Bind::Skip,
        light_pos: Bind::Skip,
        material: Bind::Skip,
        accel_struct: BindAccelerationStructure::Skip,
    };

    #[inline(always)]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder) {
        self.camera.bind::<F>(encoder, 0);
        self.light_pos.bind::<F>(encoder, 1);
        self.material.bind::<F>(encoder, 2);
        self.accel_struct.bind::<F>(encoder, 3);
    }
}

#[allow(non_camel_case_types)]
pub struct main_fragment {
    pub HasAmbient: bool,
    pub HasDiffuse: bool,
    pub OnlyNormals: bool,
    pub HasSpecular: bool,
}
impl metal_app::pipeline::function::Function for main_fragment {
    const FUNCTION_NAME: &'static str = "main_fragment";
    type Binds<'c> = main_fragment_binds<'c>;
    #[inline]
    fn get_function_constants(&self) -> Option<FunctionConstantValues> {
        let fcv = FunctionConstantValues::new();
        fcv.set_constant_value_at_index((&self.HasAmbient as *const _) as _, bool::MTL_DATA_TYPE, 0);
        fcv.set_constant_value_at_index((&self.HasDiffuse as *const _) as _, bool::MTL_DATA_TYPE, 1);
        fcv.set_constant_value_at_index((&self.OnlyNormals as *const _) as _, bool::MTL_DATA_TYPE, 2);
        fcv.set_constant_value_at_index((&self.HasSpecular as *const _) as _, bool::MTL_DATA_TYPE, 3);
        Some(fcv)
    }
}
impl PipelineFunction<FragmentFunctionType> for main_fragment {}