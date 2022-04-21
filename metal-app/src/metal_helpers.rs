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
pub fn encode_vertex_bytes<I: Into<u64>, T: Sized + Copy + Clone>(
    encoder: &RenderCommandEncoderRef,
    buffer_index: I,
    v: &T,
) {
    let max_value_ptr: *const T = v;
    encoder.set_vertex_bytes(
        buffer_index.into(),
        std::mem::size_of::<T>() as _,
        max_value_ptr as *const c_void,
    );
}
