use super::function::Function;
use crate::typed_buffer::TypedBuffer;
use metal::{FunctionRef, LibraryRef, TextureRef};

pub trait PipelineFunctionType {
    type Descriptor;
    type CommandEncoder;

    fn setup_pipeline(func: &FunctionRef, pipeline_desc: &Self::Descriptor);

    fn bytes<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        value: &'b [T],
    );
    fn buffer_and_offset<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        buffer_offset: (&'b TypedBuffer<T>, usize),
    );
    fn buffer_offset<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        offset: usize,
    );
    fn texture<'a, 'b>(encoder: &'a Self::CommandEncoder, index: usize, texture: &'b TextureRef);
}

pub trait FunctionBinder<'a, T: PipelineFunctionType> {
    fn new(encoder: &'a T::CommandEncoder) -> Self;
}

pub trait PipelineFunction<F: PipelineFunctionType>: Function {
    type Binder<'a>: FunctionBinder<'a, F>;
    fn setup_pipeline(&self, library: &LibraryRef, pipeline_desc: &F::Descriptor) {
        F::setup_pipeline(&self.get_function(library), pipeline_desc);
    }
}
