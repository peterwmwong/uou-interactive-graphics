use crate::typed_buffer::TypedBuffer;
use metal::{
    BufferRef, CommandBufferRef, DeviceRef, FunctionConstantValues, FunctionRef, LibraryRef,
    MTLClearColor, MTLDataType, MTLLoadAction, MTLPixelFormat, MTLStoreAction, NSUInteger,
    RenderCommandEncoderRef, RenderPassColorAttachmentDescriptorRef, RenderPassDescriptor,
    RenderPassDescriptorRef, RenderPipelineColorAttachmentDescriptorRef, RenderPipelineDescriptor,
    RenderPipelineDescriptorRef, RenderPipelineState, Texture, TextureRef,
};
use std::marker::PhantomData;

// TODO: Consider a TypedTexture, to enforce the pipeline and the render pass have the same pixel
// format.

#[derive(Copy, Clone)]
pub enum BindMany<'a, T: Sized + Copy + Clone> {
    Bytes(&'a [T]),
    BufferAndOffset(&'a TypedBuffer<T>, usize),
    BufferOffset(usize),
    UsePreviouslySet,
}
impl<'a, T: Sized + Copy + Clone> BindMany<'a, T> {
    #[inline]
    pub fn rolling_buffer_offset(buffer: &'a TypedBuffer<T>, element_offset: usize) -> Self {
        if element_offset == 0 {
            Self::BufferAndOffset(buffer, 0)
        } else {
            Self::BufferOffset(0)
        }
    }
}
#[derive(Copy, Clone)]
pub enum BindOne<'a, T: Sized + Copy + Clone> {
    Bytes(&'a T),
    BufferAndOffset(&'a TypedBuffer<T>, usize),
    BufferOffset(usize),
    UsePreviouslySet,
}
impl<'a, T: Sized + Copy + Clone> BindOne<'a, T> {
    #[inline]
    pub fn rolling_buffer_offset((buffer, element_offset): (&'a TypedBuffer<T>, usize)) -> Self {
        if element_offset == 0 {
            Self::BufferAndOffset(buffer, 0)
        } else {
            Self::BufferOffset(0)
        }
    }
}
pub struct BindTexture<'a>(&'a Texture);

#[inline]
fn encode_impl<'a, T: Sized + Copy + Clone>(
    encoder: &RenderCommandEncoderRef,
    encode_bytes: impl FnOnce(&RenderCommandEncoderRef, NSUInteger, NSUInteger, *const std::ffi::c_void),
    encode_buffer_and_offset: impl FnOnce(
        &RenderCommandEncoderRef,
        NSUInteger,
        Option<&'a BufferRef>,
        NSUInteger,
    ),
    encode_buffer_offset: impl FnOnce(&RenderCommandEncoderRef, NSUInteger, NSUInteger),
    bind: BindMany<'a, T>,
    bind_index: u64,
) {
    match bind {
        BindMany::Bytes(v) => encode_bytes(
            encoder,
            bind_index,
            std::mem::size_of_val(v) as _,
            v.as_ptr() as *const _,
        ),
        BindMany::BufferAndOffset(tb, o) => encode_buffer_and_offset(
            encoder,
            bind_index,
            Some(&tb.buffer),
            (std::mem::size_of::<T>() * o) as _,
        ),
        BindMany::BufferOffset(o) => {
            encode_buffer_offset(encoder, bind_index, (std::mem::size_of::<T>() * o) as _)
        }
        _ => {}
    }
}

pub trait BindEncoder {
    fn encode<'a, T: Sized + Copy + Clone>(
        encoder: &RenderCommandEncoderRef,
        bind: BindMany<'a, T>,
        bind_index: u64,
    );
    fn encode_texture<'a>(
        encoder: &RenderCommandEncoderRef,
        bind: BindTexture<'a>,
        bind_index: u64,
    );
    fn encode_one<'a, T: Sized + Copy + Clone>(
        encoder: &RenderCommandEncoderRef,
        bind: BindOne<'a, T>,
        bind_index: u64,
    ) {
        let tmp: [T; 1];
        Self::encode(
            encoder,
            match bind {
                BindOne::Bytes(&v) => {
                    tmp = [v];
                    BindMany::Bytes(&tmp)
                }
                BindOne::BufferAndOffset(b, o) => BindMany::BufferAndOffset(b, o),
                BindOne::BufferOffset(o) => BindMany::BufferOffset(o),
                BindOne::UsePreviouslySet => BindMany::UsePreviouslySet,
            },
            bind_index,
        );
    }
}

pub struct VertexBindEncoder;
impl BindEncoder for VertexBindEncoder {
    fn encode<'a, T: Sized + Copy + Clone>(
        encoder: &RenderCommandEncoderRef,
        bind: BindMany<'a, T>,
        bind_index: u64,
    ) {
        encode_impl(
            encoder,
            RenderCommandEncoderRef::set_vertex_bytes,
            RenderCommandEncoderRef::set_vertex_buffer,
            RenderCommandEncoderRef::set_vertex_buffer_offset,
            bind,
            bind_index,
        )
    }
    fn encode_texture<'a>(
        encoder: &RenderCommandEncoderRef,
        bind: BindTexture<'a>,
        bind_index: u64,
    ) {
        encoder.set_vertex_texture(bind_index, Some(&bind.0));
    }
}

pub struct FragmentBindEncoder;
impl BindEncoder for FragmentBindEncoder {
    fn encode<'a, T: Sized + Copy + Clone>(
        encoder: &RenderCommandEncoderRef,
        bind: BindMany<'a, T>,
        bind_index: u64,
    ) {
        encode_impl(
            encoder,
            RenderCommandEncoderRef::set_fragment_bytes,
            RenderCommandEncoderRef::set_fragment_buffer,
            RenderCommandEncoderRef::set_fragment_buffer_offset,
            bind,
            bind_index,
        )
    }
    fn encode_texture<'a>(
        encoder: &RenderCommandEncoderRef,
        bind: BindTexture<'a>,
        bind_index: u64,
    ) {
        encoder.set_fragment_texture(bind_index, Some(&bind.0));
    }
}

#[derive(Copy, Clone)]
pub enum BlendMode {
    NoBlend,
    Blend, // TODO: Add all the ways to color blend (source/destination alpha/rgb, blend factor, operation, etc.)
}

type ColorAttachementPipelineDesc = (MTLPixelFormat, BlendMode);
type ColorAttachementRenderPassDesc<'a> = (
    &'a TextureRef,
    (f32, f32, f32, f32),
    MTLLoadAction,
    MTLStoreAction,
);

pub struct ColorAttachement;
impl ColorAttachement {
    #[inline]
    fn setup_pipeline_attachment<'a>(
        desc: ColorAttachementPipelineDesc,
        pass: &RenderPipelineColorAttachmentDescriptorRef,
    ) {
        let (pixel_format, blend_mode) = desc;
        pass.set_pixel_format(pixel_format);
        pass.set_blending_enabled(matches!(blend_mode, BlendMode::Blend));
    }

    #[inline]
    fn setup_render_pass_attachment<'a>(
        desc: ColorAttachementRenderPassDesc<'a>,
        a: &RenderPassColorAttachmentDescriptorRef,
    ) {
        let (render_target, (r, g, b, alpha), load_action, store_action) = desc;
        a.set_clear_color(MTLClearColor::new(r as _, g as _, b as _, alpha as _));
        a.set_load_action(load_action);
        a.set_store_action(store_action);
        a.set_texture(Some(render_target));
    }
}

pub trait DepthAttachmentKind {
    type RenderPassDesc<'a>;

    #[inline]
    fn setup_pipeline_attachment(&self, _pipeline_descriptor: &RenderPipelineDescriptorRef) {}
    #[inline]
    fn setup_render_pass_attachment<'a>(
        _desc: Self::RenderPassDesc<'a>,
        _pass: &RenderPassDescriptorRef,
    ) {
    }
}
pub struct HasDepth(pub MTLPixelFormat);
impl DepthAttachmentKind for HasDepth {
    type RenderPassDesc<'a> = (&'a TextureRef, f32, MTLLoadAction, MTLStoreAction);

    #[inline]
    fn setup_pipeline_attachment(&self, pipeline_descriptor: &RenderPipelineDescriptorRef) {
        pipeline_descriptor.set_depth_attachment_pixel_format(self.0);
    }

    #[inline]
    fn setup_render_pass_attachment<'a>(
        (texture, clear_depth, load_action, store_action): Self::RenderPassDesc<'a>,
        desc: &RenderPassDescriptorRef,
    ) {
        let a = desc
            .depth_attachment()
            .expect("Failed to access depth attachment on render pass descriptor");
        a.set_clear_depth(clear_depth as f64);
        a.set_load_action(load_action);
        a.set_store_action(store_action);
        a.set_texture(Some(texture));
    }
}
pub struct NoDepth;
impl DepthAttachmentKind for NoDepth {
    type RenderPassDesc<'a> = NoDepth;
}

pub trait StencilAttachmentKind {
    type RenderPassDesc<'a>;

    fn setup_pipeline_attachment(&self, pipeline_descriptor: &RenderPipelineDescriptorRef);
    fn setup_render_pass_attachment<'a>(
        desc: Self::RenderPassDesc<'a>,
        pass: &RenderPassDescriptorRef,
    );
}
pub struct HasStencil(pub MTLPixelFormat);
impl StencilAttachmentKind for HasStencil {
    type RenderPassDesc<'a> = (&'a TextureRef, u32, MTLLoadAction, MTLStoreAction);

    #[inline]
    fn setup_pipeline_attachment(&self, pipeline_descriptor: &RenderPipelineDescriptorRef) {
        pipeline_descriptor.set_stencil_attachment_pixel_format(self.0);
    }

    #[inline]
    fn setup_render_pass_attachment<'a>(
        (texture, clear_value, load_action, store_action): Self::RenderPassDesc<'a>,
        desc: &RenderPassDescriptorRef,
    ) {
        let a = desc
            .stencil_attachment()
            .expect("Failed to access Stencil attachment on render pass descriptor");
        a.set_clear_stencil(clear_value);
        a.set_load_action(load_action);
        a.set_store_action(store_action);
        a.set_texture(Some(texture));
    }
}
pub struct NoStencil;
impl StencilAttachmentKind for NoStencil {
    type RenderPassDesc<'a> = NoStencil;

    #[inline]
    fn setup_pipeline_attachment(&self, _pipeline_descriptor: &RenderPipelineDescriptorRef) {}

    #[inline]
    fn setup_render_pass_attachment<'a>(
        _desc: Self::RenderPassDesc<'a>,
        _pass: &RenderPassDescriptorRef,
    ) {
    }
}

pub trait FunctionBinds {
    fn encode_binds<E: BindEncoder>(self, encoder: &RenderCommandEncoderRef);
}

pub struct NoBinds;
impl FunctionBinds for NoBinds {
    #[inline]
    fn encode_binds<E: BindEncoder>(self, _encoder: &RenderCommandEncoderRef) {}
}

pub trait FunctionType {
    fn setup_render_pipeline(func: &FunctionRef, pipeline_desc: &RenderPipelineDescriptorRef);
}

pub struct VertexFunctionType;
impl FunctionType for VertexFunctionType {
    fn setup_render_pipeline(func: &FunctionRef, pipeline_desc: &RenderPipelineDescriptorRef) {
        pipeline_desc.set_vertex_function(Some(&func));
    }
}

pub struct FragmentFunctionType;
impl FunctionType for FragmentFunctionType {
    fn setup_render_pipeline(func: &FunctionRef, pipeline_desc: &RenderPipelineDescriptorRef) {
        pipeline_desc.set_fragment_function(Some(&func));
    }
}

pub trait Function {
    const FUNCTION_NAME: &'static str;
    type Binds<'a>: FunctionBinds;
    type Type: FunctionType;
    type FunctionConstantsType: FunctionConstantsFactory;

    #[inline]
    fn get_function(
        lib: &LibraryRef,
        function_constants: Option<FunctionConstantValues>,
    ) -> metal::Function {
        lib.get_function(Self::FUNCTION_NAME, function_constants)
            .expect("Failed to get vertex function from library")
    }
}

pub struct NoFragmentShader;
impl Function for NoFragmentShader {
    type Binds<'a> = NoBinds;
    const FUNCTION_NAME: &'static str = "<NoFragmentShader>";
    type Type = FragmentFunctionType;
    type FunctionConstantsType = NoFunctionConstants;
}

pub trait FunctionConstantsFactory {
    fn create_function_constant_values(&self) -> Option<FunctionConstantValues>;
}

pub struct NoFunctionConstants;
impl FunctionConstantsFactory for NoFunctionConstants {
    fn create_function_constant_values(&self) -> Option<FunctionConstantValues> {
        None
    }
}

pub trait HasMTLDataType {
    const MTL_DATA_TYPE: MTLDataType;
}
macro_rules! into_mtl_data_type {
    ($from:path, $mtl_data_type:path) => {
        impl HasMTLDataType for $from {
            const MTL_DATA_TYPE: MTLDataType = $mtl_data_type;
        }
    };
}

into_mtl_data_type!(bool, MTLDataType::Bool);
into_mtl_data_type!(metal_types::float, MTLDataType::Float);
into_mtl_data_type!(metal_types::float2, MTLDataType::Float2);
// into_mtl_data_type!(metal_types::float3, MTLDataType::Float3);
into_mtl_data_type!(metal_types::float4, MTLDataType::Float4);
into_mtl_data_type!(metal_types::uint, MTLDataType::UInt);
into_mtl_data_type!(metal_types::int, MTLDataType::Int);
into_mtl_data_type!(metal_types::ushort, MTLDataType::UShort);
into_mtl_data_type!(metal_types::short, MTLDataType::Short);

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// Consider metal-build generating {Render/Compute/Mesh}Pipeline related helpers
// - Currently, users stil need define RenderPipeline<...> and apply the write combination of NUM_COLOR_ATTACHMENTS, FCS, VS, FS
//    - During metal-build binding generation we have enough information to limit combinations
//       - We know FCS is required needed or not.
//       - We know a RenderPipeline is possible or not (is there a VS?)
//          - Depending on FS return type (has `[[color(n)]]`? or just `half4`/`float4`), we know NUM_COLOR_ATTACHMENTS
//          - If there's only one VS... well the user doesn't even need to choose? (overly optimized?)
//       - We know a MeshPipeline (future) is possible or not (is there an Object and Mesh function?)
//       - We know a ComputePipeline (future) is possible or not (is there an Kernel function?)
// - We could go further to solve/mitigate the following...
//   - Use the wrong FunctionConstantFactory type
//       - ex. Accidentally use NoFunctionConstants when it is actually required
//   - Use the wrong NUM_COLOR_ATTACHMENTS
//       - ex. FS has `[[color(2)]]`, but NUM_COLOR_ATTACHMENTS is 1
// 1. Sketch out what this would look like
// 2. Does it actually worth the code generation complexity?

pub struct RenderPipeline<
    const NUM_COLOR_ATTACHMENTS: usize,
    FCF: FunctionConstantsFactory,
    VS: Function<Type = VertexFunctionType, FunctionConstantsType = FCF>,
    FS: Function<Type = FragmentFunctionType, FunctionConstantsType = FCF>,
    D: DepthAttachmentKind,
    S: StencilAttachmentKind,
> {
    pub pipeline: RenderPipelineState,
    _function_constants: PhantomData<FCF>,
    _vertex_fn: PhantomData<VS>,
    _fragment_fn: PhantomData<FS>,
    _depth_kind: PhantomData<D>,
    _stencil_kind: PhantomData<S>,
}

impl<
        const NUM_COLOR_ATTACHMENTS: usize,
        FCF: FunctionConstantsFactory,
        VS: Function<Type = VertexFunctionType, FunctionConstantsType = FCF>,
        FS: Function<Type = FragmentFunctionType, FunctionConstantsType = FCF>,
        D: DepthAttachmentKind,
        S: StencilAttachmentKind,
    > RenderPipeline<NUM_COLOR_ATTACHMENTS, FCF, VS, FS, D, S>
{
    pub fn new(
        label: &str,
        device: &DeviceRef,
        library: &LibraryRef,
        colors: [ColorAttachementPipelineDesc; NUM_COLOR_ATTACHMENTS],
        function_constants: FCF,
        depth_kind: D,
        stencil_kind: S,
    ) -> Self {
        let pipeline_desc = RenderPipelineDescriptor::new();
        pipeline_desc.set_label(label);

        for i in 0..NUM_COLOR_ATTACHMENTS {
            let desc = pipeline_desc
                .color_attachments()
                .object_at(i as u64)
                .expect("Failed to access color attachment on pipeline descriptor");
            ColorAttachement::setup_pipeline_attachment(colors[i], &desc);
        }
        depth_kind.setup_pipeline_attachment(&pipeline_desc);
        stencil_kind.setup_pipeline_attachment(&pipeline_desc);

        // TODO: Set vertex/fragment shader buffer arguments as immutable, where appropriate.

        let fcs_v = function_constants.create_function_constant_values();
        let fcs_f = fcs_v.clone();

        // TODO: Is it any faster to manually clone and avoid calling Obj-C `retain`?
        // - Does this double drop?
        //   - Can we work around this with a std::mem::forget?
        // let fcs_v = function_constants.create_function_constant_values();
        // let fcs_f = fcs_v
        //     .as_ref()
        //     .map(|m| unsafe { FunctionConstantValues::from_ptr(m.as_ptr()) });
        VS::Type::setup_render_pipeline(&VS::get_function(library, fcs_v), &pipeline_desc);
        FS::Type::setup_render_pipeline(&FS::get_function(library, fcs_f), &pipeline_desc);

        // TODO: START HERE 2
        // TODO: START HERE 2
        // TODO: START HERE 2
        // Add back non-release profile bind checking

        let pipeline = device
            .new_render_pipeline_state(&pipeline_desc)
            .expect("Failed to create pipeline state");
        Self {
            pipeline,
            _function_constants: PhantomData,
            _vertex_fn: PhantomData,
            _fragment_fn: PhantomData,
            _depth_kind: PhantomData,
            _stencil_kind: PhantomData,
        }
    }

    pub fn new_render_command_encoder<'a, 'b, 'c>(
        &self,
        label: &'static str,
        command_buffer: &'a CommandBufferRef,
        color_attachments: [ColorAttachementRenderPassDesc; NUM_COLOR_ATTACHMENTS],
        depth_attachment: D::RenderPassDesc<'b>,
        stencil_attachment: S::RenderPassDesc<'c>,
    ) -> &'a RenderCommandEncoderRef {
        let desc = RenderPassDescriptor::new();
        for i in 0..NUM_COLOR_ATTACHMENTS {
            let c = color_attachments[i];
            let a = desc
                .color_attachments()
                .object_at(i as _)
                .expect("Failed to access color attachment on render pass descriptor");
            ColorAttachement::setup_render_pass_attachment(c, a);
        }
        D::setup_render_pass_attachment(depth_attachment, desc);
        S::setup_render_pass_attachment(stencil_attachment, desc);
        let encoder = command_buffer.new_render_command_encoder(desc);
        encoder.set_label(label);
        encoder.set_render_pipeline_state(&self.pipeline);
        // TODO: How to handle depth/stencil state?
        encoder
    }

    pub fn setup_binds<'a, 'b, 'c>(
        &'a self,
        encoder: &RenderCommandEncoderRef,
        vertex_binds: VS::Binds<'b>,
        fragment_binds: FS::Binds<'c>,
    ) {
        vertex_binds.encode_binds::<VertexBindEncoder>(encoder);
        fragment_binds.encode_binds::<FragmentBindEncoder>(encoder);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use metal::{Device, MTLResourceOptions, TextureDescriptor};
    use metal_types::float4;
    use std::simd::f32x4;

    struct FunctionConstants1;
    impl FunctionConstantsFactory for FunctionConstants1 {
        #[inline]
        fn create_function_constant_values(&self) -> Option<FunctionConstantValues> {
            let fcv = FunctionConstantValues::new();
            fcv.set_constant_value_at_index(
                (&0_f32 as *const _) as _,
                metal_types::float::MTL_DATA_TYPE,
                0,
            );
            Some(fcv)
        }
    }

    // Assumed to be generated by metal-build (shader_function_parser/generate_shader_binds)
    struct Vertex1Binds<'a> {
        v_bind1: BindMany<'a, f32>,
    }
    impl FunctionBinds for Vertex1Binds<'_> {
        #[inline]
        fn encode_binds<E: BindEncoder>(self, encoder: &RenderCommandEncoderRef) {
            E::encode(encoder, self.v_bind1, 0);
        }
    }
    struct Vertex1;
    impl Function for Vertex1 {
        const FUNCTION_NAME: &'static str = "vertex1";
        type Binds<'a> = Vertex1Binds<'a>;
        type Type = VertexFunctionType;
        type FunctionConstantsType = FunctionConstants1;
    }

    // Assumed to be generated by metal-build (shader_function_parser/generate_shader_binds)
    struct Frag1Binds<'a> {
        f_bind1: BindOne<'a, float4>,
        f_tex2: BindTexture<'a>,
    }
    impl FunctionBinds for Frag1Binds<'_> {
        #[inline]
        fn encode_binds<E: BindEncoder>(self, encoder: &RenderCommandEncoderRef) {
            E::encode_one(encoder, self.f_bind1, 0);
            E::encode_texture(encoder, self.f_tex2, 1);
        }
    }
    struct Fragment1;
    impl Function for Fragment1 {
        const FUNCTION_NAME: &'static str = "fragment1";
        type Binds<'a> = Frag1Binds<'a>;
        type Type = FragmentFunctionType;
        type FunctionConstantsType = FunctionConstants1;
    }

    struct Vertex2NoFunctionConstants;
    impl Function for Vertex2NoFunctionConstants {
        const FUNCTION_NAME: &'static str = "vertex1";
        type Binds<'a> = Vertex1Binds<'a>;
        type Type = VertexFunctionType;
        type FunctionConstantsType = NoFunctionConstants;
    }

    // TODO: START HERE 3
    // TODO: START HERE 3
    // TODO: START HERE 3
    // How to handle pipeline state updates (see proj-6)
    // - I think the only limitation is the Render Pass dictates a certain Color, Depth and Stencil.
    // - So we could allow/enforce changing to a pipeline state with the same (subset?) Color/Depth/Stencil.
    //   - Enforcement would have been suuuuper helpful when developing proj-6
    //   - Accidentally setting an incompatible stencil/depth state (attachment texture format was wrong/different).

    // - Should RenderPipeline encase the whole encoder?
    //   - Handles encoder.end_encoding()
    //   - Limit/focus encoding API (encode_binds, encode_update_depth_state)

    // #[test]
    fn test() {
        let device = Device::system_default().expect("Failed to get Metal Device");
        let lib = device.new_default_library();
        let command_queue = device.new_command_queue();
        let command_buffer = command_queue.new_command_buffer();

        let texture = device.new_texture(&TextureDescriptor::new());
        let color1 = &texture;
        let color2 = &texture;
        let depth = &texture;
        let stencil = &texture;

        let f32_buffer = TypedBuffer::<f32>::with_capacity(
            "f32_buffer",
            &device as &DeviceRef,
            1,
            MTLResourceOptions::StorageModeManaged,
        );
        let float4_buffer = TypedBuffer::<float4>::with_capacity(
            "float4_buffer",
            &device as &DeviceRef,
            1,
            MTLResourceOptions::StorageModeManaged,
        );

        {
            let p: RenderPipeline<1, FunctionConstants1, Vertex1, Fragment1, NoDepth, NoStencil> =
                RenderPipeline::new(
                    "Test",
                    &device,
                    &lib,
                    [(MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend)],
                    FunctionConstants1,
                    NoDepth,
                    NoStencil,
                );
            let encoder = p.new_render_command_encoder(
                "test label",
                command_buffer,
                [(
                    color1,
                    (0., 0., 0., 0.),
                    MTLLoadAction::Clear,
                    MTLStoreAction::Store,
                )],
                NoDepth,
                NoStencil,
            );
            p.setup_binds(
                encoder,
                Vertex1Binds {
                    v_bind1: BindMany::Bytes(&[0.]),
                },
                Frag1Binds {
                    f_bind1: BindOne::Bytes(&f32x4::splat(1.).into()),
                    f_tex2: BindTexture(&texture),
                },
            );

            p.setup_binds(
                encoder,
                Vertex1Binds {
                    v_bind1: BindMany::BufferAndOffset(&f32_buffer, 0),
                },
                Frag1Binds {
                    f_bind1: BindOne::BufferAndOffset(&float4_buffer, 0),
                    f_tex2: BindTexture(&texture),
                },
            );
        }
        {
            let p: RenderPipeline<2, FunctionConstants1, Vertex1, Fragment1, NoDepth, NoStencil> =
                RenderPipeline::new(
                    "Test",
                    &device,
                    &lib,
                    [
                        (MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend),
                        (MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend),
                    ],
                    FunctionConstants1,
                    NoDepth,
                    NoStencil,
                );
            let encoder = p.new_render_command_encoder(
                "test label",
                command_buffer,
                [
                    (
                        color1,
                        (0., 0., 0., 0.),
                        MTLLoadAction::Clear,
                        MTLStoreAction::Store,
                    ),
                    (
                        color2,
                        (0., 0., 0., 0.),
                        MTLLoadAction::Clear,
                        MTLStoreAction::Store,
                    ),
                ],
                NoDepth,
                NoStencil,
            );
            p.setup_binds(
                encoder,
                Vertex1Binds {
                    v_bind1: BindMany::Bytes(&[0.]),
                },
                Frag1Binds {
                    f_bind1: BindOne::Bytes(&f32x4::splat(1.).into()),
                    f_tex2: BindTexture(&texture),
                },
            );
        }
        {
            let p: RenderPipeline<
                1,
                NoFunctionConstants,
                Vertex2NoFunctionConstants,
                NoFragmentShader,
                HasDepth,
                HasStencil,
            > = RenderPipeline::new(
                "Test",
                &device,
                &lib,
                [(MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend)],
                NoFunctionConstants,
                HasDepth(MTLPixelFormat::Depth16Unorm),
                HasStencil(MTLPixelFormat::Stencil8),
            );
            let encoder = p.new_render_command_encoder(
                "test label",
                command_buffer,
                [(
                    color1,
                    (0., 0., 0., 0.),
                    MTLLoadAction::Clear,
                    MTLStoreAction::Store,
                )],
                (depth, 1., MTLLoadAction::Clear, MTLStoreAction::DontCare),
                (stencil, 0, MTLLoadAction::Clear, MTLStoreAction::DontCare),
            );
            p.setup_binds(
                encoder,
                Vertex1Binds {
                    v_bind1: BindMany::Bytes(&[0.]),
                },
                NoBinds,
            );
        }
    }
}
