use super::*;
use crate::{
    debug_time, draw_patches, set_tesselation_factor_buffer, set_tessellation_config,
    typed_buffer::TypedBuffer,
};
use metal::{
    CommandBufferRef, DeviceRef, LibraryRef, MTLTriangleFillMode, RenderCommandEncoderRef,
    RenderPassDescriptor, RenderPipelineDescriptor, RenderPipelineState,
};
use metal_types::MTLQuadTessellationFactorsHalf;
use std::marker::PhantomData;

pub struct TesselationRenderPass<
    'a,
    const NUM_COLOR_ATTACHMENTS: usize,
    V: PipelineFunction<VertexFunctionType>,
    F: PipelineFunction<FragmentFunctionType>,
    DS: DepthStencilKind,
> {
    pub(crate) encoder: &'a RenderCommandEncoderRef,
    pub(crate) _vertex: PhantomData<V>,
    pub(crate) _fragment: PhantomData<F>,
    pub(crate) _depth_stencil: PhantomData<DS>,
}

impl<
        'a,
        const NUM_COLOR_ATTACHMENTS: usize,
        V: PipelineFunction<VertexFunctionType>,
        F: PipelineFunction<FragmentFunctionType>,
        DS: DepthStencilKind,
    > TesselationRenderPass<'a, NUM_COLOR_ATTACHMENTS, V, F, DS>
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
    pub fn draw_patches<'b>(&'a self, number_of_patch_control_points: usize) {
        draw_patches(self.encoder, number_of_patch_control_points as _);
    }

    #[inline]
    pub fn draw_patches_with_bind<'b>(
        &'a self,
        vertex_binds: V::Binds<'b>,
        fragment_binds: F::Binds<'b>,
        number_of_patch_control_points: usize,
    ) {
        self.bind(vertex_binds, fragment_binds);
        self.draw_patches(number_of_patch_control_points);
    }

    // TODO: Implement into_subpass
    // - This may require bigger refactoring to handle RenderPass <-> TesselationRenderPass transitions
    //    - OR... we don't try to over engineer a general solution...
    //       - Maybe this is the only "different type"?
    //       - Mesh shaders makes this all obsolete, yah?
    // - For example `render_pass.into_subpass(..., tesselation_pipeline, ???)`
    //    - Problem: Creating a tesselation pass needs *more* parameters, in the case the tesselation factors buffer.
    // - For example `tesselation_pass.into_subpass(..., render_pipeline, ???)`
    //    - Problem: We need less parameters... no need for a tesselation factors buffer.
    // - Maybe there's better abstraction around both render and tesselation pipelines.
    //    - I'd hate to add another param to RenderPipeline, especially if most cases it will be unused.

    #[inline]
    pub fn set_depth_stencil_state(&self, ds: DS::DepthState<'_>) {
        ds.setup_render_pass(&self.encoder)
    }

    #[inline]
    pub fn set_triangle_fill_mode(&self, mode: MTLTriangleFillMode) {
        self.encoder.set_triangle_fill_mode(mode)
    }
}

pub struct TesselationRenderPipeline<
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
    > TesselationRenderPipeline<NUM_COLOR_ATTACHMENTS, V, F, DS>
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
        debug_time("TesselationRenderPipeline", || {
            let mut pipeline_desc = RenderPipelineDescriptor::new();
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
            set_tessellation_config(&mut pipeline_desc);
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
    pub fn new_pass<
        'a,
        'b,
        'c,
        PF: FnOnce(TesselationRenderPass<'c, NUM_COLOR_ATTACHMENTS, V, F, DS>),
    >(
        &'a self,
        label: &'static str,
        command_buffer: &'a CommandBufferRef,
        color_attachments: [ColorRenderPassDesc; NUM_COLOR_ATTACHMENTS],
        depth_attachment: <DS::DepthKind as DepthKind>::RenderPassDesc<'b>,
        stencil_attachment: <DS::StencilKind as StencilKind>::RenderPassDesc<'b>,
        tesselation_factor_buffer: &TypedBuffer<MTLQuadTessellationFactorsHalf>,
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
        set_tesselation_factor_buffer(encoder, &tesselation_factor_buffer.raw);
        fun(TesselationRenderPass {
            encoder,
            _vertex: PhantomData,
            _fragment: PhantomData,
            _depth_stencil: PhantomData,
        });
        encoder.end_encoding();
    }
}
