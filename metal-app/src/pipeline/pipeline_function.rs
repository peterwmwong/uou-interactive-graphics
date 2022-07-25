use super::{bind::Binds, function::Function};
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

pub trait PipelineFunction<F: PipelineFunctionType>: Function {
    #[inline(always)]
    fn setup_pipeline(&self, library: &LibraryRef, pipeline_desc: &F::Descriptor) {
        F::setup_pipeline(&self.get_function(library), pipeline_desc);
    }

    #[inline(always)]
    fn bind<'a, 'b>(encoder: &'a F::CommandEncoder, binds: Self::Binds<'b>) {
        binds.bind::<F>(encoder);
    }
}
