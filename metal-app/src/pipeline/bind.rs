use super::pipeline_function::PipelineFunctionType;
use crate::typed_buffer::TypedBuffer;
use metal::{AccelerationStructureRef, TextureRef};

/*
TODO: Consider optimizing Binding API for consistent Bind Variant usage

Example:
    main_vertex_binds {
        camera: Bind::Value(&self.camera_space)
        ...
    }
    ...
    main_vertex_binds {
        geometry: Bind::buffer_with_rolling_offset(geometry),
        ..Binds::SKIP
    }

`camera` will always be Bind::Value or Skip, while `geometry` will always be Bind::Buffer or Skip.
I can't think of a reason why you'd be switching between other variants (ex. Value, then Buffer?).
*/

pub trait Binds {
    const SKIP: Self;
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder);
}

pub trait AnyBind<T: Sized + Copy + Clone> {
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize);
}

#[derive(Copy, Clone)]
pub enum BindBuffer<'a, T: Sized + Copy + Clone> {
    WithOffset(&'a TypedBuffer<T>, usize),
    Offset(usize),
}
impl<'a, T: Sized + Copy + Clone> AnyBind<T> for BindBuffer<'a, T> {
    #[inline]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        use BindBuffer::*;
        match self {
            WithOffset(buf, offset) => F::buffer_and_offset(encoder, index, (buf, offset)),
            Offset(offset) => F::buffer_offset::<T>(encoder, index, offset),
        }
    }
}

#[derive(Copy, Clone)]
pub enum Bind<'a, T: Sized + Copy + Clone> {
    Value(&'a T),
    Buffer(BindBuffer<'a, T>),
    Skip,
}
impl<'a, T: Sized + Copy + Clone> AnyBind<T> for Bind<'a, T> {
    #[inline]
    fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        use Bind::*;
        match self {
            Value(&v) => F::bytes(encoder, index, &[v]),
            Buffer(bind_buf) => bind_buf.bind::<F>(encoder, index),
            Skip => {}
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
        use BindMany::*;
        match self {
            Values(v) => F::bytes(encoder, index, v),
            Buffer(bind_buf) => bind_buf.bind::<F>(encoder, index),
            Skip => {}
        }
    }
}

macro_rules! impl_bind_buffer_helpers {
    ($bind_ident:ident) => {
        impl<'a, T: Sized + Copy + Clone> $bind_ident<'a, T> {
            #[inline(always)]
            pub fn buffer(buffer: &'a TypedBuffer<T>) -> Self {
                Self::buffer_and_offset(buffer, 0)
            }

            #[inline(always)]
            pub fn buffer_and_offset(buffer: &'a TypedBuffer<T>, offset: usize) -> Self {
                Self::Buffer(BindBuffer::WithOffset(buffer, offset))
            }

            #[inline(always)]
            pub fn buffer_with_rolling_offset(
                (buffer, element_offset): (&'a TypedBuffer<T>, usize),
            ) -> Self {
                if element_offset == 0 {
                    Self::Buffer(BindBuffer::WithOffset(buffer, 0))
                } else {
                    Self::Buffer(BindBuffer::Offset(element_offset))
                }
            }

            #[inline(always)]
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
    };
}
impl_bind_buffer_helpers!(Bind);
impl_bind_buffer_helpers!(BindMany);

pub enum BindTexture<'a> {
    Texture(&'a TextureRef),
    Null,
    Skip,
}
#[allow(non_snake_case)]
pub fn BindTexture<'a>(texture: &'a TextureRef) -> BindTexture<'a> {
    BindTexture::Texture(texture)
}

impl<'a> BindTexture<'a> {
    #[inline]
    pub fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        use BindTexture::*;
        match self {
            Texture(texture) => F::texture(encoder, index, texture),
            Null => F::texture_null(encoder, index),
            Skip => {}
        }
    }
}

pub enum BindAccelerationStructure<'a> {
    AccelerationStructure(&'a AccelerationStructureRef),
    Null,
    Skip,
}

#[allow(non_snake_case)]
pub fn BindAccelerationStructure<'a>(
    accel: &'a AccelerationStructureRef,
) -> BindAccelerationStructure<'a> {
    BindAccelerationStructure::AccelerationStructure(accel)
}

impl<'a> BindAccelerationStructure<'a> {
    #[inline]
    pub fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder, index: usize) {
        use BindAccelerationStructure::*;
        match self {
            AccelerationStructure(accel_struct) => {
                F::acceleration_structure(encoder, index, accel_struct)
            }
            Null => F::acceleration_structure_null(encoder, index),
            Skip => {}
        }
    }
}
