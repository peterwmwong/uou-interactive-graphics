#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{metal::*, *};
use proj_4_textures::Delegate as Proj4Delegate;
use shader_bindings::*;
use std::f32::consts::PI;
use std::ops::Neg;
use std::simd::f32x2;

const INITIAL_CAMERA_DISTANCE: f32 = 1.;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

const N: f32 = 0.1;
const F: f32 = 100000.0;
const NEAR_FIELD_MAJOR_AXIS: f32 = N / INITIAL_CAMERA_DISTANCE;
const PERSPECTIVE_MATRIX: f32x4x4 = f32x4x4::new(
    [N, 0., 0., 0.],
    [0., N, 0., 0.],
    [0., 0., N + F, -N * F],
    [0., 0., 1., 0.],
);

struct Delegate {
    camera_distance: f32,
    camera_rotation: f32x2,
    matrix_model_to_world: f32x4x4,
    matrix_model_to_projection: f32x4x4,
    render_pipeline_state: RenderPipelineState,
    plane_texture: Texture,
    screen_size: f32x2,
    needs_render: bool,
}

impl RendererDelgate for Delegate {
    fn new(device: Device, command_queue: &CommandQueue) -> Self {
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        let render_pipeline_state = {
            let base_pipeline_desc = RenderPipelineDescriptor::new();
            {
                let desc = unwrap_option_dcheck(
                    base_pipeline_desc.color_attachments().object_at(0 as u64),
                    "Failed to access color attachment on pipeline descriptor",
                );
                desc.set_blending_enabled(false);
                desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
            }
            create_pipeline(
                &device,
                &library,
                &base_pipeline_desc,
                "Plane",
                None,
                &"main_vertex",
                VertexBufferIndex::LENGTH as _,
                &"main_fragment",
                FragBufferIndex::LENGTH as _,
            )
            .pipeline_state
        };
        let matrix_model_to_world =
            f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.) * f32x4x4::scale(0.5, 0.5, 0.5, 1.);

        // Render Project 4 (loads model specified on the command line)
        // TODO: START HERE
        // TODO: START HERE
        // TODO: START HERE
        // We need to rerender whenever ALT/Option Mouse Drag!
        // - Remove hardcoded 512x512 sizing, use max of screen size whenever it changes.
        let plane_texture = {
            let desc = TextureDescriptor::new();
            desc.set_width(512);
            desc.set_height(512);
            desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
            device.new_texture(&desc)
        };
        {
            let mut proj4 = Proj4Delegate::<false>::new(device, command_queue);
            proj4.on_event(UserEvent::WindowResize {
                size: f32x2::from_array([512., 512.]),
            });
            let command_buffer = proj4.render(command_queue, &plane_texture);
            command_buffer.commit();
            command_buffer.wait_until_completed();
        }

        let mut delegate = Self {
            camera_distance: INITIAL_CAMERA_DISTANCE,
            camera_rotation: INITIAL_CAMERA_ROTATION,
            matrix_model_to_world,
            matrix_model_to_projection: f32x4x4::identity(),
            plane_texture,
            render_pipeline_state,
            screen_size: f32x2::default(),
            needs_render: false,
        };

        delegate.update_camera(
            delegate.screen_size,
            delegate.camera_rotation,
            delegate.camera_distance,
        );
        delegate.reset_needs_render();
        delegate
    }

    #[inline]
    fn render<'a>(
        &mut self,
        command_queue: &'a CommandQueue,
        render_target: &TextureRef,
    ) -> &'a CommandBufferRef {
        self.reset_needs_render();
        let command_buffer = command_queue.new_command_buffer_with_unretained_references();
        command_buffer.set_label("Renderer Command Buffer");
        let encoder = command_buffer.new_render_command_encoder({
            let desc = RenderPassDescriptor::new();
            {
                let a = unwrap_option_dcheck(
                    desc.color_attachments().object_at(0),
                    "Failed to access color attachment on render pass descriptor",
                );
                a.set_texture(Some(render_target));
                a.set_load_action(MTLLoadAction::Clear);
                a.set_clear_color(MTLClearColor::new(0.0, 0.0, 0.0, 0.0));
                a.set_store_action(MTLStoreAction::Store);
            }
            desc
        });
        // Render Model
        {
            encoder.push_debug_group("Model");
            encoder.set_render_pipeline_state(&self.render_pipeline_state);
            encode_vertex_bytes(
                encoder,
                VertexBufferIndex::MatrixModelToProjection as _,
                self.matrix_model_to_projection.metal_float4x4(),
            );
            encoder.set_fragment_texture(FragBufferIndex::Texture as _, Some(&self.plane_texture));
            encoder.draw_primitives(MTLPrimitiveType::TriangleStrip, 0, 4);
            encoder.pop_debug_group();
        }
        encoder.end_encoding();
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        use MouseButton::*;
        use UserEvent::*;
        match event {
            MouseDrag {
                button,
                modifier_keys,
                drag_amount,
                ..
            } => {
                if modifier_keys.is_empty() {
                    let mut camera_rotation = self.camera_rotation;
                    let mut camera_distance = self.camera_distance;
                    match button {
                        Left => {
                            camera_rotation += {
                                let adjacent = f32x2::splat(self.camera_distance);
                                let opposite = drag_amount / f32x2::splat(500.);
                                let &[x, y] = (opposite / adjacent).as_array();
                                f32x2::from_array([
                                    y.atan(), // Rotation on x-axis
                                    x.atan(), // Rotation on y-axis
                                ])
                            }
                        }
                        Right => camera_distance += -drag_amount[1] / 250.,
                    }
                    self.update_camera(self.screen_size, camera_rotation, camera_distance);
                }
            }
            WindowResize { size, .. } => {
                self.update_camera(size, self.camera_rotation, self.camera_distance);
            }
            _ => {}
        }
    }

    #[inline(always)]
    fn needs_render(&self) -> bool {
        self.needs_render
    }
}

impl Delegate {
    #[inline(always)]
    fn reset_needs_render(&mut self) {
        self.needs_render = false;
    }

    #[inline]
    fn calc_matrix_camera_to_projection(&self, aspect_ratio: f32) -> f32x4x4 {
        let (w, h) = (NEAR_FIELD_MAJOR_AXIS, aspect_ratio * NEAR_FIELD_MAJOR_AXIS);
        let orthographic_matrix = {
            f32x4x4::new(
                [2. / w, 0., 0., 0.],
                [0., 2. / h, 0., 0.],
                // IMPORTANT: Metal's NDC coordinate space has a z range of [0.,1], **NOT [-1,1]** (OpenGL).
                [0., 0., 1. / (F - N), -N / (F - N)],
                [0., 0., 0., 1.],
            )
        };
        orthographic_matrix * PERSPECTIVE_MATRIX
    }

    fn update_camera(&mut self, screen_size: f32x2, camera_rotation: f32x2, camera_distance: f32) {
        self.screen_size = screen_size;
        self.camera_rotation = camera_rotation;
        self.camera_distance = camera_distance;
        let &[rotx, roty] = self.camera_rotation.neg().as_array();
        let matrix_world_to_camera =
            f32x4x4::translate(0., 0., self.camera_distance) * f32x4x4::rotate(rotx, roty, 0.);
        let &[sx, sy, ..] = screen_size.as_array();
        let aspect_ratio = sy / sx;
        let matrix_world_to_projection =
            self.calc_matrix_camera_to_projection(aspect_ratio) * matrix_world_to_camera;
        self.matrix_model_to_projection = matrix_world_to_projection * self.matrix_model_to_world;
        self.needs_render = true;
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 5 - Render Buffers");
}
