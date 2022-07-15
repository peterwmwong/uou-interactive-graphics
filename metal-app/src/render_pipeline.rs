use std::marker::PhantomData;

use metal::{
    CommandBufferRef, MTLClearColor, MTLLoadAction, MTLStoreAction, RenderCommandEncoderRef,
    RenderPassDescriptor, RenderPassDescriptorRef, TextureRef,
};

pub struct Bind {}

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
pub struct NoDepth();
impl DepthAttachmentKind for NoDepth {
    type RenderPassDesc<'a> = ();
}

pub trait StencilAttachmentKind {
    type RenderPassDesc<'a>;

    fn setup_attachment<'a>(_desc: Self::RenderPassDesc<'a>, _pass: &RenderPassDescriptorRef) {}
}
pub struct HasStencil();
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
pub struct NoStencil();
impl StencilAttachmentKind for NoStencil {
    type RenderPassDesc<'a> = ();
}

pub struct RenderPipeline<
    const NUM_COLOR_ATTACHMENTS: usize,
    D: DepthAttachmentKind,
    S: StencilAttachmentKind,
> {
    _depth_kind: PhantomData<D>,
    _stencil_kind: PhantomData<S>,
}

type RenderPassColorAttachment<'a> = (
    &'a TextureRef,
    (f32, f32, f32, f32),
    MTLLoadAction,
    MTLStoreAction,
);

impl<const NUM_COLOR_ATTACHMENTS: usize, D: DepthAttachmentKind, S: StencilAttachmentKind>
    RenderPipeline<NUM_COLOR_ATTACHMENTS, D, S>
{
    pub fn new() -> Self {
        Self {
            _depth_kind: PhantomData,
            _stencil_kind: PhantomData,
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
}

#[cfg(test)]
mod test {
    use super::*;
    use metal::{Device, TextureDescriptor};

    // #[test]
    fn test() {
        let device = Device::system_default().expect("Failed to get Metal Device");
        let command_queue = device.new_command_queue();
        let command_buffer = command_queue.new_command_buffer();
        let texture = device.new_texture(&TextureDescriptor::new());

        {
            let p = RenderPipeline::<0, NoDepth, NoStencil>::new();
            p.new_render_command_encoder(command_buffer, [], (), ());
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
                (),
                (),
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
                (),
            );
        }
        {
            let p = RenderPipeline::<0, NoDepth, HasStencil>::new();
            let stencil_texture = &texture;
            p.new_render_command_encoder(
                command_buffer,
                [],
                (),
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
