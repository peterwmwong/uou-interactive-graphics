use cocoa::base::{nil, NO};
use metal::*;
use objc::runtime::Sel;
use std::{ffi::c_void, ops::Deref};

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
pub fn encode_vertex_bytes<T: Sized>(encoder: &RenderCommandEncoderRef, buffer_index: u64, v: &T) {
    let ptr: *const T = v;
    encoder.set_vertex_bytes(
        buffer_index,
        std::mem::size_of::<T>() as _,
        ptr as *const c_void,
    );
}

#[inline]
pub fn encode_fragment_bytes<T: Sized>(
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

pub const DEFAULT_PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::BGRA8Unorm;

#[inline]
pub fn new_render_pipeline_descriptor(
    label: &str,
    library: &Library,
    color_attachment_format_blending: Option<(MTLPixelFormat, bool)>,
    depth_attachment_format: Option<MTLPixelFormat>,
    func_constants: Option<&FunctionConstantValues>,
    vertex_func_name_num_imm_buffers: Option<(&str, usize)>,
    frag_func_name_num_imm_buffers: Option<(&str, usize)>,
) -> RenderPipelineDescriptor {
    let pipeline_desc = RenderPipelineDescriptor::new();
    pipeline_desc.set_label(label);

    if let Some((pixel_format, blending)) = color_attachment_format_blending {
        let desc = pipeline_desc
            .color_attachments()
            .object_at(0 as u64)
            .expect("Failed to access color attachment on pipeline descriptor");
        desc.set_blending_enabled(blending);
        desc.set_pixel_format(pixel_format);
    }

    if let Some(depth_pixel_format) = depth_attachment_format {
        pipeline_desc.set_depth_attachment_pixel_format(depth_pixel_format);
    }

    let set_buffers_immutable = |buffers: Option<&PipelineBufferDescriptorArrayRef>, num: usize| {
        let buffers = buffers
            .expect("Failed to access render pipeline descriptor buffers (vertex or fragment)");
        for buffer_index in 0..num {
            buffers
                .object_at(buffer_index as _)
                .expect("Failed to access fragment buffer")
                .set_mutability(MTLMutability::Immutable);
        }
    };

    if let Some((vertex_func_name, num_imm_buffers)) = vertex_func_name_num_imm_buffers {
        let vertex_function = library
            .get_function(vertex_func_name, func_constants.map(|f| f.to_owned()))
            .expect("Failed to access vertex shader function from metal library");
        pipeline_desc.set_vertex_function(Some(&vertex_function));
        set_buffers_immutable(pipeline_desc.vertex_buffers(), num_imm_buffers);
    }

    if let Some((frag_func_name, num_imm_buffers)) = frag_func_name_num_imm_buffers {
        let fragment_function = library
            .get_function(frag_func_name, func_constants.map(|f| f.to_owned()))
            .expect("Failed to access fragment shader function from metal library");
        pipeline_desc.set_fragment_function(Some(&fragment_function));
        set_buffers_immutable(pipeline_desc.fragment_buffers(), num_imm_buffers);
    }

    pipeline_desc
}

pub struct CreateRenderPipelineResults {
    pub pipeline_state: RenderPipelineState,
    #[cfg(debug_assertions)]
    pub pipeline_state_reflection: RenderPipelineReflection,
}

pub fn create_render_pipeline(
    device: &Device,
    pipeline_desc: &RenderPipelineDescriptor,
) -> CreateRenderPipelineResults {
    #[cfg(debug_assertions)]
    let (pipeline_state, pipeline_state_reflection) = device
        .new_render_pipeline_state_with_reflection(&pipeline_desc, MTLPipelineOption::ArgumentInfo)
        .expect("Failed to create render pipeline");
    #[cfg(not(debug_assertions))]
    let pipeline_state = device
        .new_render_pipeline_state(&pipeline_desc)
        .expect("Failed to create render pipeline");
    CreateRenderPipelineResults {
        pipeline_state,
        #[cfg(debug_assertions)]
        pipeline_state_reflection,
    }
}

#[inline]
pub fn new_render_pass_descriptor<'a, 'b, 'c>(
    render_target: Option<&'a TextureRef>,
    depth_texture: Option<(&'b Texture, MTLStoreAction)>,
) -> &'c RenderPassDescriptorRef {
    let desc = RenderPassDescriptor::new();
    if let Some(render_target) = render_target {
        let a = desc
            .color_attachments()
            .object_at(0)
            .expect("Failed to access color attachment on render pass descriptor");
        a.set_clear_color(MTLClearColor::new(0.0, 0.0, 0.0, 0.0));
        a.set_load_action(MTLLoadAction::Clear);
        a.set_store_action(MTLStoreAction::Store);
        a.set_texture(Some(render_target));
    }
    if let Some((depth_texture, store_action)) = depth_texture {
        let a = desc
            .depth_attachment()
            .expect("Failed to access depth/stencil attachment on render pass descriptor");
        a.set_clear_depth(1.);
        a.set_load_action(MTLLoadAction::Clear);
        a.set_store_action(store_action);
        a.set_texture(Some(depth_texture));
    }
    desc
}

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

pub enum FunctionType {
    Vertex,
    Fragment,
}

pub fn debug_assert_argument_buffer_size<const BUFFER_INDEX: u64, T>(
    p: &CreateRenderPipelineResults,
    func_type: FunctionType,
) {
    #[cfg(debug_assertions)]
    {
        let bindings: &BindingArrayRef = match func_type {
            FunctionType::Vertex => p.pipeline_state_reflection.vertex_bindings(),
            FunctionType::Fragment => p.pipeline_state_reflection.fragment_bindings(),
        };
        let arg_size = bindings
            .object_at_as::<BufferBindingRef>(BUFFER_INDEX)
            .expect(&format!(
                "Failed to access binding information at buffer index {BUFFER_INDEX}"
            ))
            .buffer_data_size();
        debug_assert_eq!(
            std::mem::size_of::<T>(),
            arg_size as _,
            "Shader bindings generated a differently sized argument struct than what Metal expects for buffer index {BUFFER_INDEX}"
        );
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
    let command_buf = command_queue.new_command_buffer_with_unretained_references();
    let enc = command_buf.new_blit_command_encoder();
    for &texture in textures {
        enc.optimize_contents_for_gpu_access(texture);
    }
    enc.end_encoding();
    command_buf.commit();
    command_buf.wait_until_completed();
}

pub fn set_tessellation_config(desc: &mut RenderPipelineDescriptor) {
    unsafe {
        let d: &RenderPipelineDescriptorRef = desc.deref();
        let _: () = msg_send![d, setTessellationFactorScaleEnabled: NO];

        #[allow(non_upper_case_globals)]
        const MTLTessellationFactorFormatHalf: NSUInteger = 0;
        let _: () = msg_send![
            d,
            setTessellationFactorFormat: MTLTessellationFactorFormatHalf
        ];

        #[allow(non_upper_case_globals)]
        const MTLTessellationControlPointIndexTypeNone: NSUInteger = 0;
        let _: () = msg_send![
            d,
            setTessellationControlPointIndexType: MTLTessellationControlPointIndexTypeNone
        ];

        #[allow(non_upper_case_globals)]
        const MTLTessellationFactorStepFunctionConstant: NSUInteger = 0;
        let _: () = msg_send![
            d,
            setTessellationFactorStepFunction: MTLTessellationFactorStepFunctionConstant
        ];

        #[allow(non_upper_case_globals)]
        const winding: MTLWinding = MTLWinding::Clockwise;
        let _: () = msg_send![d, setTessellationOutputWindingOrder: winding];

        #[allow(non_upper_case_globals)]
        const MTLTessellationPartitionModeFractionalEven: NSUInteger = 3;
        // TODO: Could be MTLTessellationPartitionModeInteger = 1 or MTLTessellationPartitionModePow2 = 0
        let _: () = msg_send![
            d,
            setTessellationPartitionMode: MTLTessellationPartitionModeFractionalEven
        ];

        const MAX_TESSELLATION_FACTOR: NSUInteger = 64;
        let _: () = msg_send![d, setMaxTessellationFactor: MAX_TESSELLATION_FACTOR];
    };
}

#[inline]
pub fn draw_patches_with_tesselation_factor_buffer<'a, 'b>(
    encoder: &'a RenderCommandEncoderRef,
    buf: &'b BufferRef,
    patch_control_points: NSUInteger,
) {
    unsafe {
        let _: () = msg_send![encoder, setTessellationFactorBuffer:buf
                                       offset: 0
                                       instanceStride: 0];
        let _: () = msg_send![encoder, drawPatches:patch_control_points
                                       patchStart:0
                                       patchCount:1
                                       patchIndexBuffer:nil
                                       patchIndexBufferOffset:0
                                       instanceCount:1
                                       baseInstance:0];
    };
}
