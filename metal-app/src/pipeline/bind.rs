use super::pipeline_function::PipelineFunctionType;
use crate::typed_buffer::TypedBuffer;
use metal::TextureRef;

pub trait Binds {
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder);
}

pub trait AnyBind<T: Sized + Copy + Clone> {
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize);
}

pub enum BindBuffer<'a, T: Sized + Copy + Clone> {
    WithOffset(&'a TypedBuffer<T>, usize),
    Offset(usize),
}
impl<'a, T: Sized + Copy + Clone> AnyBind<T> for BindBuffer<'a, T> {
    #[inline]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        match self {
            BindBuffer::WithOffset(buf, offset) => {
                F::buffer_and_offset(encoder, index, (buf, offset))
            }
            BindBuffer::Offset(offset) => F::buffer_offset::<T>(encoder, index, offset),
        }
    }
}

pub enum Bind<'a, T: Sized + Copy + Clone> {
    Value(&'a T),
    Buffer(BindBuffer<'a, T>),
    Skip,
}
impl<'a, T: Sized + Copy + Clone> AnyBind<T> for Bind<'a, T> {
    #[inline]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        match self {
            Bind::Value(&v) => F::bytes(encoder, index, &[v]),
            Bind::Buffer(bind_buf) => bind_buf.bind::<F>(encoder, index),
            _ => {}
        }
    }
}
impl<'a, T: Sized + Copy + Clone> Bind<'a, T> {
    #[inline]
    pub fn buffer_with_rolling_offset(
        (buffer, element_offset): (&'a TypedBuffer<T>, usize),
    ) -> Self {
        if element_offset == 0 {
            Self::Buffer(BindBuffer::WithOffset(buffer, 0))
        } else {
            Self::Buffer(BindBuffer::Offset(element_offset))
        }
    }

    #[inline]
    pub fn iterating_buffer_offset(
        iteration: usize,
        (buffer, element_offset): (&'a TypedBuffer<T>, usize),
    ) -> Self {
        if iteration == 0 {
            Self::Buffer(BindBuffer::WithOffset(buffer, element_offset))
        } else {
            Self::Buffer(BindBuffer::Offset(element_offset))
        }
    }
}

pub enum BindMany<'a, T: Sized + Copy + Clone> {
    Values(&'a [T]),
    Buffer(BindBuffer<'a, T>),
    Skip,
}
impl<'a, T: Sized + Copy + Clone> AnyBind<T> for BindMany<'a, T> {
    #[inline]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        match self {
            BindMany::Values(v) => F::bytes(encoder, index, v),
            BindMany::Buffer(bind_buf) => bind_buf.bind::<F>(encoder, index),
            _ => {}
        }
    }
}
impl<'a, T: Sized + Copy + Clone> BindMany<'a, T> {
    #[inline]
    pub fn buffer_with_rolling_offset(
        (buffer, element_offset): (&'a TypedBuffer<T>, usize),
    ) -> Self {
        if element_offset == 0 {
            Self::Buffer(BindBuffer::WithOffset(buffer, 0))
        } else {
            Self::Buffer(BindBuffer::Offset(element_offset))
        }
    }

    #[inline]
    pub fn iterating_buffer_offset(
        iteration: usize,
        (buffer, element_offset): (&'a TypedBuffer<T>, usize),
    ) -> Self {
        if iteration == 0 {
            Self::Buffer(BindBuffer::WithOffset(buffer, element_offset))
        } else {
            Self::Buffer(BindBuffer::Offset(element_offset))
        }
    }
}

pub enum BindTexture<'a> {
    Texture(&'a TextureRef),
    Skip,
}
impl<'a> BindTexture<'a> {
    #[inline]
    pub fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        match self {
            BindTexture::Texture(texture) => F::texture(encoder, index, texture),
            _ => {}
        }
    }
}
