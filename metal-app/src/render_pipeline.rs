use metal::{
    CommandBufferRef, MTLClearColor, MTLLoadAction, MTLStoreAction, RenderCommandEncoderRef,
    RenderPassDescriptor, RenderPassDescriptorRef, TextureRef,
};
use std::marker::PhantomData;

pub mod bind {
    use crate::{encode_fragment_bytes, encode_vertex_bytes};
    use metal::{BufferRef, RenderCommandEncoderRef};

    pub trait Bind {
        fn encode_fragment(&self, buffer_index: usize, encoder: &RenderCommandEncoderRef);
        fn encode_vertex(&self, buffer_index: usize, encoder: &RenderCommandEncoderRef);
    }

    pub struct BindBytes<'a, T: Sized>(&'a T);
    impl<'a, T: Sized> Bind for BindBytes<'a, T> {
        #[inline]
        fn encode_fragment(&self, buffer_index: usize, encoder: &RenderCommandEncoderRef) {
            encode_fragment_bytes(encoder, buffer_index as _, &self.0);
        }

        #[inline]
        fn encode_vertex(&self, buffer_index: usize, encoder: &RenderCommandEncoderRef) {
            encode_vertex_bytes(encoder, buffer_index as _, &self.0);
        }
    }

    pub struct BindBufferAndOffset<'a>(&'a BufferRef, u32);
    impl<'a> Bind for BindBufferAndOffset<'a> {
        #[inline]
        fn encode_fragment(&self, buffer_index: usize, encoder: &RenderCommandEncoderRef) {
            encoder.set_fragment_buffer(buffer_index as _, Some(self.0), self.1 as _);
        }

        #[inline]
        fn encode_vertex(&self, buffer_index: usize, encoder: &RenderCommandEncoderRef) {
            encoder.set_vertex_buffer(buffer_index as _, Some(self.0), self.1 as _);
        }
    }

    pub struct BindBufferOffset(u32);
    impl Bind for BindBufferOffset {
        #[inline]
        fn encode_fragment(&self, buffer_index: usize, encoder: &RenderCommandEncoderRef) {
            encoder.set_fragment_buffer_offset(buffer_index as _, self.0 as _);
        }

        #[inline]
        fn encode_vertex(&self, buffer_index: usize, encoder: &RenderCommandEncoderRef) {
            encoder.set_vertex_buffer_offset(buffer_index as _, self.0 as _);
        }
    }

    pub fn bytes<'a, T: Sized>(v: &'a T) -> impl Bind + 'a {
        BindBytes(v)
    }
    pub fn buffer<'a>(b: &'a BufferRef) -> BindBufferAndOffset<'a> {
        BindBufferAndOffset(b, 0)
    }
    pub fn buffer_and_offset<'a>(b: &'a BufferRef, o: u32) -> impl Bind + 'a {
        BindBufferAndOffset(b, o)
    }
    pub fn buffer_offset(o: u32) -> impl Bind {
        BindBufferOffset(o)
    }
}

pub trait DepthAttachmentKind {
    type RenderPassDesc<'a>;

    fn setup_attachment<'a>(_desc: Self::RenderPassDesc<'a>, _pass: &RenderPassDescriptorRef) {}
}
pub struct HasDepth();
impl DepthAttachmentKind for HasDepth {
    type RenderPassDesc<'a> = (&'a TextureRef, f32, MTLLoadAction, MTLStoreAction);

    fn setup_attachment<'a>(
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

    fn setup_attachment<'a>(_desc: Self::RenderPassDesc<'a>, _pass: &RenderPassDescriptorRef) {}
}
pub struct HasStencil;
impl StencilAttachmentKind for HasStencil {
    type RenderPassDesc<'a> = (&'a TextureRef, u32, MTLLoadAction, MTLStoreAction);

    fn setup_attachment<'a>(
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
}

pub trait ShaderArgs {
    fn encode_vertex_args<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef);
    fn encode_fragment_args<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef);
}

pub struct NoShaderArgs;
impl ShaderArgs for NoShaderArgs {
    #[inline]
    fn encode_vertex_args<'a, 'b>(&'a self, _encoder: &'b RenderCommandEncoderRef) {}
    #[inline]
    fn encode_fragment_args<'a, 'b>(&'a self, _encoder: &'b RenderCommandEncoderRef) {}
}

pub struct RenderPipeline<
    const NUM_COLOR_ATTACHMENTS: usize = 1,
    D: DepthAttachmentKind = NoDepth,
    S: StencilAttachmentKind = NoStencil,
    VA: ShaderArgs = NoShaderArgs,
    FA: ShaderArgs = NoShaderArgs,
> {
    _depth_kind: PhantomData<D>,
    _stencil_kind: PhantomData<S>,
    _vertex_args: PhantomData<VA>,
    _fragment_args: PhantomData<FA>,
}

type RenderPassColorAttachment<'a> = (
    &'a TextureRef,
    (f32, f32, f32, f32),
    MTLLoadAction,
    MTLStoreAction,
);

impl<
        const NUM_COLOR_ATTACHMENTS: usize,
        D: DepthAttachmentKind,
        S: StencilAttachmentKind,
        VA: ShaderArgs,
        FA: ShaderArgs,
    > RenderPipeline<NUM_COLOR_ATTACHMENTS, D, S, VA, FA>
{
    pub fn new() -> Self {
        Self {
            _depth_kind: PhantomData,
            _stencil_kind: PhantomData,
            _vertex_args: PhantomData,
            _fragment_args: PhantomData,
        }
    }

    pub fn new_render_command_encoder<'a, 'b, 'c>(
        &self,
        command_buffer: &'a CommandBufferRef,
        color_attachments: [RenderPassColorAttachment; NUM_COLOR_ATTACHMENTS],
        depth_attachment: D::RenderPassDesc<'b>,
        stencil_attachment: S::RenderPassDesc<'c>,
    ) -> &'a RenderCommandEncoderRef {
        let desc = RenderPassDescriptor::new();
        for i in 0..NUM_COLOR_ATTACHMENTS {
            let (render_target, (r, g, b, alpha), load_action, store_action) = color_attachments[i];
            let a = desc
                .color_attachments()
                .object_at(i as _)
                .expect("Failed to access color attachment on render pass descriptor");
            a.set_clear_color(MTLClearColor::new(r as _, g as _, b as _, alpha as _));
            a.set_load_action(load_action);
            a.set_store_action(store_action);
            a.set_texture(Some(render_target));
        }
        D::setup_attachment(depth_attachment, desc);
        S::setup_attachment(stencil_attachment, desc);
        command_buffer.new_render_command_encoder(desc)
    }

    pub fn setup(&self, encoder: &RenderCommandEncoderRef, vertex_args: VA, fragment_args: FA) {
        vertex_args.encode_vertex_args(encoder);
        fragment_args.encode_fragment_args(encoder);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use metal::{Device, MTLResourceOptions, TextureDescriptor};

    // Assumed to be generated by metal-build (shader_function_parser/generate_shader_args)
    struct Vertex1<'a> {
        arg1: &'a dyn bind::Bind,
    }
    impl<'c> ShaderArgs for Vertex1<'c> {
        #[inline]
        fn encode_vertex_args<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef) {
            self.arg1.encode_vertex(0, encoder);
        }
        #[inline]
        fn encode_fragment_args<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef) {
            self.arg1.encode_fragment(0, encoder);
        }
    }

    // #[test]
    fn test() {
        let device = Device::system_default().expect("Failed to get Metal Device");
        let command_queue = device.new_command_queue();
        let command_buffer = command_queue.new_command_buffer();
        let texture = device.new_texture(&TextureDescriptor::new());
        let arg_buffer = device.new_buffer(4, MTLResourceOptions::StorageModeManaged);

        {
            let p = RenderPipeline::<1, NoDepth, NoStencil, Vertex1>::new();
            let color1 = &texture;
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
                Vertex1 {
                    arg1: &bind::buffer(&arg_buffer),
                },
                NoShaderArgs,
            );
        }
        {
            let p = RenderPipeline::<2, NoDepth, NoStencil>::new();
            let color1 = &texture;
            let color2 = &texture;
            p.new_render_command_encoder(
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
        }
        {
            let p = RenderPipeline::<0, HasDepth, NoStencil>::new();
            let depth_texture = &texture;
            p.new_render_command_encoder(
                command_buffer,
                [],
                (
                    depth_texture,
                    0.,
                    MTLLoadAction::Clear,
                    MTLStoreAction::Store,
                ),
                NoStencil,
            );
        }
        {
            let p = RenderPipeline::<0, NoDepth, HasStencil>::new();
            let stencil_texture = &texture;
            p.new_render_command_encoder(
                command_buffer,
                [],
                NoDepth,
                (
                    stencil_texture,
                    0,
                    MTLLoadAction::Clear,
                    MTLStoreAction::Store,
                ),
            );
        }
    }
}
