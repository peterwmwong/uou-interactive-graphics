use crate::{unwrap_option_dcheck, unwrap_result_dcheck};
use foreign_types::ForeignType;
use metal::*;
use objc::runtime::{Object, Sel};
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
pub fn copy_into_buffer<T: Sized>(src: &[T], dst: *mut T, byte_offset: usize) -> usize {
    unsafe {
        let count = src.len();
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst.byte_add(byte_offset), count);
        byte_offset + std::mem::size_of::<T>() * count
    }
}

#[inline]
pub fn debug_assert_buffers_equal(a: &Buffer, b: &Buffer) {
    #[cfg(debug_assertions)]
    {
        let a_length = a.length();
        let b_length = b.length();
        debug_assert_eq!(a_length, b_length, "Buffer lengths are not equal");

        let a_contents = a.contents() as *const u8;
        let b_contents = b.contents() as *const u8;
        for i in 0..a_length {
            unsafe {
                let a_val = *(a_contents.add(i as _));
                let b_val = *(b_contents.add(i as _));
                debug_assert_eq!(a_val, b_val, "Byte {i} is not equal.");
            }
        }
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
#[inline]
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

pub const DEFAULT_PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::BGRA8Unorm;

#[inline]
pub fn new_basic_render_pipeline_descriptor(
    pixel_format: MTLPixelFormat,
    depth_pixel_format: Option<MTLPixelFormat>,
    blending: bool,
) -> RenderPipelineDescriptor {
    let base_pipeline_desc = RenderPipelineDescriptor::new();
    let desc = base_pipeline_desc
        .color_attachments()
        .object_at(0 as u64)
        .expect("Failed to access color attachment on pipeline descriptor");
    desc.set_blending_enabled(blending);
    desc.set_pixel_format(pixel_format);
    if let Some(depth_pixel_format) = depth_pixel_format {
        base_pipeline_desc.set_depth_attachment_pixel_format(depth_pixel_format);
    }
    base_pipeline_desc
}

// TODO: START HERE 2
// TODO: START HERE 2
// TODO: START HERE 2
// Remove vertex_function and fragment function!
// - We needed it before for the argument encoder, but we're migrating to Metal 3 Bindless.
pub struct CreateRenderPipelineResults {
    pub vertex_function: Function,
    pub fragment_function: Function,
    pub pipeline_state: RenderPipelineState,
    #[cfg(debug_assertions)]
    pub pipeline_state_reflection: RenderPipelineReflection,
}

#[inline]
pub fn create_pipeline(
    device: &Device,
    library: &Library,
    pipeline_desc: &RenderPipelineDescriptor,
    label: &str,
    func_constants: Option<&FunctionConstantValues>,
    vertex_func_name: &str,
    num_vertex_immutable_buffers: usize,
    frag_func_name: &str,
    num_frag_immutable_buffers: usize,
) -> CreateRenderPipelineResults {
    pipeline_desc.set_label(label);

    let vertex_function = unwrap_result_dcheck(
        new_function_from_library(library, vertex_func_name, func_constants),
        "Failed to access vertex shader function from metal library",
    );
    pipeline_desc.set_vertex_function(Some(&vertex_function));
    let fragment_function = unwrap_result_dcheck(
        new_function_from_library(library, frag_func_name, func_constants),
        "Failed to access fragment shader function from metal library",
    );
    pipeline_desc.set_fragment_function(Some(&fragment_function));

    for (buffers, num) in [
        (pipeline_desc.vertex_buffers(), num_vertex_immutable_buffers),
        (pipeline_desc.fragment_buffers(), num_frag_immutable_buffers),
    ] {
        let buffers = unwrap_option_dcheck(
            buffers,
            "Failed to access render pipeline descriptor buffers (vertex or fragment)",
        );
        for buffer_index in 0..num {
            unwrap_option_dcheck(
                buffers.object_at(buffer_index as _),
                "Failed to access fragment buffer",
            )
            .set_mutability(MTLMutability::Immutable);
        }
    }

    #[cfg(debug_assertions)]
    let (pipeline_state, pipeline_state_reflection) = unwrap_result_dcheck(
        device.new_render_pipeline_state_with_reflection(
            &pipeline_desc,
            MTLPipelineOption::ArgumentInfo,
        ),
        "Failed to create render pipeline",
    );
    #[cfg(not(debug_assertions))]
    let pipeline_state = unwrap_result_dcheck(
        device.new_render_pipeline_state(&pipeline_desc),
        "Failed to create render pipeline",
    );

    CreateRenderPipelineResults {
        vertex_function,
        fragment_function,
        pipeline_state,
        #[cfg(debug_assertions)]
        pipeline_state_reflection,
    }
}

#[inline]
pub fn new_basic_render_pass_descriptor<'a, 'b, 'c>(
    render_target: &'a TextureRef,
    depth_texture: Option<&'b Texture>,
) -> &'c RenderPassDescriptorRef {
    let desc = RenderPassDescriptor::new();
    {
        let a = desc
            .color_attachments()
            .object_at(0)
            .expect("Failed to access color attachment on render pass descriptor");
        a.set_texture(Some(render_target));
        a.set_load_action(MTLLoadAction::Clear);
        a.set_clear_color(MTLClearColor::new(0.0, 0.0, 0.0, 0.0));
        a.set_store_action(MTLStoreAction::Store);
    }
    if let Some(depth_texture) = depth_texture {
        let a = desc.depth_attachment().unwrap();
        a.set_clear_depth(1.);
        a.set_load_action(MTLLoadAction::Clear);
        a.set_store_action(MTLStoreAction::DontCare);
        a.set_texture(Some(depth_texture));
    }
    desc
}

// TODO: START HERE 2
// TODO: START HERE 2
// TODO: START HERE 2
// Make this a New Type so we don't accidentally try to do bad math.
pub type MetalGPUAddress = std::os::raw::c_ulong;
pub const METAL_GPU_ADDRESS_BYTE_SIZE: usize = std::mem::size_of::<MetalGPUAddress>();

#[inline(always)]
pub fn get_gpu_addresses<const N: usize>(bufs: [&BufferRef; N]) -> [MetalGPUAddress; N] {
    let sel = sel!(gpuAddress);
    bufs.map(|b| unsafe {
        let result = objc::__send_message(&*b, sel, ());
        #[cfg(debug_assertions)]
        match result {
            Err(s) => panic!("{}", s),
            Ok(r) => r,
        }
        #[cfg(not(debug_assertions))]
        result.unwrap_unchecked()
    })
}

#[inline(always)]
pub unsafe fn objc_sendmsg_with_cached_sel<T, R>(obj: *const T, sel: Sel) -> R
where
    T: objc::Message,
    R: Copy + Clone + 'static,
{
    let result = objc::__send_message(&*obj, sel, ());
    #[cfg(debug_assertions)]
    match result {
        Err(s) => panic!("{}", s),
        Ok(r) => r,
    }
    #[cfg(not(debug_assertions))]
    result.unwrap_unchecked()
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
    let command_buf = command_queue.new_command_buffer_with_unretained_references();
    let enc = command_buf.new_blit_command_encoder();
    for &texture in textures {
        enc.optimize_contents_for_gpu_access(texture);
    }
    enc.end_encoding();
    command_buf.commit();
    command_buf.wait_until_completed();
}
