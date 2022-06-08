use crate::{unwrap_option_dcheck, unwrap_result_dcheck};
use foreign_types::ForeignType;
use metal::*;
use objc::runtime::Object;
use std::ffi::{c_void, CStr};

pub const DEFAULT_RESOURCE_OPTIONS: MTLResourceOptions = MTLResourceOptions::from_bits_truncate(
    MTLResourceOptions::StorageModeShared.bits()
        | MTLResourceOptions::CPUCacheModeWriteCombined.bits(),
);

#[inline(always)]
pub const fn align_size(MTLSizeAndAlign { size, align }: MTLSizeAndAlign) -> usize {
    (size + (align - (size & (align - 1)))) as _
}

#[inline(always)]
pub fn copy_into_buffer<T: Sized>(src: &[T], dst: *mut T, offset: usize) -> usize {
    unsafe {
        let count = src.len();
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst.byte_add(offset), count);
        offset + std::mem::size_of::<T>() * count
    }
}

#[inline(always)]
pub const fn byte_size_of_slice<T: Sized>(slice: &[T]) -> usize {
    slice.len() * std::mem::size_of::<T>()
}

#[inline]
pub fn allocate_new_buffer_with_heap<T: Sized>(
    heap: &Heap,
    label: &'static str,
    bytes: usize,
) -> (*mut T, Buffer) {
    let buf = heap
        .new_buffer(bytes as u64, DEFAULT_RESOURCE_OPTIONS)
        .expect(&format!("Failed to allocate buffer for {label}"));
    buf.set_label(label);
    (buf.contents() as *mut T, buf)
}

#[inline]
pub fn allocate_new_buffer<T: Sized>(
    device: &DeviceRef,
    label: &'static str,
    bytes: usize,
) -> (*mut T, Buffer) {
    let buf = device.new_buffer(bytes as u64, DEFAULT_RESOURCE_OPTIONS);
    buf.set_label(label);
    (buf.contents() as *mut T, buf)
}

#[inline]
pub fn allocate_new_buffer_with_data<T: Sized>(
    device: &DeviceRef,
    label: &'static str,
    data: &[T],
) -> Buffer {
    let (contents, buffer) =
        allocate_new_buffer::<T>(&device, label, std::mem::size_of::<T>() * data.len());
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), contents, data.len());
    }
    buffer
}

#[inline]
pub fn encode_vertex_bytes<T: Sized + Copy + Clone>(
    encoder: &RenderCommandEncoderRef,
    buffer_index: u64,
    v: &T,
) {
    let ptr: *const T = v;
    encoder.set_vertex_bytes(
        buffer_index,
        std::mem::size_of::<T>() as _,
        ptr as *const c_void,
    );
}

#[inline]
pub fn encode_fragment_bytes<T: Sized + Copy + Clone>(
    encoder: &RenderCommandEncoderRef,
    buffer_index: u64,
    v: &T,
) {
    let ptr: *const T = v;
    encoder.set_fragment_bytes(
        buffer_index,
        std::mem::size_of::<T>() as _,
        ptr as *const c_void,
    );
}

// TODO: Consider upstreaming change to metal-rs
// - This avoids needing to FunctionConstantValues.clone() (under the covers, calls Obj-C `retain()`).
pub fn new_function_from_library(
    library: &Library,
    name: &str,
    constants: Option<&FunctionConstantValues>,
) -> Result<Function, String> {
    macro_rules! try_objc {
        {
            $err_name: ident => $body:expr
        } => {
            {
                let mut $err_name: *mut ::objc::runtime::Object = ::std::ptr::null_mut();
                let value = $body;
                if !$err_name.is_null() {
                    let desc: *mut Object = msg_send![$err_name, localizedDescription];
                    let compile_error: *const std::os::raw::c_char = msg_send![desc, UTF8String];
                    let message = CStr::from_ptr(compile_error).to_string_lossy().into_owned();
                    let () = msg_send![$err_name, release];
                    return Err(message);
                }
                value
            }
        };
    }

    fn nsstring_from_str(string: &str) -> *mut objc::runtime::Object {
        const UTF8_ENCODING: usize = 4;

        let cls = class!(NSString);
        let bytes = string.as_ptr() as *const c_void;
        unsafe {
            let obj: *mut objc::runtime::Object = msg_send![cls, alloc];
            let obj: *mut objc::runtime::Object = msg_send![
                obj,
                initWithBytes:bytes
                length:string.len()
                encoding:UTF8_ENCODING
            ];
            let _: *mut c_void = msg_send![obj, autorelease];
            obj
        }
    }

    unsafe {
        let nsname = nsstring_from_str(name);

        let function: *mut MTLFunction = match constants {
            Some(c) => try_objc! { err => msg_send![library.as_ref(),
                newFunctionWithName: nsname.as_ref()
                constantValues: c.as_ref()
                error: &mut err
            ]},
            None => msg_send![library.as_ref(), newFunctionWithName: nsname.as_ref()],
        };

        if !function.is_null() {
            Ok(Function::from_ptr(function))
        } else {
            Err(format!("Function '{}' does not exist", name))
        }
    }
}

pub struct CreateRenderPipelineResults {
    pub vertex_function: Function,
    pub fragment_function: Function,
    pub pipeline_state: RenderPipelineState,
}

pub fn create_pipeline(
    device: &Device,
    library: &Library,
    base_pipeline_desc: &RenderPipelineDescriptor,
    label: &str,
    func_constants: Option<&FunctionConstantValues>,
    vertex_func_name: &str,
    num_vertex_immutable_buffers: u32,
    frag_func_name: &str,
    num_frag_immutable_buffers: u32,
) -> CreateRenderPipelineResults {
    base_pipeline_desc.set_label(label);

    let fcs = func_constants;
    let vertex_function = unwrap_result_dcheck(
        new_function_from_library(library, vertex_func_name, fcs),
        "Failed to access vertex shader function from metal library",
    );
    base_pipeline_desc.set_vertex_function(Some(&vertex_function));

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

    let fragment_function = unwrap_result_dcheck(
        new_function_from_library(library, frag_func_name, fcs),
        "Failed to access fragment shader function from metal library",
    );
    base_pipeline_desc.set_fragment_function(Some(&fragment_function));

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

    let pipeline_state = unwrap_result_dcheck(
        device.new_render_pipeline_state(&base_pipeline_desc),
        "Failed to create render pipeline",
    );
    CreateRenderPipelineResults {
        vertex_function,
        fragment_function,
        pipeline_state,
    }
}

// TODO: Investigate when this improves performance.
// - In quick performance profiling of proj-4, no performance improvements were observed
//   - Methodology
//      - Maximize Window
//      - Frame Capture
//      - Profiling GPU in "Maximum" performance state
//      - Navigate to Fragment Shader (source) that includes line-by-line time percentages in right gutter
//      - Observe texture sample lines on contribute to ~7% (ambient) and ~1.8% (specular),
//        regardless whether optimize_textures_for_gpu_access was used or not.
// - Guess: Only improves non-Apple Silicon CPU/GPU
pub fn optimize_textures_for_gpu_access(textures: &[&Texture], command_queue: &CommandQueue) {
    let command_buf = command_queue.new_command_buffer();
    let enc = command_buf.new_blit_command_encoder();
    for &texture in textures {
        enc.optimize_contents_for_gpu_access(texture);
    }
    enc.end_encoding();
    command_buf.commit();
    command_buf.wait_until_completed();
}
