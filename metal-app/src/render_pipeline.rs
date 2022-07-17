use metal::{
    Buffer, CommandBufferRef, DeviceRef, HeapRef, LibraryRef, MTLClearColor, MTLLoadAction,
    MTLPixelFormat, MTLResourceOptions, MTLStoreAction, RenderCommandEncoderRef,
    RenderPassColorAttachmentDescriptorRef, RenderPassDescriptor, RenderPassDescriptorRef,
    RenderPipelineColorAttachmentDescriptorRef, RenderPipelineDescriptor,
    RenderPipelineDescriptorRef, RenderPipelineState, Texture, TextureRef,
};
use std::marker::PhantomData;

pub trait MetalBufferAllocator {
    fn with_capacity<T: Sized>(&self, capacity: usize, options: MTLResourceOptions) -> Buffer;
}

impl MetalBufferAllocator for &DeviceRef {
    fn with_capacity<T: Sized>(&self, capacity: usize, options: MTLResourceOptions) -> Buffer {
        self.new_buffer((std::mem::size_of::<T>() * capacity) as _, options)
    }
}

impl MetalBufferAllocator for &HeapRef {
    fn with_capacity<T: Sized>(&self, capacity: usize, options: MTLResourceOptions) -> Buffer {
        self.new_buffer((std::mem::size_of::<T>() * capacity) as _, options)
            .expect("Failed to heap allocate buffer")
    }
}

// TODO: Consider a TypedTexture

// TODO: Move into it's own file
pub struct TypedBuffer<T: Sized> {
    buffer: Buffer,
    _type: PhantomData<T>,
}

impl<T: Sized> TypedBuffer<T> {
    #[inline]
    fn with_capacity<A: MetalBufferAllocator>(
        allocator: A,
        capacity: usize,
        options: MTLResourceOptions,
    ) -> Self {
        let buffer = allocator.with_capacity::<T>(capacity, options);
        Self {
            buffer,
            _type: PhantomData,
        }
    }

    #[inline]
    fn from_data<A: MetalBufferAllocator>(
        allocator: A,
        data: &[T],
        options: MTLResourceOptions,
    ) -> Self {
        let tb = Self::with_capacity(allocator, data.len(), options);
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), tb.buffer.contents() as *mut T, data.len())
        };
        tb
    }
    // TODO: Add update_data or get_mut function
}

pub enum BindMany<'a, const BUFFER_INDEX: u64, T: Sized> {
    Bytes(&'a [T]),
    BufferAndOffset(&'a TypedBuffer<T>, usize),
    BufferOffset(usize),
}
pub enum BindOne<'a, const BUFFER_INDEX: u64, T: Sized> {
    Bytes(&'a T),
    BufferAndOffset(&'a TypedBuffer<T>, usize),
    BufferOffset(usize),
}
impl<'a, const BUFFER_INDEX: u64, T: Sized> BindMany<'a, BUFFER_INDEX, T> {
    #[inline]
    pub fn encode_for_vertex<'b>(&self, encoder: &'b RenderCommandEncoderRef) {
        match self {
            &BindMany::Bytes(v) => encoder.set_vertex_bytes(
                BUFFER_INDEX,
                (std::mem::size_of::<T>() * v.len()) as _,
                v.as_ptr() as *const _,
            ),
            &BindMany::BufferAndOffset(tb, o) => {
                encoder.set_vertex_buffer(BUFFER_INDEX, Some(&tb.buffer), o as _)
            }
            &BindMany::BufferOffset(o) => encoder.set_vertex_buffer_offset(BUFFER_INDEX, o as _),
        }
    }
    #[inline]
    pub fn encode_for_fragment<'b>(&self, encoder: &'b RenderCommandEncoderRef) {
        match self {
            &BindMany::Bytes(v) => encoder.set_fragment_bytes(
                BUFFER_INDEX,
                (std::mem::size_of::<T>() * v.len()) as _,
                v.as_ptr() as *const _,
            ),
            &BindMany::BufferAndOffset(tb, o) => {
                encoder.set_fragment_buffer(BUFFER_INDEX, Some(&tb.buffer), o as _)
            }
            &BindMany::BufferOffset(o) => encoder.set_fragment_buffer_offset(BUFFER_INDEX, o as _),
        }
    }
}
impl<'a, const BUFFER_INDEX: u64, T: Sized> BindOne<'a, BUFFER_INDEX, T> {
    #[inline]
    pub fn encode_for_vertex<'b>(&self, encoder: &'b RenderCommandEncoderRef) {
        match self {
            &BindOne::Bytes(v) => encoder.set_vertex_bytes(
                BUFFER_INDEX,
                std::mem::size_of::<T>() as _,
                (v as *const T) as _,
            ),
            &BindOne::BufferAndOffset(tb, o) => {
                encoder.set_vertex_buffer(BUFFER_INDEX, Some(&tb.buffer), o as _)
            }
            &BindOne::BufferOffset(o) => encoder.set_vertex_buffer_offset(BUFFER_INDEX, o as _),
        }
    }
    #[inline]
    pub fn encode_for_fragment<'b>(&self, encoder: &'b RenderCommandEncoderRef) {
        match self {
            &BindOne::Bytes(v) => encoder.set_fragment_bytes(
                BUFFER_INDEX,
                std::mem::size_of::<T>() as _,
                (v as *const T) as _,
            ),
            &BindOne::BufferAndOffset(tb, o) => {
                encoder.set_fragment_buffer(BUFFER_INDEX, Some(&tb.buffer), o as _)
            }
            &BindOne::BufferOffset(o) => encoder.set_fragment_buffer_offset(BUFFER_INDEX, o as _),
        }
    }
}

pub struct BindTexture<'a, const TEXTURE_INDEX: u64>(&'a Texture);
impl<'a, const TEXTURE_INDEX: u64> BindTexture<'a, TEXTURE_INDEX> {
    #[inline]
    pub fn encode_for_vertex<'b>(&self, encoder: &'b RenderCommandEncoderRef) {
        encoder.set_vertex_texture(TEXTURE_INDEX, Some(self.0));
    }

    #[inline]
    pub fn encode_for_fragment<'b>(&self, encoder: &'b RenderCommandEncoderRef) {
        encoder.set_fragment_texture(TEXTURE_INDEX, Some(self.0));
    }

    // TODO: Figure out a way to make this work.
    // - How do we type ??? (texture index const generic)
    // - Do we have to remove the generic and have the caller store texture index for this work? :/
    //
    // #[inline]
    // fn encode_many_for_vertex<'b, 'c>(
    //     encoder: &'b RenderCommandEncoderRef,
    //     start_index: usize,
    //     binds: &[BindTexture<'c, ???>],
    // ) {
    //     encoder.set_vertex_textures(
    //         start_index as _,
    //         &binds
    //             .iter()
    //             .map(|a| Some(a.0.deref()))
    //             .collect::<Vec<Option<&TextureRef>>>(),
    //     );
    // }
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
pub struct HasDepth(MTLPixelFormat);
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
pub struct HasStencil(MTLPixelFormat);
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

pub trait VertexShaderBinds {
    fn encode_vertex_binds<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef);
}

pub trait FragmentShaderBinds {
    fn encode_fragment_binds<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef);
}

pub struct NoBinds;
impl VertexShaderBinds for NoBinds {
    #[inline]
    fn encode_vertex_binds<'a, 'b>(&'a self, _encoder: &'b RenderCommandEncoderRef) {}
}
impl FragmentShaderBinds for NoBinds {
    #[inline]
    fn encode_fragment_binds<'a, 'b>(&'a self, _encoder: &'b RenderCommandEncoderRef) {}
}

pub trait VertexShader {
    type Binds<'a>: VertexShaderBinds;

    #[inline]
    fn setup_pipeline_vertex_function(
        &self,
        lib: &LibraryRef,
        pipeline_desc: &RenderPipelineDescriptorRef,
    ) {
        let func = lib
            .get_function(Self::function_name(), None)
            .expect("Failed to get vertex function from library");
        pipeline_desc.set_vertex_function(Some(&func));
    }

    fn function_name() -> &'static str;
}

pub trait FragmentShader {
    type Binds<'a>: FragmentShaderBinds;

    #[inline]
    fn setup_pipeline_fragment_function(
        &self,
        lib: &LibraryRef,
        pipeline_desc: &RenderPipelineDescriptorRef,
    ) {
        let func = lib
            .get_function(Self::function_name(), None)
            .expect("Failed to get vertex function from library");
        pipeline_desc.set_fragment_function(Some(&func));
    }

    fn function_name() -> &'static str;
}

pub struct NoFragmentShader;
impl FragmentShader for NoFragmentShader {
    type Binds<'a> = NoBinds;

    #[inline]
    fn setup_pipeline_fragment_function(
        &self,
        _lib: &LibraryRef,
        _pipeline_desc: &RenderPipelineDescriptorRef,
    ) {
    }

    #[inline]
    fn function_name() -> &'static str {
        ""
    }
}

pub struct RenderPipeline<
    const NUM_COLOR_ATTACHMENTS: usize,
    VS: VertexShader,
    FS: FragmentShader,
    D: DepthAttachmentKind,
    S: StencilAttachmentKind,
> {
    pipeline: RenderPipelineState,
    _depth_kind: PhantomData<D>,
    _stencil_kind: PhantomData<S>,
    _vertex_fn: PhantomData<VS>,
    _fragment_fn: PhantomData<FS>,
}

impl<
        const NUM_COLOR_ATTACHMENTS: usize,
        VS: VertexShader,
        FS: FragmentShader,
        D: DepthAttachmentKind,
        S: StencilAttachmentKind,
    > RenderPipeline<NUM_COLOR_ATTACHMENTS, VS, FS, D, S>
{
    pub fn new(
        label: &str,
        device: &DeviceRef,
        library: &LibraryRef,
        colors: [ColorAttachementPipelineDesc; NUM_COLOR_ATTACHMENTS],
        vertex_fn: VS,
        fragment_fn: FS,
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
        vertex_fn.setup_pipeline_vertex_function(library, &pipeline_desc);
        fragment_fn.setup_pipeline_fragment_function(library, &pipeline_desc);

        // TODO: START HERE 2
        // TODO: START HERE 2
        // TODO: START HERE 2
        // Add back non-release profile bind checking

        let pipeline = device
            .new_render_pipeline_state(&pipeline_desc)
            .expect("Failed to create pipeline state");
        Self {
            pipeline,
            _vertex_fn: PhantomData,
            _fragment_fn: PhantomData,
            _depth_kind: PhantomData,
            _stencil_kind: PhantomData,
        }
    }

    pub fn new_render_command_encoder<'a, 'b, 'c>(
        &self,
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
        command_buffer.new_render_command_encoder(desc)
    }

    pub fn setup<'a, 'b, 'c>(
        &'a self,
        encoder: &RenderCommandEncoderRef,
        vertex_binds: VS::Binds<'b>,
        fragment_binds: FS::Binds<'c>,
    ) {
        vertex_binds.encode_vertex_binds(encoder);
        fragment_binds.encode_fragment_binds(encoder);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use metal::{Device, TextureDescriptor};
    use metal_types::float4;
    use std::simd::f32x4;

    // TODO: START HERE
    // TODO: START HERE
    // TODO: START HERE
    // DOOOO ITTTT! Parse. Shader. Functions.

    // Assumed to be generated by metal-build (shader_function_parser/generate_shader_binds)
    struct Vertex1Binds<'a> {
        v_bind1: BindMany<'a, 0, f32>,
    }
    impl<'c> VertexShaderBinds for Vertex1Binds<'c> {
        #[inline]
        fn encode_vertex_binds<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef) {
            self.v_bind1.encode_for_vertex(encoder);
        }
    }
    struct Vertex1;
    impl VertexShader for Vertex1 {
        type Binds<'a> = Vertex1Binds<'a>;

        #[inline]
        fn function_name() -> &'static str {
            "vertex1"
        }
    }

    // Assumed to be generated by metal-build (shader_function_parser/generate_shader_binds)
    struct Frag1Binds<'a> {
        f_bind1: BindOne<'a, 0, float4>,
    }
    impl<'c> FragmentShaderBinds for Frag1Binds<'c> {
        #[inline]
        fn encode_fragment_binds<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef) {
            self.f_bind1.encode_for_fragment(encoder);
        }
    }
    struct Fragment1;
    impl FragmentShader for Fragment1 {
        type Binds<'a> = Frag1Binds<'a>;

        #[inline]
        fn function_name() -> &'static str {
            "fragment1"
        }
    }

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
            &device as &DeviceRef,
            1,
            MTLResourceOptions::StorageModeManaged,
        );
        let float4_buffer = TypedBuffer::<float4>::with_capacity(
            &device as &DeviceRef,
            1,
            MTLResourceOptions::StorageModeManaged,
        );

        {
            let p = RenderPipeline::new(
                "Test",
                &device,
                &lib,
                [(MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend)],
                Vertex1,
                Fragment1,
                NoDepth,
                NoStencil,
            );
            let encoder = p.new_render_command_encoder(
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
            p.setup(
                encoder,
                Vertex1Binds {
                    v_bind1: BindMany::Bytes(&[0.]),
                },
                Frag1Binds {
                    f_bind1: BindOne::Bytes(&f32x4::splat(1.).into()),
                },
            );

            p.setup(
                encoder,
                Vertex1Binds {
                    v_bind1: BindMany::BufferAndOffset(&f32_buffer, 0),
                },
                Frag1Binds {
                    f_bind1: BindOne::BufferAndOffset(&float4_buffer, 0),
                },
            );
        }
        {
            let p = RenderPipeline::new(
                "Test",
                &device,
                &lib,
                [
                    (MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend),
                    (MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend),
                ],
                Vertex1,
                Fragment1,
                NoDepth,
                NoStencil,
            );
            let encoder = p.new_render_command_encoder(
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
            p.setup(
                encoder,
                Vertex1Binds {
                    v_bind1: BindMany::Bytes(&[0.]),
                },
                Frag1Binds {
                    f_bind1: BindOne::Bytes(&f32x4::splat(1.).into()),
                },
            );
        }
        {
            let p = RenderPipeline::new(
                "Test",
                &device,
                &lib,
                [(MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend)],
                Vertex1,
                NoFragmentShader,
                HasDepth(MTLPixelFormat::Depth16Unorm),
                HasStencil(MTLPixelFormat::Stencil8),
            );
            let encoder = p.new_render_command_encoder(
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
            p.setup(
                encoder,
                Vertex1Binds {
                    v_bind1: BindMany::Bytes(&[0.]),
                },
                NoBinds,
            );
        }
    }
}
