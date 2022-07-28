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

#[repr(u8)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum TextureFilterMode {
    Nearest = 0,
    Linear = 1,
    Mipmap = 2,
    Anistropic = 3,
}

/****************
 Shader functions
*****************/

#[allow(non_camel_case_types)]
pub struct checkerboard_vertex;
impl metal_app::pipeline::function::Function for checkerboard_vertex {
    const FUNCTION_NAME: &'static str = "checkerboard_vertex";
    type Binds<'c> = NoBinds;
}
impl PipelineFunction<VertexFunctionType> for checkerboard_vertex {}

#[allow(non_camel_case_types)]
pub struct checkerboard_fragment;
impl metal_app::pipeline::function::Function for checkerboard_fragment {
    const FUNCTION_NAME: &'static str = "checkerboard_fragment";
    type Binds<'c> = NoBinds;
}
impl PipelineFunction<FragmentFunctionType> for checkerboard_fragment {}

#[allow(non_camel_case_types)]
pub struct main_vertex_binds<'c> {
    pub m_model_to_projection: Bind<'c, float4x4>,
}
impl Binds for main_vertex_binds<'_> {
    const SKIP: Self = Self {
        m_model_to_projection: Bind::Skip,
    };

    #[inline(always)]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder) {
        self.m_model_to_projection.bind::<F>(encoder, 0);
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
    pub texture: BindTexture<'c>,
    pub mode: Bind<'c, TextureFilterMode>,
}
impl Binds for main_fragment_binds<'_> {
    const SKIP: Self = Self {
        texture: BindTexture::Skip,
        mode: Bind::Skip,
    };

    #[inline(always)]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder) {
        self.texture.bind::<F>(encoder, 0);
        self.mode.bind::<F>(encoder, 0);
    }
}

#[allow(non_camel_case_types)]
pub struct main_fragment;
impl metal_app::pipeline::function::Function for main_fragment {
    const FUNCTION_NAME: &'static str = "main_fragment";
    type Binds<'c> = main_fragment_binds<'c>;
}
impl PipelineFunction<FragmentFunctionType> for main_fragment {}
