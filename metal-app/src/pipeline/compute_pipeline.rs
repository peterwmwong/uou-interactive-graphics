use super::pipeline_function::PipelineFunctionType;
use crate::typed_buffer::TypedBuffer;
use metal::{ComputeCommandEncoderRef, ComputePipelineDescriptorRef, FunctionRef, TextureRef};

pub struct ComputeFunctionType;
impl PipelineFunctionType for ComputeFunctionType {
    type Descriptor = ComputePipelineDescriptorRef;
    type CommandEncoder = ComputeCommandEncoderRef;

    #[inline(always)]
    fn setup_pipeline(func: &FunctionRef, pipeline_desc: &Self::Descriptor) {
        pipeline_desc.set_compute_function(Some(func));
    }

    #[inline(always)]
    fn bytes<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        value: &'b [T],
    ) {
        encoder.set_bytes(
            index as _,
            std::mem::size_of_val(value) as _,
            value.as_ptr() as *const _,
        )
    }
    #[inline(always)]
    fn buffer_and_offset<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        (buffer, offset): (&'b TypedBuffer<T>, usize),
    ) {
        encoder.set_buffer(
            index as _,
            Some(&buffer.buffer),
            (std::mem::size_of::<T>() * offset) as _,
        );
    }
    #[inline(always)]
    fn buffer_offset<'a, 'b, T: Sized + Copy + Clone>(
        _encoder: &'a Self::CommandEncoder,
        _index: usize,
        _offset: usize,
    ) {
        // TODO: metal-rs forgot to implement set_buffer_offset!
        todo!();
        // encoder.set_buffer_offset(index as _, (std::mem::size_of::<T>() * offset) as _);
    }
    #[inline(always)]
    fn texture<'a, 'b>(encoder: &'a Self::CommandEncoder, index: usize, texture: &'b TextureRef) {
        encoder.set_texture(index as _, Some(texture));
    }

    #[inline(always)]
    fn acceleration_structure<'a, 'b>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        accel_struct: &'b metal::AccelerationStructureRef,
    ) {
        encoder.set_acceleration_structure(Some(accel_struct), index as _);
    }
}
