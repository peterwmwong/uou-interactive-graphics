use cocoa::base::{nil, NO};
use metal::*;
use objc::runtime::Sel;
use std::{ffi::c_void, ops::Deref, path::Path};

use crate::image_helpers::{self, BYTES_PER_PIXEL};

pub const DEFAULT_RESOURCE_OPTIONS: MTLResourceOptions = MTLResourceOptions::from_bits_truncate(
    MTLResourceOptions::StorageModeShared.bits()
        | MTLResourceOptions::CPUCacheModeWriteCombined.bits(),
);

#[inline(always)]
pub const fn align_size(MTLSizeAndAlign { size, align }: MTLSizeAndAlign) -> usize {
    (size + (align - (size & (align - 1)))) as _
}

#[inline(always)]
pub const fn copy_into_buffer<T: Sized>(src: &[T], dst: *mut T, byte_offset: usize) -> usize {
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
        debug_assert_eq!(a.length(), b.length(), "Buffer lengths are not equal");
        let a_contents =
            unsafe { std::slice::from_raw_parts(a.contents() as *const u8, a.length() as _) };
        let b_contents =
            unsafe { std::slice::from_raw_parts(b.contents() as *const u8, b.length() as _) };
        assert_eq!(a_contents, b_contents, "Buffer contents are not equal");
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
    debug_assert_eq!(
        0,
        bytes % std::mem::size_of::<T>(),
        "Attempting to heap allocate by byte size that does not match the size of the type"
    );
    let buf = heap
        .new_buffer(bytes as u64, DEFAULT_RESOURCE_OPTIONS)
        .expect(&format!("Failed to allocate buffer for {label}"));
    buf.set_label(label);
    (buf.contents() as *mut T, buf)
}

#[inline]
pub fn allocate_new_buffer<'a, T: Sized>(
    device: &DeviceRef,
    label: &'static str,
    num_elements: usize,
) -> (&'a mut T, Buffer) {
    let buf = device.new_buffer(
        (std::mem::size_of::<T>() * num_elements) as u64,
        DEFAULT_RESOURCE_OPTIONS,
    );
    buf.set_label(label);
    (unsafe { &mut *(buf.contents() as *mut T) }, buf)
}

#[inline]
pub fn allocate_new_buffer_with_data<T: Sized>(
    device: &DeviceRef,
    label: &'static str,
    data: &[T],
) -> Buffer {
    let (contents, buffer) = allocate_new_buffer::<T>(&device, label, data.len());
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
    new_render_pipeline_descriptor_with_stencil(
        label,
        library,
        color_attachment_format_blending,
        depth_attachment_format,
        None,
        func_constants,
        vertex_func_name_num_imm_buffers,
        frag_func_name_num_imm_buffers,
    )
}

#[inline]
pub fn new_render_pipeline_descriptor_with_stencil(
    label: &str,
    library: &Library,
    color_attachment_format_blending: Option<(MTLPixelFormat, bool)>,
    depth_attachment_format: Option<MTLPixelFormat>,
    stencil_attachment_format: Option<MTLPixelFormat>,
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
    if let Some(stencil_pixel_format) = stencil_attachment_format {
        pipeline_desc.set_stencil_attachment_pixel_format(stencil_pixel_format);
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
pub fn new_render_pass_descriptor<'a, 'b, 'c, 'd>(
    color: Option<(
        &'a TextureRef,
        (f32, f32, f32, f32),
        MTLLoadAction,
        MTLStoreAction,
    )>,
    depth: Option<(&'b Texture, f32, MTLLoadAction, MTLStoreAction)>,
    stencil: Option<(&'d Texture, u32, MTLLoadAction, MTLStoreAction)>,
) -> &'c RenderPassDescriptorRef {
    let desc = RenderPassDescriptor::new();
    if let Some((render_target, (r, g, b, alpha), load_action, store_action)) = color {
        let a = desc
            .color_attachments()
            .object_at(0)
            .expect("Failed to access color attachment on render pass descriptor");
        a.set_clear_color(MTLClearColor::new(r as _, g as _, b as _, alpha as _));
        a.set_load_action(load_action);
        a.set_store_action(store_action);
        a.set_texture(Some(render_target));
    }
    if let Some((depth_texture, clear_depth, load_action, store_action)) = depth {
        let a = desc
            .depth_attachment()
            .expect("Failed to access depth attachment on render pass descriptor");
        a.set_clear_depth(clear_depth as f64);
        a.set_load_action(load_action);
        a.set_store_action(store_action);
        a.set_texture(Some(depth_texture));
    }
    if let Some((stencil_texture, clear_value, load_action, store_action)) = stencil {
        let a = desc
            .stencil_attachment()
            .expect("Failed to access stencil attachment on render pass descriptor");
        a.set_clear_stencil(clear_value);
        a.set_load_action(load_action);
        a.set_store_action(store_action);
        a.set_texture(Some(stencil_texture));
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

// TODO: Consider putting this under #[cfg(debug_assertions)]
// - This would force callers to also wrap calls with #[cfg(debug_assertions)]
pub mod debug_assert_pipeline_function_arguments {
    use super::*;

    enum BindingInner {
        Value {
            index: usize,
            size: usize,
        },
        Texture {
            index: usize,
            texture_type: MTLTextureType,
        },
        // TODO: Handle Buffer Pointer Type
        // - Example: `constant packed_float4 * positions [[buffer(0)]]`
        // - metal-rs needs to implement BufferPointerType and accessor on BufferBinding
        //   - https://developer.apple.com/documentation/metal/mtlbufferbinding/3929858-bufferpointertype
        // Pointer { index: usize, size: usize },
    }

    pub struct Binding(BindingInner);

    pub fn value_arg<T: Sized>(index: usize) -> Binding {
        Binding(BindingInner::Value {
            index,
            size: std::mem::size_of::<T>(),
        })
    }

    pub fn pointer_arg<T: Sized>(index: usize) -> Binding {
        value_arg::<T>(index)
    }

    pub fn texture_arg(index: usize, texture_type: MTLTextureType) -> Binding {
        Binding(BindingInner::Texture {
            index,
            texture_type,
        })
    }

    pub fn debug_assert_render_pipeline_function_arguments(
        p: &CreateRenderPipelineResults,
        vertex_buffer_index_and_sizes: &[Binding],
        fragment_buffer_index_and_sizes: Option<&[Binding]>,
    ) {
        #[cfg(debug_assertions)]
        for func_binds in [
            Some((
                "Vertex",
                p.pipeline_state_reflection.vertex_bindings(),
                vertex_buffer_index_and_sizes,
            )),
            fragment_buffer_index_and_sizes.map(|exp| {
                (
                    "Fragment",
                    p.pipeline_state_reflection.fragment_bindings(),
                    exp,
                )
            }),
        ] {
            if let Some((func, binds, expected)) = func_binds {
                debug_assert_eq!(
                    binds.count(),
                    expected.len() as _,
                    "Unexpected number of arguments for {func} function"
                );
                for (arg_index, binding) in expected.iter().enumerate() {
                    match binding {
                        &Binding(BindingInner::Value {
                            index: expected_index,
                            size: expected_size,
                        }) => {
                            let bind = binds
                            .object_at_as::<BufferBindingRef>(arg_index as _)
                            .expect(&format!(
                                "Failed to access {func} function buffer argument [{arg_index}] binding information"
                            ));
                            debug_assert_eq!(
                                expected_index,
                                bind.index() as _,
                                "Incorrect buffer index for {func} function buffer argument [{arg_index}]"
                            );
                            debug_assert_eq!(
                                expected_size, bind.buffer_data_size() as _,
                                "Incorrect argument size for {func} function buffer argument [{arg_index}]"
                            );
                        }
                        &Binding(BindingInner::Texture {
                            index: expected_index,
                            texture_type: expected_type,
                        }) => {
                            let bind = binds
                            .object_at_as::<TextureBindingRef>(arg_index as _)
                            .expect(&format!(
                                "Failed to access {func} function texture argument [{arg_index}] binding information"
                            ));
                            debug_assert_eq!(
                                expected_index, bind.index() as _,
                                "Incorrect texture index for {func} function texture argument {arg_index}"
                            );
                            debug_assert_eq!(
                                expected_type, bind.texture_type(),
                                "Incorrect texture type for {func} function texture argument {arg_index}"
                            );
                        }
                    }
                }
            }
        }
    }
}

pub fn new_texture_from_png<P: AsRef<Path>>(
    path_to_png: P,
    device: &Device,
    mut buffer: &mut Vec<u8>,
) -> Texture {
    let (bytes, (width, height)) =
        image_helpers::read_png_pixel_bytes_into(path_to_png, &mut buffer);

    let desc = TextureDescriptor::new();
    desc.set_pixel_format(MTLPixelFormat::RGBA8Unorm);
    desc.set_compression_type(MTLTextureCompressionType::Lossless);
    desc.set_resource_options(DEFAULT_RESOURCE_OPTIONS);
    desc.set_usage(MTLTextureUsage::ShaderRead);
    desc.set_width(width as _);
    desc.set_height(height as _);
    desc.set_depth(1);
    let texture = device.new_texture(&desc);

    texture.replace_region_in_slice(
        MTLRegion {
            origin: MTLOrigin { x: 0, y: 0, z: 0 },
            size: MTLSize {
                width: width as _,
                height: height as _,
                depth: 1,
            },
        },
        0,
        0,
        buffer.as_ptr() as _,
        (width * BYTES_PER_PIXEL) as _,
        bytes as _,
    );
    texture
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
        let _: () = msg_send![
            d,
            setTessellationPartitionMode: MTLTessellationPartitionModeFractionalEven
        ];

        const MAX_TESSELLATION_FACTOR: NSUInteger = 64;
        let _: () = msg_send![d, setMaxTessellationFactor: MAX_TESSELLATION_FACTOR];
    };
}

#[inline]
pub fn set_tesselation_factor_buffer<'a, 'b>(
    encoder: &'a RenderCommandEncoderRef,
    buf: &'b BufferRef,
) {
    unsafe {
        let _: () = msg_send![encoder, setTessellationFactorBuffer:buf
                                       offset: 0
                                       instanceStride: 0];
    }
}

#[allow(non_snake_case)]
#[inline]
pub fn draw_patches<'a, 'b>(
    encoder: &'a RenderCommandEncoderRef,
    numberOfPatchControlPoints: NSUInteger,
) {
    unsafe {
        let patchStart: NSUInteger = 0;
        let patchCount: NSUInteger = 1;
        let patchIndexBufferOffset: NSUInteger = 0;
        let instanceCount: NSUInteger = 1;
        let baseInstance: NSUInteger = 0;
        let _: () = msg_send![encoder, drawPatches:numberOfPatchControlPoints
                                       patchStart:patchStart
                                       patchCount:patchCount
                                       patchIndexBuffer:nil
                                       patchIndexBufferOffset:patchIndexBufferOffset
                                       instanceCount:instanceCount
                                       baseInstance:baseInstance];
    };
}
