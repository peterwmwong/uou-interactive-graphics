#![feature(array_methods)]
#![feature(const_ptr_offset_from)]
#![feature(portable_simd)]
#![feature(slice_as_chunks)]
mod model;
mod shader_bindings;

use bitflags::bitflags;
use metal_app::metal::*;
use metal_app::*;
use model::{MaxBounds, Model};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, f32x4},
};

bitflags! {
    struct Mode: u8 {
        const HAS_AMBIENT = 1 << FC::HAS_AMBIENT as u8;
        const HAS_DIFFUSE = 1 << FC::HAS_DIFFUSE as u8;
        const HAS_NORMAL = 1 << FC::HAS_NORMAL as u8;
        const HAS_SPECULAR = 1 << FC::HAS_SPECULAR as u8;
        const DEFAULT = Self::HAS_AMBIENT.bits | Self::HAS_DIFFUSE.bits | Self::HAS_SPECULAR.bits;
    }
}

const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_DISTANCE: f32 = 50.;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 4., 0.]);
const INITIAL_MODE: Mode = Mode::DEFAULT;
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_DISTANCE: f32 = INITIAL_CAMERA_DISTANCE / 2.;

struct Delegate {
    camera_distance: f32,
    camera_rotation: f32x2,
    camera_world_position: f32x4,
    depth_state: DepthStencilState,
    depth_texture: Option<Texture>,
    device: Device,
    library: Library,
    light_world_position: f32x4,
    light_xy_rotation: f32x2,
    matrix_model_to_projection: f32x4x4,
    matrix_model_to_world: f32x4x4,
    matrix_projection_to_world: f32x4x4,
    matrix_world_to_camera: f32x4x4,
    matrix_world_to_projection: f32x4x4,
    mode: Mode,
    model: Model,
    render_light_pipeline_state: RenderPipelineState,
    render_pipeline_state: RenderPipelineState,
    screen_size: f32x2,
}

fn create_pipelines(
    device: &Device,
    library: &Library,
    mode: Mode,
) -> (
    (Function, Function, RenderPipelineState),
    RenderPipelineState,
) {
    let base_pipeline_desc = RenderPipelineDescriptor::new();
    base_pipeline_desc.set_depth_attachment_pixel_format(DEPTH_TEXTURE_FORMAT);
    {
        let desc = unwrap_option_dcheck(
            base_pipeline_desc.color_attachments().object_at(0 as u64),
            "Failed to access color attachment on pipeline descriptor",
        );
        desc.set_blending_enabled(false);
        desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
    }

    let function_constants = FunctionConstantValues::new();
    for index in [
        FC::HAS_AMBIENT as usize,
        FC::HAS_DIFFUSE as usize,
        FC::HAS_SPECULAR as usize,
        FC::HAS_NORMAL as usize,
    ] {
        function_constants.set_constant_value_at_index(
            (&mode.contains(Mode::from_bits_truncate(1 << index)) as *const _) as _,
            MTLDataType::Bool,
            index as _,
        );
    }
    (
        create_pipeline_with_constants(
            &device,
            &library,
            &base_pipeline_desc,
            "Teapot",
            Some(&function_constants),
            &"main_vertex",
            VertexBufferIndex::LENGTH as _,
            &"main_fragment",
            FragBufferIndex::LENGTH as _,
        ),
        create_pipeline_with_constants(
            &device,
            &library,
            &base_pipeline_desc,
            "Light",
            Some(&function_constants),
            &"light_vertex",
            LightVertexBufferIndex::LENGTH as _,
            &"light_fragment",
            0,
        )
        .2,
    )
}

impl RendererDelgate for Delegate {
    fn new(device: Device, _command_queue: &CommandQueue) -> Self {
        let teapot_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("teapot.obj");
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        let mode = INITIAL_MODE;
        let ((vertex_fn, frag_fn, render_pipeline_state), render_light_pipeline_state) =
            create_pipelines(&device, &library, mode);
        let model = Model::from_file(
            teapot_file,
            &device,
            &vertex_fn.new_argument_encoder(VertexBufferIndex::ObjectGeometry as _),
            &frag_fn.new_argument_encoder(FragBufferIndex::Material as _),
        );

        let mut delegate = Self {
            camera_distance: INITIAL_CAMERA_DISTANCE,
            camera_rotation: INITIAL_CAMERA_ROTATION,
            camera_world_position: f32x4::default(),
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: None,
            library,
            light_world_position: f32x4::default(),
            light_xy_rotation: INITIAL_LIGHT_ROTATION,
            matrix_model_to_projection: f32x4x4::identity(),
            matrix_model_to_world: {
                // TODO: START HERE 3
                // TODO: START HERE 3
                // TODO: START HERE 3
                // This should be based on model bounds
                let height_of_teapot = 15.75;
                f32x4x4::x_rotate(PI / 2.) * f32x4x4::translate(0., 0., -height_of_teapot / 2.0)
            },
            matrix_projection_to_world: f32x4x4::identity(),
            matrix_world_to_camera: f32x4x4::identity(),
            matrix_world_to_projection: f32x4x4::identity(),
            mode,
            model,
            render_pipeline_state,
            render_light_pipeline_state,
            screen_size: f32x2::default(),
            device,
        };
        delegate.update_light(delegate.light_xy_rotation);
        delegate.update_camera(
            delegate.screen_size,
            delegate.camera_rotation,
            delegate.camera_distance,
        );
        delegate
    }

    #[inline]
    fn draw(&mut self, command_queue: &CommandQueue, drawable: &MetalDrawableRef) {
        let command_buffer = command_queue.new_command_buffer();
        command_buffer.set_label("Renderer Command Buffer");
        let encoder = command_buffer.new_render_command_encoder({
            let desc = RenderPassDescriptor::new();
            {
                let a = unwrap_option_dcheck(
                    desc.color_attachments().object_at(0),
                    "Failed to access color attachment on render pass descriptor",
                );
                a.set_texture(Some(drawable.texture()));
                a.set_load_action(MTLLoadAction::Clear);
                a.set_clear_color(MTLClearColor::new(0.0, 0.0, 0.0, 0.0));
                a.set_store_action(MTLStoreAction::Store);
            }
            {
                let a = desc.depth_attachment().unwrap();
                a.set_clear_depth(1.);
                a.set_load_action(MTLLoadAction::Clear);
                a.set_store_action(MTLStoreAction::DontCare);
                a.set_texture(self.depth_texture.as_deref());
            }
            desc
        });
        encoder.set_render_pipeline_state(&self.render_pipeline_state);
        self.model.encode_use_resources(&encoder);
        encoder.set_depth_stencil_state(&self.depth_state);

        let light_world_position = float4::from(self.light_world_position);

        // Render Model
        for o in self.model.object_iter() {
            encoder.push_debug_group(&o.name());
            o.encode_vertex_buffer_for_geometry_argument_buffer(
                encoder,
                VertexBufferIndex::ObjectGeometry as _,
            );
            encode_vertex_bytes(
                &encoder,
                VertexBufferIndex::MatrixNormalToWorld as _,
                // IMPORTANT: In the shader, this maps to a float3x3. This works because...
                // 1. Conceptually, we want a matrix that ONLY applies rotation (no translation)
                //   - Since normals are directions (not positions), translations are meaningless and
                //     should not be applied.
                // 2. Memory layout-wise, float3x3 and float4x4 have the same size and alignment.
                //
                // TODO: Although this performs great (compare assembly running "asm proj-3-shading"
                //       task), this may be wayyy too tricky/error-prone/assumes-metal-ignores-the-extra-stuff.
                &self.matrix_model_to_world,
            );
            encode_vertex_bytes(
                &encoder,
                VertexBufferIndex::MatrixModelToProjection as _,
                self.matrix_model_to_projection.metal_float4x4(),
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex::MatrixProjectionToWorld as _,
                self.matrix_projection_to_world.metal_float4x4(),
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex::ScreenSize as _,
                &float2::from(self.screen_size),
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex::LightPosition as _,
                // IMPORTANT: In the shader, this maps to a float3. This works because the float4
                // and float3 have the same size and alignment.
                &light_world_position,
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex::CameraPosition as _,
                // IMPORTANT: In the shader, this maps to a float3. This works because the float4
                // and float3 have the same size and alignment.
                &float4::from(self.camera_world_position),
            );
            o.encode_fragment_buffer_for_material_argument_buffer(
                encoder,
                FragBufferIndex::Material as _,
            );
            encoder.draw_primitives_instanced(
                MTLPrimitiveType::Triangle,
                0,
                3,
                o.num_triangles() as _,
            );
            encoder.pop_debug_group();
        }

        // Render Light
        {
            encoder.push_debug_group("Light");
            // TODO: Figure out a better way to unset this buffers from the previous draw call
            encoder.set_vertex_buffers(
                0,
                &[None; VertexBufferIndex::LENGTH as _],
                &[0; VertexBufferIndex::LENGTH as _],
            );
            encoder.set_fragment_buffers(
                0,
                &[None; FragBufferIndex::LENGTH as _],
                &[0; FragBufferIndex::LENGTH as _],
            );
            encoder.set_render_pipeline_state(&self.render_light_pipeline_state);
            encode_vertex_bytes(
                &encoder,
                LightVertexBufferIndex::MatrixWorldToProjection as _,
                self.matrix_world_to_projection.metal_float4x4(),
            );
            encode_vertex_bytes(
                &encoder,
                LightVertexBufferIndex::LightPosition as _,
                &light_world_position,
            );
            encoder.draw_primitives(MTLPrimitiveType::Point, 0, 1);
            encoder.pop_debug_group();
        }
        encoder.end_encoding();
        command_buffer.present_drawable(drawable);
        command_buffer.commit();
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
                                let offsets = drag_amount / f32x2::splat(4.);
                                let ratio = offsets / adjacent;
                                f32x2::from_array([
                                    ratio[1].atan(), // Rotation on x-axis
                                    ratio[0].atan(), // Rotation on y-axis
                                ])
                            }
                        }
                        Right => camera_distance += -drag_amount[1] / 8.0,
                    }
                    self.update_camera(self.screen_size, camera_rotation, camera_distance);
                } else if modifier_keys.contains(ModifierKeys::CONTROL) {
                    match button {
                        Left => {
                            let adjacent = f32x2::splat(self.camera_distance);
                            let opposite = -drag_amount / f32x2::splat(16.);
                            let ratio = opposite / adjacent;
                            self.update_light(
                                self.light_xy_rotation
                                    + f32x2::from_array([
                                        ratio[1].atan(), // Rotation on x-axis
                                        ratio[0].atan(), // Rotation on y-axis
                                    ]),
                            );
                        }
                        _ => {}
                    }
                }
            }
            KeyDown { key_code, .. } => {
                self.update_mode(match key_code {
                    29 /* 0 */ => Mode::DEFAULT,
                    18 /* 1 */ => Mode::HAS_NORMAL,
                    19 /* 2 */ => Mode::HAS_AMBIENT,
                    20 /* 3 */ => Mode::HAS_AMBIENT | Mode::HAS_DIFFUSE,
                    21 /* 4 */ => Mode::HAS_SPECULAR,
                    _ => self.mode
                });
            }
            WindowResize { size, .. } => {
                self.update_depth_texture_size(size);
                self.update_camera(size, self.camera_rotation, self.camera_distance);
            }
            _ => {}
        }
    }
}

impl Delegate {
    fn update_mode(&mut self, mode: Mode) {
        if mode != self.mode {
            self.mode = mode;
            (
                (_, _, self.render_pipeline_state),
                self.render_light_pipeline_state,
            ) = create_pipelines(&self.device, &self.library, mode);
        }
    }

    fn update_depth_texture_size(&mut self, size: f32x2) {
        let desc = TextureDescriptor::new();
        desc.set_width(size[0] as _);
        desc.set_height(size[1] as _);
        desc.set_pixel_format(DEPTH_TEXTURE_FORMAT);
        desc.set_storage_mode(MTLStorageMode::Memoryless);
        desc.set_usage(MTLTextureUsage::RenderTarget);
        let texture = self.device.new_texture(&desc);
        texture.set_label("Depth");
        self.depth_texture = Some(texture);
    }

    #[inline]
    fn calc_matrix_camera_to_projection(&self, aspect_ratio: f32) -> f32x4x4 {
        // TODO: START HERE 4
        // TODO: START HERE 4
        // TODO: START HERE 4
        // TODO: Calculate once, constantify
        let n = 0.1;
        let f = 1000.0;
        let perspective_matrix = f32x4x4::new(
            [n, 0., 0., 0.],
            [0., n, 0., 0.],
            [0., 0., n + f, -n * f],
            [0., 0., 1., 0.],
        );
        // TODO: START HERE 5
        // TODO: START HERE 5
        // TODO: START HERE 5
        // TODO: Calculate once
        let MaxBounds { width, height } = self.model.max_bounds;
        let (w, h) = if width > height {
            let w = n * width / INITIAL_CAMERA_DISTANCE;
            let h = aspect_ratio * w;
            (w, h)
        } else {
            let h = n * height / INITIAL_CAMERA_DISTANCE;
            let w = h / aspect_ratio;
            (w, h)
        };
        let orthographic_matrix = {
            f32x4x4::new(
                [2. / w, 0., 0., 0.],
                [0., 2. / h, 0., 0.],
                // IMPORTANT: Metal's NDC coordinate space has a z range of [0.,1], **NOT [-1,1]** (OpenGL).
                [0., 0., 1. / (f - n), -n / (f - n)],
                [0., 0., 0., 1.],
            )
        };
        orthographic_matrix * perspective_matrix
    }

    fn update_light(&mut self, light_xy_rotation: f32x2) {
        self.light_xy_rotation = light_xy_rotation;
        self.light_world_position =
            f32x4x4::rotate(self.light_xy_rotation[0], self.light_xy_rotation[1], 0.)
                * f32x4::from_array([0., 0., -LIGHT_DISTANCE, 1.])
    }

    fn update_camera(&mut self, screen_size: f32x2, camera_rotation: f32x2, camera_distance: f32) {
        self.camera_rotation = camera_rotation;
        self.camera_distance = camera_distance;
        self.matrix_world_to_camera = f32x4x4::translate(0., 0., self.camera_distance)
            * f32x4x4::rotate(-self.camera_rotation[0], -self.camera_rotation[1], 0.);
        self.camera_world_position =
            self.matrix_world_to_camera.inverse() * f32x4::from_array([0., 0., 0., 1.]);

        self.screen_size = screen_size;
        let aspect_ratio = screen_size[1] / screen_size[0];
        self.matrix_world_to_projection =
            self.calc_matrix_camera_to_projection(aspect_ratio) * self.matrix_world_to_camera;
        self.matrix_model_to_projection =
            self.matrix_world_to_projection * self.matrix_model_to_world;
        self.matrix_projection_to_world = self.matrix_world_to_projection.inverse();
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 4 - Textures");
}
