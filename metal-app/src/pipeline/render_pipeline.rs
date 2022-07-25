use super::{bind::Binds, function, pipeline_function::*};
use crate::{debug_time, typed_buffer::TypedBuffer};
use metal::*;
use std::marker::PhantomData;

// TODO: START HERE 2
// TODO: START HERE 2
// TODO: START HERE 2
// Create a TypedTexture, to enforce the pipeline and the render pass have the same pixel format.

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// RenderPipeline::binds() generates terrible code
// - Looking at the asm, every `match` on a Bind* enum... generates branches.
// - It was assumed some inlining and constant propagation would be sufficent...
// - Consider switching from an enum to a type for each variant (bytes, buffer/offset, buffer offset, skip)
// 0. Stash changes
// 1. Take a test example (ex. Vertex1/Fragment1) and create metal-app main()
// 2. Commit
// 3. Generate asm before
// 4. Reapply stashed changes
// 5. Adapt test example
// 6. Generate asm after
// 7. asm diff and assess code generated is much better
// 8. Update generate_rust_bindings

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
    #[inline(always)]
    fn setup_pipeline_attachment<'a>(
        desc: ColorAttachementPipelineDesc,
        pass: &RenderPipelineColorAttachmentDescriptorRef,
    ) {
        let (pixel_format, blend_mode) = desc;
        pass.set_pixel_format(pixel_format);
        pass.set_blending_enabled(matches!(blend_mode, BlendMode::Blend));
    }

    #[inline(always)]
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

    #[inline(always)]
    fn setup_pipeline_attachment(&self, _pipeline_descriptor: &RenderPipelineDescriptorRef) {}
    #[inline(always)]
    fn setup_render_pass_attachment<'a>(
        _desc: Self::RenderPassDesc<'a>,
        _pass: &RenderPassDescriptorRef,
    ) {
    }
}
pub struct HasDepth(pub MTLPixelFormat);
impl DepthAttachmentKind for HasDepth {
    type RenderPassDesc<'a> = (&'a TextureRef, f32, MTLLoadAction, MTLStoreAction);

    #[inline(always)]
    fn setup_pipeline_attachment(&self, pipeline_descriptor: &RenderPipelineDescriptorRef) {
        pipeline_descriptor.set_depth_attachment_pixel_format(self.0);
    }

    #[inline(always)]
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
    // TODO: START HERE
    // TODO: START HERE
    // TODO: START HERE
    // 1. This should include DepthState
    // 2. This trait should include a setup_render_pass/setup_render_encoder
    type RenderPassDesc<'a> = (&'a TextureRef, u32, MTLLoadAction, MTLStoreAction);

    #[inline(always)]
    fn setup_pipeline_attachment(&self, pipeline_descriptor: &RenderPipelineDescriptorRef) {
        pipeline_descriptor.set_stencil_attachment_pixel_format(self.0);
    }

    #[inline(always)]
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

    #[inline(always)]
    fn setup_pipeline_attachment(&self, _pipeline_descriptor: &RenderPipelineDescriptorRef) {}

    #[inline(always)]
    fn setup_render_pass_attachment<'a>(
        _desc: Self::RenderPassDesc<'a>,
        _pass: &RenderPassDescriptorRef,
    ) {
    }
}

pub struct VertexFunctionType;
impl PipelineFunctionType for VertexFunctionType {
    type Descriptor = RenderPipelineDescriptorRef;
    type CommandEncoder = RenderCommandEncoderRef;

    #[inline(always)]
    fn setup_pipeline(func: &FunctionRef, pipeline_desc: &Self::Descriptor) {
        pipeline_desc.set_vertex_function(Some(&func));
    }

    #[inline(always)]
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
    #[inline(always)]
    fn buffer_and_offset<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a RenderCommandEncoderRef,
        index: usize,
        (buffer, offset): (&'b TypedBuffer<T>, usize),
    ) {
        encoder.set_vertex_buffer(
            index as _,
            Some(&buffer.buffer),
            (std::mem::size_of::<T>() * offset) as _,
        );
    }
    #[inline(always)]
    fn buffer_offset<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a RenderCommandEncoderRef,
        index: usize,
        offset: usize,
    ) {
        encoder.set_vertex_buffer_offset(index as _, (std::mem::size_of::<T>() * offset) as _);
    }
    #[inline(always)]
    fn texture<'a, 'b>(
        encoder: &'a RenderCommandEncoderRef,
        index: usize,
        texture: &'b TextureRef,
    ) {
        encoder.set_vertex_texture(index as _, Some(texture));
    }
}

pub struct FragmentFunctionType;
impl PipelineFunctionType for FragmentFunctionType {
    type Descriptor = RenderPipelineDescriptorRef;
    type CommandEncoder = RenderCommandEncoderRef;

    #[inline(always)]
    fn setup_pipeline(func: &FunctionRef, pipeline_desc: &Self::Descriptor) {
        pipeline_desc.set_fragment_function(Some(&func));
    }

    #[inline(always)]
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
    #[inline(always)]
    fn buffer_and_offset<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        (buffer, offset): (&'b TypedBuffer<T>, usize),
    ) {
        encoder.set_fragment_buffer(
            index as _,
            Some(&buffer.buffer),
            (std::mem::size_of::<T>() * offset) as _,
        );
    }
    #[inline(always)]
    fn buffer_offset<'a, 'b, T: Sized + Copy + Clone>(
        encoder: &'a Self::CommandEncoder,
        index: usize,
        offset: usize,
    ) {
        encoder.set_fragment_buffer_offset(index as _, (std::mem::size_of::<T>() * offset) as _);
    }
    #[inline(always)]
    fn texture<'a, 'b>(encoder: &'a Self::CommandEncoder, index: usize, texture: &'b TextureRef) {
        encoder.set_fragment_texture(index as _, Some(texture));
    }
}

pub struct NoBinds;
pub struct NoFragmentFunction;
impl function::Function for NoFragmentFunction {
    const FUNCTION_NAME: &'static str = "<NoFragmentShader>";
    type Binds<'a> = NoBinds;
}
impl PipelineFunction<FragmentFunctionType> for NoFragmentFunction {}

impl Binds for NoBinds {
    #[inline(always)]
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

pub struct RenderPass<
    'a,
    VF: PipelineFunction<VertexFunctionType>,
    FF: PipelineFunction<FragmentFunctionType>,
> {
    pub encoder: &'a RenderCommandEncoderRef,
    pub _vertex: PhantomData<VF>,
    pub _fragment: PhantomData<FF>,
}

impl<'a, VF: PipelineFunction<VertexFunctionType>, FF: PipelineFunction<FragmentFunctionType>>
    RenderPass<'a, VF, FF>
{
    #[inline(always)]
    pub fn bind<'b>(&'a self, vertex_binds: VF::Binds<'b>, fragment_binds: FF::Binds<'b>) {
        VF::bind(self.encoder, vertex_binds);
        FF::bind(self.encoder, fragment_binds);
    }
}

pub struct RenderPipeline<
    const NUM_COLOR_ATTACHMENTS: usize,
    VS: PipelineFunction<VertexFunctionType>,
    FS: PipelineFunction<FragmentFunctionType>,
    D: DepthAttachmentKind,
    S: StencilAttachmentKind,
> {
    pub pipeline: RenderPipelineState,
    _vertex_function: PhantomData<VS>,
    _fragment_function: PhantomData<FS>,
    _depth_kind: PhantomData<D>,
    _stencil_kind: PhantomData<S>,
}

impl<
        const NUM_COLOR_ATTACHMENTS: usize,
        VS: PipelineFunction<VertexFunctionType>,
        FS: PipelineFunction<FragmentFunctionType>,
        D: DepthAttachmentKind,
        S: StencilAttachmentKind,
    > RenderPipeline<NUM_COLOR_ATTACHMENTS, VS, FS, D, S>
{
    pub fn new(
        label: &str,
        device: &DeviceRef,
        library: &LibraryRef,
        colors: [ColorAttachementPipelineDesc; NUM_COLOR_ATTACHMENTS],
        vertex_function: VS,
        fragment_function: FS,
        depth_kind: D,
        stencil_kind: S,
    ) -> Self {
        debug_time("RenderPipeline", || {
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
            vertex_function.setup_pipeline(library, &pipeline_desc);
            fragment_function.setup_pipeline(library, &pipeline_desc);
            let pipeline = device
                .new_render_pipeline_state(&pipeline_desc)
                .expect("Failed to create pipeline state");
            Self {
                pipeline,
                _vertex_function: PhantomData,
                _fragment_function: PhantomData,
                _depth_kind: PhantomData,
                _stencil_kind: PhantomData,
            }
        })
    }

    // TODO: START HERE
    // TODO: START HERE
    // TODO: START HERE
    // This should include:
    // 1. vertex/fragment binds
    // 2. #[inline(always)] to make sure binds have perfect/near-perfect codegen
    // 3. A slice/array of Resource's (new trait) to power use_resource/use_heap_at/etc.
    pub fn new_pass<'a, 'b, 'c>(
        &self,
        label: &'static str,
        command_buffer: &'a CommandBufferRef,
        color_attachments: [ColorAttachementRenderPassDesc; NUM_COLOR_ATTACHMENTS],
        depth_attachment: D::RenderPassDesc<'b>,
        stencil_attachment: S::RenderPassDesc<'c>,
    ) -> RenderPass<'a, VS, FS> {
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
        RenderPass {
            encoder,
            _vertex: PhantomData,
            _fragment: PhantomData,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        pipeline::{bind::*, compute_pipeline::ComputeFunctionType},
        typed_buffer::TypedBuffer,
    };
    use metal_types::*;
    use std::simd::f32x4;

    // Assumed to be generated by metal-build (shader_function_parser/generate_shader_binds)
    struct Vertex1Binds<'a> {
        v_bind1: BindMany<'a, f32>,
    }
    impl<'a> Binds for Vertex1Binds<'a> {
        fn bind<F: PipelineFunctionType>(self, encoder: &F::CommandEncoder) {
            self.v_bind1.bind::<F>(encoder, 0);
        }
    }
    struct Vertex1 {
        function_constant_1: bool,
    }
    impl function::Function for Vertex1 {
        const FUNCTION_NAME: &'static str = "vertex1";
        type Binds<'a> = Vertex1Binds<'a>;
        fn get_function_constants(&self) -> Option<FunctionConstantValues> {
            let fcv = FunctionConstantValues::new();
            fcv.set_constant_value_at_index(
                (&self.function_constant_1 as *const _) as _,
                bool::MTL_DATA_TYPE,
                1,
            );
            Some(fcv)
        }
    }
    impl PipelineFunction<VertexFunctionType> for Vertex1 {}

    // TEST
    // TEST
    // TEST

    #[derive(Clone, Copy)]
    pub struct TestStruct {
        yolo: float,
    }
    #[derive(Clone, Copy)]
    pub struct float3 {
        yolo: float,
    }

    #[allow(non_camel_case_types)]
    pub struct test_vertex_binds<'a> {
        pub buf0: BindMany<'a, float>,
        pub buf1: Bind<'a, float2>,
        pub buf2: BindMany<'a, float3>,
        pub buf3: Bind<'a, float3>,
        pub tex1: BindTexture<'a>,
        pub buf5: Bind<'a, TestStruct>,
        pub buf4: BindMany<'a, TestStruct>,
    }
    impl<'a> Binds for test_vertex_binds<'a> {
        fn bind<F: PipelineFunctionType>(self, e: &F::CommandEncoder) {
            self.buf0.bind::<F>(e, 0);
            self.buf1.bind::<F>(e, 1);
            self.buf2.bind::<F>(e, 2);
            self.buf3.bind::<F>(e, 3);
            self.tex1.bind::<F>(e, 1);
            self.buf5.bind::<F>(e, 5);
            self.buf4.bind::<F>(e, 4);
        }
    }
    // TEST
    // TEST
    // TEST

    // Assumed to be generated by metal-build (shader_function_parser/generate_shader_binds)
    struct Frag1Binds<'a> {
        f_bind1: Bind<'a, float4>,
        f_tex2: BindTexture<'a>,
    }
    struct Fragment1Binder<'a, F: PipelineFunctionType>(&'a F::CommandEncoder);
    impl<'a> Binds for Frag1Binds<'a> {
        fn bind<F: PipelineFunctionType>(self, e: &F::CommandEncoder) {
            self.f_bind1.bind::<F>(e, 0);
        }
    }
    struct Fragment1 {
        function_constant_2: bool,
    }
    impl function::Function for Fragment1 {
        const FUNCTION_NAME: &'static str = "fragment1";
        type Binds<'a> = Frag1Binds<'a>;
        fn get_function_constants(&self) -> Option<FunctionConstantValues> {
            let fcv = FunctionConstantValues::new();
            fcv.set_constant_value_at_index(
                (&self.function_constant_2 as *const _) as _,
                bool::MTL_DATA_TYPE,
                1,
            );
            Some(fcv)
        }
    }
    impl PipelineFunction<FragmentFunctionType> for Fragment1 {}
    // TODO: Just an example of function being able to be used as 2 types.
    // For example, tile render pipeline function vs compute function. Both are `[[kernel]]` in
    // Metal but can be applied to 2 different types of pipelines/command encoders. We won't know
    // which way is applicable, so we need to allow function/binds to be used in either way.
    impl PipelineFunction<ComputeFunctionType> for Fragment1 {}

    // Assumed to be generated by metal-build (shader_function_parser/generate_shader_binds)
    struct Vertex2NoFunctionConstants;
    impl function::Function for Vertex2NoFunctionConstants {
        const FUNCTION_NAME: &'static str = "vertex1";
        type Binds<'a> = Vertex1Binds<'a>;
    }
    impl PipelineFunction<VertexFunctionType> for Vertex2NoFunctionConstants {}

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
            let p: RenderPipeline<1, Vertex1, Fragment1, NoDepth, NoStencil> = RenderPipeline::new(
                "Test",
                &device,
                &lib,
                [(MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend)],
                Vertex1 {
                    function_constant_1: true,
                },
                Fragment1 {
                    function_constant_2: true,
                },
                NoDepth,
                NoStencil,
            );
            let pass = p.new_pass(
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
            pass.bind(
                Vertex1Binds {
                    v_bind1: BindMany::Values(&[0_f32]),
                },
                Frag1Binds {
                    f_bind1: Bind::Value(&f32x4::splat(0.).into()),
                    f_tex2: BindTexture(&texture),
                },
            );
            pass.bind(
                Vertex1Binds {
                    v_bind1: BindMany::Buffer(BindBuffer::WithOffset(&f32_buffer, 0)),
                },
                Frag1Binds {
                    f_bind1: Bind::Buffer(BindBuffer::WithOffset(&float4_buffer, 0)),
                    f_tex2: BindTexture(&texture),
                },
            );
        }
        {
            let p: RenderPipeline<2, Vertex1, Fragment1, NoDepth, NoStencil> = RenderPipeline::new(
                "Test",
                &device,
                &lib,
                [
                    (MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend),
                    (MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend),
                ],
                Vertex1 {
                    function_constant_1: true,
                },
                Fragment1 {
                    function_constant_2: true,
                },
                NoDepth,
                NoStencil,
            );
            let pass = p.new_pass(
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
            pass.bind(
                Vertex1Binds {
                    v_bind1: BindMany::Values(&[0.]),
                },
                Frag1Binds {
                    f_bind1: Bind::Value(&f32x4::splat(1.).into()),
                    f_tex2: BindTexture(&texture),
                },
            );
        }
        {
            let p: RenderPipeline<
                1,
                Vertex2NoFunctionConstants,
                NoFragmentFunction,
                HasDepth,
                HasStencil,
            > = RenderPipeline::new(
                "Test",
                &device,
                &lib,
                [(MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend)],
                Vertex2NoFunctionConstants,
                NoFragmentFunction,
                HasDepth(MTLPixelFormat::Depth16Unorm),
                HasStencil(MTLPixelFormat::Stencil8),
            );
            let pass = p.new_pass(
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
            pass.bind(
                Vertex1Binds {
                    v_bind1: BindMany::Values(&[0.]),
                },
                NoBinds,
            );
        }
    }
}
