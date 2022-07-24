use super::pipeline_function::PipelineFunctionType;
use crate::typed_buffer::TypedBuffer;
use metal::TextureRef;
use std::marker::PhantomData;

pub struct BindTexture<'a>(pub &'a TextureRef);
impl<'a> BindTexture<'a> {
    pub fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        F::texture(encoder, index, self.0);
    }
}

pub trait Bind<T: Sized + Copy + Clone> {
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize);
}

// Marker trait identifying Bind implementations that applicable for singular (by reference) Metal
// function binds. For example, a Metal function binding considered singular and be marked with
// BindOne...
//
//      [[vertex]]
//      float4 main(
//          constant float4 & a_float4 [[buffer(0)]]
//                       // ^--- by reference / singular
//      ) { ... }
//
pub trait BindOne<T: Sized + Copy + Clone>: Bind<T> {}

// Marker trait identifying Bind implementations that applicable for multiple (by pointer) Metal
// function binds. For example, a Metal function binding considered multiple and be marked with
// BindMany...
//
//      [[vertex]]
//      float4 main(
//          constant float4 * many_float4s [[buffer(0)]]
//                       // ^--- by pointer / multiple
//      ) { ... }
//
pub trait BindMany<T: Sized + Copy + Clone>: Bind<T> {}

pub struct BindBytes<'a, T: Sized + Copy + Clone>(pub &'a T);
impl<T: Sized + Copy + Clone> Bind<T> for BindBytes<'_, T> {
    #[inline(always)]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        F::bytes(encoder, index, &[*self.0])
    }
}
impl<T: Sized + Copy + Clone> BindOne<T> for BindBytes<'_, T> {}

pub struct BindBytesMany<'a, T: Sized + Copy + Clone>(pub &'a [T]);
impl<T: Sized + Copy + Clone> Bind<T> for BindBytesMany<'_, T> {
    #[inline(always)]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        F::bytes(encoder, index, self.0)
    }
}
impl<T: Sized + Copy + Clone> BindMany<T> for BindBytesMany<'_, T> {}

pub struct BindBufferAndOffset<'a, T: Sized + Copy + Clone>(pub &'a TypedBuffer<T>, pub usize);
impl<T: Sized + Copy + Clone> Bind<T> for BindBufferAndOffset<'_, T> {
    #[inline(always)]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        F::buffer_and_offset(encoder, index, (self.0, self.1 as _))
    }
}
impl<'a, T: Sized + Copy + Clone> BindMany<T> for BindBufferAndOffset<'a, T> {}
impl<'a, T: Sized + Copy + Clone> BindOne<T> for BindBufferAndOffset<'a, T> {}

pub struct BindBufferOffsetType<T: Sized + Copy + Clone>(usize, PhantomData<T>);
impl<T: Sized + Copy + Clone> Bind<T> for BindBufferOffsetType<T> {
    #[inline(always)]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        F::buffer_offset::<T>(encoder, index, self.0)
    }
}
impl<T: Sized + Copy + Clone> BindMany<T> for BindBufferOffsetType<T> {}
impl<T: Sized + Copy + Clone> BindOne<T> for BindBufferOffsetType<T> {}
#[allow(non_snake_case)]
pub fn BindBufferOffset<T: Sized + Copy + Clone>(offset: usize) -> BindBufferOffsetType<T> {
    BindBufferOffsetType(offset, PhantomData)
}
