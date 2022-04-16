use metal::*;
use std::ffi::c_void;

#[allow(dead_code)]
#[inline]
pub(crate) fn allocate_new_buffer(
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
