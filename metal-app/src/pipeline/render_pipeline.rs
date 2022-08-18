use super::{
    bind::Binds, function, pipeline_function::*, TesselationRenderPass, TesselationRenderPipeline,
};
use crate::{debug_time, set_tesselation_factor_buffer, typed_buffer::TypedBuffer};
use metal::*;
use metal_types::MTLQuadTessellationFactorsHalf;
use std::marker::PhantomData;

// TODO: START HERE 2
// TODO: START HERE 2
// TODO: START HERE 2
// Create a TypedTexture, to enforce the pipeline and the render pass have the same pixel format.
// - There's probably overlap with components::TextureAndConfig

#[derive(Copy, Clone)]
pub enum BlendMode {
    NoBlend,
    Blend, // TODO: Add all the ways to color blend (source/destination alpha/rgb, blend factor, operation, etc.)
}

pub type ColorPipelineDesc = (MTLPixelFormat, BlendMode);
pub type ColorRenderPassDesc<'a> = (
    &'a TextureRef,
    (f32, f32, f32, f32),
    MTLLoadAction,
    MTLStoreAction,
);

pub struct Color;
impl Color {
    #[inline]
    pub(crate) fn setup_pipeline(
        desc: ColorPipelineDesc,
        pass: &RenderPipelineColorAttachmentDescriptorRef,
    ) {
        let (pixel_format, blend_mode) = desc;
        pass.set_pixel_format(pixel_format);
        pass.set_blending_enabled(matches!(blend_mode, BlendMode::Blend));
    }

    #[inline]
    pub(crate) fn setup_render_pass<'a>(
        desc: ColorRenderPassDesc<'a>,
        a: &RenderPassColorAttachmentDescriptorRef,
    ) {
        let (render_target, (r, g, b, alpha), load_action, store_action) = desc;
        a.set_clear_color(MTLClearColor::new(r as _, g as _, b as _, alpha as _));
        a.set_load_action(load_action);
        a.set_store_action(store_action);
        a.set_texture(Some(render_target));
    }
}

pub trait DepthKind {
    type RenderPassDesc<'a>;

    #[inline]
    fn setup_pipeline(&self, _pipeline_descriptor: &RenderPipelineDescriptorRef) {}
    #[inline]
    fn setup_render_pass<'a>(_desc: Self::RenderPassDesc<'a>, _pass: &RenderPassDescriptorRef) {}
}
pub struct Depth(pub MTLPixelFormat);
impl DepthKind for Depth {
    type RenderPassDesc<'a> = (&'a TextureRef, f32, MTLLoadAction, MTLStoreAction);

    #[inline]
    fn setup_pipeline(&self, pipeline_descriptor: &RenderPipelineDescriptorRef) {
        pipeline_descriptor.set_depth_attachment_pixel_format(self.0);
    }

    #[inline]
    fn setup_render_pass<'a>(
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
impl DepthKind for NoDepth {
    type RenderPassDesc<'a> = NoDepth;
}

pub trait StencilKind {
    type RenderPassDesc<'a>;

    fn setup_pipeline(&self, pipeline_descriptor: &RenderPipelineDescriptorRef);
    fn setup_render_pass<'a>(desc: Self::RenderPassDesc<'a>, pass: &RenderPassDescriptorRef);
}
pub struct Stencil(pub MTLPixelFormat);
impl StencilKind for Stencil {
    type RenderPassDesc<'a> = (&'a TextureRef, u32, MTLLoadAction, MTLStoreAction);

    #[inline]
    fn setup_pipeline(&self, pipeline_descriptor: &RenderPipelineDescriptorRef) {
        pipeline_descriptor.set_stencil_attachment_pixel_format(self.0);
    }

    #[inline]
    fn setup_render_pass<'a>(
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
impl StencilKind for NoStencil {
    type RenderPassDesc<'a> = NoStencil;

    #[inline]
    fn setup_pipeline(&self, _pipeline_descriptor: &RenderPipelineDescriptorRef) {}

    #[inline]
    fn setup_render_pass<'a>(_desc: Self::RenderPassDesc<'a>, _pass: &RenderPassDescriptorRef) {}
}

pub trait DepthState {
    fn setup_render_pass<'a>(&self, encoder: &'a RenderCommandEncoderRef);
}

impl DepthState for &DepthStencilStateRef {
    #[inline]
    fn setup_render_pass<'a>(&self, encoder: &'a RenderCommandEncoderRef) {
        encoder.set_depth_stencil_state(self)
    }
}

impl DepthState for (&'_ DepthStencilStateRef, u32, u32) {
    #[inline]
    fn setup_render_pass<'a>(&self, encoder: &'a RenderCommandEncoderRef) {
        encoder.set_depth_stencil_state(self.0);
        encoder.set_stencil_front_back_reference_value(self.1 as _, self.2 as _);
    }
}

pub struct NoDepthState;
impl DepthState for NoDepthState {
    #[inline]
    fn setup_render_pass<'a>(&self, _: &'a RenderCommandEncoderRef) {}
}

pub trait DepthStencilKind {
    type DepthKind: DepthKind;
    type StencilKind: StencilKind;
    type DepthState<'a>: DepthState;

    #[inline]
    fn setup_pipeline(&self, pipeline_descriptor: &RenderPipelineDescriptorRef) {
        self.depth_kind().setup_pipeline(pipeline_descriptor);
        self.stencil_kind().setup_pipeline(pipeline_descriptor);
    }
    #[inline]
    fn setup_render_pass<'a>(
        depth_desc: <Self::DepthKind as DepthKind>::RenderPassDesc<'a>,
        stencil_desc: <Self::StencilKind as StencilKind>::RenderPassDesc<'a>,
        pass: &'a RenderPassDescriptorRef,
    ) {
        Self::DepthKind::setup_render_pass(depth_desc, pass);
        Self::StencilKind::setup_render_pass(stencil_desc, pass);
    }

    fn depth_kind(&self) -> &Self::DepthKind;
    fn stencil_kind(&self) -> &Self::StencilKind;
}

impl DepthStencilKind for (Depth, Stencil) {
    type DepthKind = Depth;
    type StencilKind = Stencil;
    type DepthState<'a> = (&'a DepthStencilStateRef, u32, u32);

    #[inline]
    fn depth_kind(&self) -> &Self::DepthKind {
        &self.0
    }

    #[inline]
    fn stencil_kind(&self) -> &Self::StencilKind {
        &self.1
    }
}

impl DepthStencilKind for (Depth, NoStencil) {
    type DepthKind = Depth;
    type StencilKind = NoStencil;
    type DepthState<'a> = &'a DepthStencilStateRef;

    #[inline]
    fn depth_kind(&self) -> &Self::DepthKind {
        &self.0
    }

    #[inline]
    fn stencil_kind(&self) -> &Self::StencilKind {
        &NoStencil
    }
}

impl DepthStencilKind for (NoDepth, Stencil) {
    type DepthKind = NoDepth;
    type StencilKind = Stencil;
    type DepthState<'a> = (&'a DepthStencilStateRef, u32, u32);

    #[inline]
    fn depth_kind(&self) -> &Self::DepthKind {
        &NoDepth
    }

    #[inline]
    fn stencil_kind(&self) -> &Self::StencilKind {
        &self.1
    }
}

impl DepthStencilKind for (NoDepth, NoStencil) {
    type DepthKind = NoDepth;
    type StencilKind = NoStencil;
    type DepthState<'a> = NoDepthState;

    #[inline]
    fn depth_kind(&self) -> &Self::DepthKind {
        &NoDepth
    }

    #[inline]
    fn stencil_kind(&self) -> &Self::StencilKind {
        &NoStencil
    }
}

pub struct VertexFunctionType;
impl PipelineFunctionType for VertexFunctionType {
    type Descriptor = RenderPipelineDescriptorRef;
    type CommandEncoder = RenderCommandEncoderRef;

    #[inline]
    fn setup_pipeline(func: &FunctionRef, pipeline_desc: &Self::Descriptor) {
        pipeline_desc.set_vertex_function(Some(func));
    }

    #[inline]
    fn bytes<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a RenderCommandEncoderRef,
        index: usize,
        value: &'b [T],
    ) {
        encoder.set_vertex_bytes(
            index as _,
            std::mem::size_of_val(value) as _,
            value.as_ptr() as *const _,
        )
    }
    #[inline]
    fn buffer_and_offset<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a RenderCommandEncoderRef,
        index: usize,
        (buffer, offset): (&'b TypedBuffer<T>, usize),
    ) {
        encoder.set_vertex_buffer(
            index as _,
            Some(&buffer.raw),
            (std::mem::size_of::<T>() * offset) as _,
        );
    }
    #[inline]
    fn buffer_offset<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a RenderCommandEncoderRef,
        index: usize,
        offset: usize,
    ) {
        encoder.set_vertex_buffer_offset(index as _, (std::mem::size_of::<T>() * offset) as _);
    }
    #[inline]
    fn texture<'a, 'b>(
        encoder: &'a RenderCommandEncoderRef,
        index: usize,
        texture: &'b TextureRef,
    ) {
        encoder.set_vertex_texture(index as _, Some(texture));
    }
    #[inline]
    fn texture_null(encoder: &Self::CommandEncoder, index: usize) {
        encoder.set_vertex_texture(index as _, None);
    }
    #[inline]
    fn acceleration_structure<'a, 'b>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        accel_struct: &'b AccelerationStructureRef,
    ) {
        encoder.set_vertex_acceleration_structure(index as _, Some(accel_struct));
    }
    #[inline]
    fn acceleration_structure_null(encoder: &Self::CommandEncoder, index: usize) {
        encoder.set_vertex_acceleration_structure(index as _, None);
    }
}

pub struct FragmentFunctionType;
impl PipelineFunctionType for FragmentFunctionType {
    type Descriptor = RenderPipelineDescriptorRef;
    type CommandEncoder = RenderCommandEncoderRef;

    #[inline]
    fn setup_pipeline(func: &FunctionRef, pipeline_desc: &Self::Descriptor) {
        pipeline_desc.set_fragment_function(Some(func));
    }

    #[inline]
    fn bytes<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        value: &'b [T],
    ) {
        encoder.set_fragment_bytes(
            index as _,
            std::mem::size_of_val(value) as _,
            value.as_ptr() as *const _,
        )
    }
    #[inline]
    fn buffer_and_offset<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        (buffer, offset): (&'b TypedBuffer<T>, usize),
    ) {
        encoder.set_fragment_buffer(
            index as _,
            Some(&buffer.raw),
            (std::mem::size_of::<T>() * offset) as _,
        );
    }
    #[inline]
    fn buffer_offset<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        offset: usize,
    ) {
        encoder.set_fragment_buffer_offset(index as _, (std::mem::size_of::<T>() * offset) as _);
    }
    #[inline]
    fn texture<'a, 'b>(encoder: &'a Self::CommandEncoder, index: usize, texture: &'b TextureRef) {
        encoder.set_fragment_texture(index as _, Some(texture));
    }
    #[inline]
    fn texture_null(encoder: &Self::CommandEncoder, index: usize) {
        encoder.set_fragment_texture(index as _, None);
    }
    #[inline]
    fn acceleration_structure<'a, 'b>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        accel_struct: &'b AccelerationStructureRef,
    ) {
        encoder.set_fragment_acceleration_structure(index as _, Some(accel_struct));
    }
    #[inline]
    fn acceleration_structure_null(encoder: &Self::CommandEncoder, index: usize) {
        encoder.set_fragment_acceleration_structure(index as _, None);
    }
}

pub struct NoBinds;
pub struct NoFragmentFunction;
impl function::Function for NoFragmentFunction {
    const FUNCTION_NAME: &'static str = "<NoFragmentFunction>";
    type Binds<'a> = NoBinds;
}
impl PipelineFunction<FragmentFunctionType> for NoFragmentFunction {
    #[inline]
    fn setup_pipeline(
        &self,
        _library: &LibraryRef,
        _pipeline_desc: &<FragmentFunctionType as PipelineFunctionType>::Descriptor,
    ) {
    }

    #[inline]
    fn bind<'a, 'b>(
        _encoder: &'a <FragmentFunctionType as PipelineFunctionType>::CommandEncoder,
        _binds: Self::Binds<'b>,
    ) {
    }
}

impl Binds for NoBinds {
    const SKIP: Self = Self;

    #[inline]
    fn bind<F: PipelineFunctionType>(self, _: &F::CommandEncoder) {}
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

// TODO: Consider metal-build generating {Render/Compute/Mesh}Pipeline related helpers
// - Currently, users stil need define RenderPipeline<...> and apply the write combination of NUM_COLOR_ATTACHMENTS, FCS, VS, FS
//    - During metal-build binding generation we have enough information to limit combinations
//       - We know FCS is required needed or not.
//       - We know a RenderPipeline is possible or not (is there a VS?)
//          - Depending on FS return type (has `[[color(n)]]`? or just `half4`/`float4`), we know NUM_COLOR_ATTACHMENTS
//          - If there's only one VS... well the user doesn't even need to choose? (overly optimized?)
//       - We know a MeshPipeline (future) is possible or not (is there an Object and Mesh function?)
//       - We know a ComputePipeline (future) is possible or not (is there an Kernel function?)
// - We could go further to solve/mitigate the following...
//   - Use the wrong NUM_COLOR_ATTACHMENTS
//       - ex. FS has `[[color(2)]]`, but NUM_COLOR_ATTACHMENTS is 1
// 1. Sketch out what this would look like
// 2. Does it actually worth the code generation complexity?
pub struct RenderPass<
    'a,
    const NUM_COLOR_ATTACHMENTS: usize,
    V: PipelineFunction<VertexFunctionType>,
    F: PipelineFunction<FragmentFunctionType>,
    DS: DepthStencilKind,
> {
    encoder: &'a RenderCommandEncoderRef,
    _vertex: PhantomData<V>,
    _fragment: PhantomData<F>,
    _depth_stencil: PhantomData<DS>,
}

impl<
        'a,
        const NUM_COLOR_ATTACHMENTS: usize,
        V: PipelineFunction<VertexFunctionType>,
        F: PipelineFunction<FragmentFunctionType>,
        DS: DepthStencilKind,
    > RenderPass<'a, NUM_COLOR_ATTACHMENTS, V, F, DS>
{
    // IMPORTANT: As of writing (7/25/2022), `inline(always)` is very crucial for generating decent
    // code. With only `inline`, the compiler misjudges, doesn't inline and generates a bunch of
    // branches associated with the `match` Bind/BindMany enum variant in `V::bind()`/`F::Bind()`.
    #[inline(always)]
    pub fn bind<'b>(&'a self, vertex_binds: V::Binds<'b>, fragment_binds: F::Binds<'b>) {
        V::bind(self.encoder, vertex_binds);
        F::bind(self.encoder, fragment_binds);
    }

    #[inline]
    pub fn debug_group(&self, label: &str, fun: impl FnOnce()) {
        self.encoder.push_debug_group(label);
        fun();
        self.encoder.pop_debug_group();
    }

    #[inline]
    pub fn draw_primitives<'b>(
        &'a self,
        primitive_type: MTLPrimitiveType,
        vertex_start: usize,
        vertex_count: usize,
    ) {
        self.encoder
            .draw_primitives(primitive_type, vertex_start as _, vertex_count as _);
    }

    #[inline]
    pub fn draw_primitives_with_binds<'b>(
        &'a self,
        vertex_binds: V::Binds<'b>,
        fragment_binds: F::Binds<'b>,
        primitive_type: MTLPrimitiveType,
        vertex_start: usize,
        vertex_count: usize,
    ) {
        self.bind(vertex_binds, fragment_binds);
        self.draw_primitives(primitive_type, vertex_start, vertex_count);
    }

    #[inline]
    pub fn into_subpass<
        'b,
        VNew: PipelineFunction<VertexFunctionType>,
        FNew: PipelineFunction<FragmentFunctionType>,
        PF: FnOnce(RenderPass<'a, NUM_COLOR_ATTACHMENTS, VNew, FNew, DS>),
    >(
        self,
        debug_group: &str,
        subpass_pipeline: &'b RenderPipeline<NUM_COLOR_ATTACHMENTS, VNew, FNew, DS>,
        new_depth_state: Option<DS::DepthState<'b>>,
        fun: PF,
    ) {
        let encoder = self.encoder;
        self.debug_group(debug_group, || {
            encoder.set_render_pipeline_state(&subpass_pipeline.pipeline);
            if let Some(depth_state) = new_depth_state {
                depth_state.setup_render_pass(encoder)
            }
            fun(RenderPass {
                encoder,
                _vertex: PhantomData,
                _fragment: PhantomData,
                _depth_stencil: PhantomData,
            });
        });
    }

    #[inline]
    pub fn into_tesselation_subpass<
        'b,
        VNew: PipelineFunction<VertexFunctionType>,
        FNew: PipelineFunction<FragmentFunctionType>,
        PF: FnOnce(TesselationRenderPass<'a, NUM_COLOR_ATTACHMENTS, VNew, FNew, DS>),
    >(
        self,
        debug_group: &str,
        subpass_pipeline: &'b TesselationRenderPipeline<NUM_COLOR_ATTACHMENTS, VNew, FNew, DS>,
        new_depth_state: Option<DS::DepthState<'b>>,
        new_tesselation_factors_buffer: Option<&'b TypedBuffer<MTLQuadTessellationFactorsHalf>>,
        fun: PF,
    ) {
        let encoder = self.encoder;
        self.debug_group(debug_group, || {
            encoder.set_render_pipeline_state(&subpass_pipeline.pipeline);
            if let Some(depth_state) = new_depth_state {
                depth_state.setup_render_pass(encoder)
            }
            if let Some(buf) = new_tesselation_factors_buffer {
                set_tesselation_factor_buffer(encoder, &buf.raw);
            }
            fun(TesselationRenderPass {
                encoder,
                _vertex: PhantomData,
                _fragment: PhantomData,
                _depth_stencil: PhantomData,
            });
        });
    }

    #[inline]
    pub fn set_depth_stencil_state(&self, ds: DS::DepthState<'_>) {
        ds.setup_render_pass(&self.encoder)
    }
}

pub trait ResourceUsage {
    fn use_resource<'b>(&self, encoder: &'b RenderCommandEncoderRef);
}

#[derive(Copy, Clone)]
pub struct BufferUsage<'a, T: Sized + Copy + Clone>(
    pub &'a TypedBuffer<T>,
    pub MTLResourceUsage,
    pub MTLRenderStages,
);
impl<T: Sized + Copy + Clone> ResourceUsage for BufferUsage<'_, T> {
    #[inline]
    fn use_resource<'b>(&self, encoder: &RenderCommandEncoderRef) {
        encoder.use_resource_at(&self.0.raw, self.1, self.2)
    }
}

#[derive(Copy, Clone)]
pub struct HeapUsage<'a>(pub &'a HeapRef, pub MTLRenderStages);
impl ResourceUsage for HeapUsage<'_> {
    #[inline]
    fn use_resource<'b>(&self, encoder: &RenderCommandEncoderRef) {
        encoder.use_heap_at(self.0, self.1)
    }
}

#[derive(Copy, Clone)]
pub struct TextureUsage<'a>(
    pub &'a TextureRef,
    pub MTLResourceUsage,
    pub MTLRenderStages,
);
impl ResourceUsage for TextureUsage<'_> {
    #[inline]
    fn use_resource<'b>(&self, encoder: &RenderCommandEncoderRef) {
        encoder.use_resource_at(&self.0, self.1, self.2)
    }
}

pub struct RenderPipeline<
    const NUM_COLOR_ATTACHMENTS: usize,
    V: PipelineFunction<VertexFunctionType>,
    F: PipelineFunction<FragmentFunctionType>,
    DS: DepthStencilKind,
> {
    pub pipeline: RenderPipelineState,
    _vertex_function: PhantomData<V>,
    _fragment_function: PhantomData<F>,
    _depth_stencil_kind: PhantomData<DS>,
}

impl<
        const NUM_COLOR_ATTACHMENTS: usize,
        V: PipelineFunction<VertexFunctionType>,
        F: PipelineFunction<FragmentFunctionType>,
        DS: DepthStencilKind,
    > RenderPipeline<NUM_COLOR_ATTACHMENTS, V, F, DS>
{
    pub fn new(
        label: &str,
        device: &DeviceRef,
        library: &LibraryRef,
        colors: [ColorPipelineDesc; NUM_COLOR_ATTACHMENTS],
        vertex_function: V,
        fragment_function: F,
        depth_stencil_kind: DS,
    ) -> Self {
        debug_time("RenderPipeline", || {
            let pipeline_desc = RenderPipelineDescriptor::new();
            pipeline_desc.set_label(label);

            for i in 0..NUM_COLOR_ATTACHMENTS {
                let desc = pipeline_desc
                    .color_attachments()
                    .object_at(i as u64)
                    .expect("Failed to access color attachment on pipeline descriptor");
                Color::setup_pipeline(colors[i], &desc);
            }
            depth_stencil_kind.setup_pipeline(&pipeline_desc);
            vertex_function.setup_pipeline(library, &pipeline_desc);
            fragment_function.setup_pipeline(library, &pipeline_desc);
            let pipeline = device
                .new_render_pipeline_state(&pipeline_desc)
                .expect("Failed to create pipeline state");
            Self {
                pipeline,
                _vertex_function: PhantomData,
                _fragment_function: PhantomData,
                _depth_stencil_kind: PhantomData,
            }
        })
    }

    #[inline]
    pub fn new_pass<'a, 'b, 'c, PF: FnOnce(RenderPass<'c, NUM_COLOR_ATTACHMENTS, V, F, DS>)>(
        &'a self,
        label: &'static str,
        command_buffer: &'a CommandBufferRef,
        color_attachments: [ColorRenderPassDesc; NUM_COLOR_ATTACHMENTS],
        depth_attachment: <DS::DepthKind as DepthKind>::RenderPassDesc<'b>,
        stencil_attachment: <DS::StencilKind as StencilKind>::RenderPassDesc<'b>,
        depth_state: DS::DepthState<'b>,
        resources: &[&dyn ResourceUsage],
        fun: PF,
    ) where
        'a: 'c,
    {
        let desc = RenderPassDescriptor::new();
        for i in 0..NUM_COLOR_ATTACHMENTS {
            let c = color_attachments[i];
            let a = desc
                .color_attachments()
                .object_at(i as _)
                .expect("Failed to access color attachment on render pass descriptor");
            Color::setup_render_pass(c, a);
        }
        DS::setup_render_pass(depth_attachment, stencil_attachment, desc);
        let encoder = command_buffer.new_render_command_encoder(desc);
        encoder.set_label(label);
        for r in resources {
            r.use_resource(encoder)
        }
        encoder.set_render_pipeline_state(&self.pipeline);
        depth_state.setup_render_pass(encoder);
        fun(RenderPass {
            encoder,
            _vertex: PhantomData,
            _fragment: PhantomData,
            _depth_stencil: PhantomData,
        });
        encoder.end_encoding();
    }
}
