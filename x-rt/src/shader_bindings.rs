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
pub struct main_vertex;
impl metal_app::pipeline::function::Function for main_vertex {
    const FUNCTION_NAME: &'static str = "main_vertex";
    type Binds<'c> = NoBinds;
}
impl PipelineFunction<VertexFunctionType> for main_vertex {}

#[allow(non_camel_case_types)]
pub struct main_fragment_binds<'c> {
    pub accelerationStructure: BindAccelerationStructure<'c>,
    pub camera: Bind<'c, ProjectedSpace>,
    pub m_normal_to_worlds: BindMany<'c, half3x3>,
}
impl Binds for main_fragment_binds<'_> {
    const SKIP: Self = Self {
        accelerationStructure: BindAccelerationStructure::Skip,
        camera: Bind::Skip,
        m_normal_to_worlds: BindMany::Skip,
    };

    #[inline(always)]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder) {
        self.accelerationStructure.bind::<F>(encoder, 0);
        self.camera.bind::<F>(encoder, 1);
        self.m_normal_to_worlds.bind::<F>(encoder, 2);
    }
}

#[allow(non_camel_case_types)]
pub struct main_fragment;
impl metal_app::pipeline::function::Function for main_fragment {
    const FUNCTION_NAME: &'static str = "main_fragment";
    type Binds<'c> = main_fragment_binds<'c>;
}
impl PipelineFunction<FragmentFunctionType> for main_fragment {}
