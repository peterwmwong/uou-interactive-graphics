use crate::{unwrap_option_dcheck, unwrap_result_dcheck};
use metal::*;
use std::ffi::c_void;

#[inline]
pub fn allocate_new_buffer(
    device: &DeviceRef,
    label: &'static str,
    bytes: usize,
) -> (*mut c_void, Buffer) {
    let buf = device.new_buffer(
        bytes as u64,
        MTLResourceOptions::CPUCacheModeWriteCombined | MTLResourceOptions::StorageModeShared,
    );
    buf.set_label(label);
    (buf.contents(), buf)
}

#[inline]
pub fn allocate_new_buffer_with_data<T: Sized>(
    device: &DeviceRef,
    label: &'static str,
    data: &[T],
) -> Buffer {
    let (contents, buffer) =
        allocate_new_buffer(&device, label, std::mem::size_of::<T>() * data.len());
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), contents as *mut T, data.len());
    }
    buffer
}

#[inline]
pub fn encode_vertex_bytes<I: Into<u64>, T: Sized + Copy + Clone>(
    encoder: &RenderCommandEncoderRef,
    buffer_index: I,
    v: &T,
) {
    let ptr: *const T = v;
    encoder.set_vertex_bytes(
        buffer_index.into(),
        std::mem::size_of::<T>() as _,
        ptr as *const c_void,
    );
}

#[inline]
pub fn encode_fragment_bytes<I: Into<u64>, T: Sized + Copy + Clone>(
    encoder: &RenderCommandEncoderRef,
    buffer_index: I,
    v: &T,
) {
    let ptr: *const T = v;
    encoder.set_fragment_bytes(
        buffer_index.into(),
        std::mem::size_of::<T>() as _,
        ptr as *const c_void,
    );
}

pub fn create_pipeline(
    device: &Device,
    library: &Library,
    base_pipeline_desc: &RenderPipelineDescriptor,
    label: &str,
    vertex_func_name: &str,
    num_vertex_immutable_buffers: u32,
    frag_func_name: &str,
    num_frag_immutable_buffers: u32,
) -> RenderPipelineState {
    base_pipeline_desc.set_label(label);

    let fun = unwrap_result_dcheck(
        library.get_function(vertex_func_name, None),
        "Failed to access vertex shader function from metal library",
    );
    base_pipeline_desc.set_vertex_function(Some(&fun));

    let buffers = base_pipeline_desc
        .vertex_buffers()
        .expect("Failed to access vertex buffers");
    for buffer_index in 0..num_vertex_immutable_buffers {
        unwrap_option_dcheck(
            buffers.object_at(buffer_index as _),
            "Failed to access vertex buffer",
        )
        .set_mutability(MTLMutability::Immutable);
    }

    let fun = unwrap_result_dcheck(
        library.get_function(frag_func_name, None),
        "Failed to access fragment shader function from metal library",
    );
    base_pipeline_desc.set_fragment_function(Some(&fun));

    let buffers = base_pipeline_desc
        .fragment_buffers()
        .expect("Failed to access fragment buffers");
    for buffer_index in 0..num_frag_immutable_buffers {
        unwrap_option_dcheck(
            buffers.object_at(buffer_index as _),
            "Failed to access fragment buffer",
        )
        .set_mutability(MTLMutability::Immutable);
    }

    unwrap_result_dcheck(
        device.new_render_pipeline_state(&base_pipeline_desc),
        "Failed to create render pipeline",
    )
}
