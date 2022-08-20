use crate::image_helpers::{self, BYTES_PER_PIXEL};
use cocoa::base::{nil, NO};
use metal::*;
use objc::runtime::Sel;
use std::{ops::Deref, path::Path};

pub const DEFAULT_RESOURCE_OPTIONS: MTLResourceOptions = MTLResourceOptions::from_bits_truncate(
    MTLResourceOptions::StorageModeShared.bits()
        | MTLResourceOptions::CPUCacheModeWriteCombined.bits(),
);

#[inline(always)]
pub const fn align_size(MTLSizeAndAlign { size, align }: MTLSizeAndAlign) -> usize {
    (size + (align - (size & (align - 1)))) as _
}

#[inline]
pub fn rolling_copy<'a, 'b, T: Sized + Clone>(src: &'a [T], dest: &'b mut [T]) -> &'b mut [T] {
    let (l, r) = dest.split_at_mut(src.len());
    l.clone_from_slice(src);
    r
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

pub const DEFAULT_COLOR_FORMAT: MTLPixelFormat = MTLPixelFormat::BGRA8Unorm;
pub const DEFAULT_DEPTH_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;

pub struct CreateRenderPipelineResults {
    pub pipeline_state: RenderPipelineState,
    #[cfg(debug_assertions)]
    pub pipeline_state_reflection: RenderPipelineReflection,
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
