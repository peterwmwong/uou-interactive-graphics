#![allow(non_upper_case_globals, non_snake_case)]
/**************************************************************************************************
 GENERATED FILE. DO NOT MODIFY.

 This file is generated by the `metal_build` crate, check you're `build.rs` for
 `metal_build::build()`.
 Structs and Enums are generated based on `shader_src/shader_bindings.h` and `shader_src/shaders.metal`.
***************************************************************************************************/
#[allow(unused_imports)]
use metal_app::{metal::*, metal_types::*, pipeline::*};

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
    pub light: Bind<'c, ProjectedSpace>,
    pub material: Bind<'c, Material>,
    pub shadow_tx: BindTexture<'c>,
}
impl Binds for main_fragment_binds<'_> {
    const SKIP: Self = Self {
        camera: Bind::Skip,
        light: Bind::Skip,
        material: Bind::Skip,
        shadow_tx: BindTexture::Skip,
    };

    #[inline(always)]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder) {
        self.camera.bind::<F>(encoder, 0);
        self.light.bind::<F>(encoder, 1);
        self.material.bind::<F>(encoder, 2);
        self.shadow_tx.bind::<F>(encoder, 0);
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
