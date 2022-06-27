#![feature(array_zip)]
#![feature(portable_simd)]
#![feature(slice_as_chunks)]
mod shader_bindings;

use metal_app::*;
use metal_app::{metal::*, metal_types::*};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, f32x4},
};
use tobj::{LoadOptions, Mesh};

const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth32Float;
const INITIAL_CAMERA_DISTANCE: f32 = 50.;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 4., 0.]);
const INITIAL_MODE: FragMode = FragMode::AmbientDiffuseSpecular;
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_DISTANCE: f32 = INITIAL_CAMERA_DISTANCE / 2.;

struct Delegate {
    camera_distance: f32,
    camera_rotation: f32x2,
    camera_world_position: f32x4,
    command_queue: CommandQueue,
    depth_state: DepthStencilState,
    depth_texture: Option<Texture>,
    device: Device,
    light_world_position: f32x4,
    light_xy_rotation: f32x2,
    matrix_model_to_projection: f32x4x4,
    matrix_model_to_world: f32x4x4,
    matrix_projection_to_world: f32x4x4,
    matrix_world_to_camera: f32x4x4,
    matrix_world_to_projection: f32x4x4,
    max_bound: f32,
    mode: FragMode,
    num_triangles: usize,
    render_light_pipeline_state: RenderPipelineState,
    render_pipeline_state: RenderPipelineState,
    screen_size: f32x2,
    vertex_buffer_indices: Buffer,
    vertex_buffer_normals: Buffer,
    vertex_buffer_positions: Buffer,
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let teapot_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("teapot.obj");
        let (mut models, ..) = tobj::load_obj(
            teapot_file,
            &LoadOptions {
                single_index: true,
                triangulate: true,
                ignore_points: true,
                ignore_lines: true,
            },
        )
        .expect("Failed to load OBJ file");

        let model = models
            .pop()
            .expect("Failed to parse model, expecting atleast one model (teapot)");
        let Mesh {
            positions,
            indices,
            normals,
            ..
        } = model.mesh;

        debug_assert_eq!(
            indices.len() % 3,
            0,
            "`mesh.indices` should contain triples (triangle vertices). Model should have been loaded with `triangulate`, guaranteeing all faces have 3 vertices."
        );
        debug_assert_eq!(
            positions.len() % 3,
            0,
            "`mesh.positions` should contain triples (3D position)"
        );
        debug_assert_eq!(
            normals.len(),
            positions.len(),
            "`mesh.normals` should contain triples (3D vector)"
        );

        let (positions3, ..) = positions.as_chunks::<3>();
        let mut mins = f32x4::splat(f32::MAX);
        let mut maxs = f32x4::splat(f32::MIN);
        for &[x, y, z] in positions3 {
            let input = f32x4::from_array([x, y, z, 0.0]);
            mins = mins.min(input);
            maxs = maxs.max(input);
        }
        let max_bound = mins.reduce_min().abs().max(maxs.reduce_max());
        let matrix_model_to_world = {
            let height_of_teapot = maxs[2] - mins[2];
            f32x4x4::x_rotate(PI / 2.) * f32x4x4::translate(0., 0., -height_of_teapot / 2.0)
        };

        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");

        // Setup Render Pipeline Descriptor used for rendering the teapot and light
        let mut base_pipeline_desc =
            new_basic_render_pipeline_descriptor(DEFAULT_PIXEL_FORMAT, None, false);
        let mut delegate = Self {
            camera_distance: INITIAL_CAMERA_DISTANCE,
            camera_rotation: INITIAL_CAMERA_ROTATION,
            camera_world_position: f32x4::default(),
            command_queue: device.new_command_queue(),
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: None,
            light_world_position: f32x4::default(),
            light_xy_rotation: INITIAL_LIGHT_ROTATION,
            matrix_model_to_projection: f32x4x4::identity(),
            matrix_model_to_world,
            matrix_projection_to_world: f32x4x4::identity(),
            matrix_world_to_camera: f32x4x4::identity(),
            matrix_world_to_projection: f32x4x4::identity(),
            max_bound,
            mode: INITIAL_MODE,
            num_triangles: indices.len() / 3,
            render_pipeline_state: create_pipeline(
                &device,
                &library,
                &mut base_pipeline_desc,
                "Render Teapot Pipeline",
                None,
                (&"main_vertex", VertexBufferIndex::LENGTH as _),
                Some((&"main_fragment", FragBufferIndex::LENGTH as _)),
            )
            .pipeline_state,
            render_light_pipeline_state: create_pipeline(
                &device,
                &library,
                &mut base_pipeline_desc,
                "Render Light Pipeline",
                None,
                (&"light_vertex", LightVertexBufferIndex::LENGTH as _),
                Some((&"light_fragment", 0)),
            )
            .pipeline_state,
            screen_size: f32x2::default(),
            vertex_buffer_indices: allocate_new_buffer_with_data(
                &device,
                "Vertex Buffer Indices",
                &indices,
            ),
            vertex_buffer_normals: allocate_new_buffer_with_data(
                &device,
                "Vertex Buffer Normals",
                &normals,
            ),
            vertex_buffer_positions: allocate_new_buffer_with_data(
                &device,
                "Vertex Buffer Positions",
                &positions,
            ),
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
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        command_buffer.set_label("Renderer Command Buffer");
        let encoder = command_buffer.new_render_command_encoder(new_basic_render_pass_descriptor(
            render_target,
            self.depth_texture.as_ref(),
        ));
        encoder.set_render_pipeline_state(&self.render_pipeline_state);
        encoder.set_depth_stencil_state(&self.depth_state);

        let light_world_position = float4::from(self.light_world_position);

        // Render Teapot
        {
            encoder.set_vertex_buffer(
                VertexBufferIndex::Indices as _,
                Some(&self.vertex_buffer_indices),
                0,
            );
            encoder.set_vertex_buffer(
                VertexBufferIndex::Positions as _,
                Some(&self.vertex_buffer_positions),
                0,
            );
            encoder.set_vertex_buffer(
                VertexBufferIndex::Normals as _,
                Some(&self.vertex_buffer_normals),
                0,
            );
            encode_vertex_bytes(
                &encoder,
                VertexBufferIndex::MatrixNormalToWorld as _,
                // IMPORTANT: In the shader, this maps to a float3x3. This works because...
                // Conceptually, we want a matrix that ONLY applies rotation (no translation).
                // Since normals are directions (not positions, relative to a point on a surface),
                // translations are meaningless.
                &self.matrix_model_to_world,
            );
            encode_vertex_bytes(
                &encoder,
                VertexBufferIndex::MatrixModelToProjection as _,
                &self.matrix_model_to_projection,
            );
            encode_fragment_bytes(&encoder, FragBufferIndex::FragMode as _, &self.mode);
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex::MatrixProjectionToWorld as _,
                &self.matrix_projection_to_world,
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
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex::ScreenSize as _,
                &float2::from(self.screen_size),
            );
            encoder.draw_primitives_instanced(
                MTLPrimitiveType::Triangle,
                0,
                3,
                self.num_triangles as _,
            );
        }

        // Render Light
        {
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
                &self.matrix_world_to_projection,
            );
            encode_vertex_bytes(
                &encoder,
                LightVertexBufferIndex::LightPosition as _,
                &light_world_position,
            );
            encoder.draw_primitives(MTLPrimitiveType::Point, 0, 1);
        }
        encoder.end_encoding();
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        use metal_app::MouseButton::*;
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
                self.mode = match key_code {
                    29 /* 0 */ => FragMode::AmbientDiffuseSpecular,
                    18 /* 1 */ => FragMode::Normals,
                    19 /* 2 */ => FragMode::Ambient,
                    20 /* 3 */ => FragMode::AmbientDiffuse,
                    21 /* 4 */ => FragMode::Specular,
                    _ => self.mode
                };
            }
            WindowFocusedOrResized { size, .. } => {
                self.update_depth_texture_size(size);
                self.update_camera(size, self.camera_rotation, self.camera_distance);
            }
            _ => {}
        }
    }

    fn device(&self) -> &Device {
        &self.device
    }
}

impl Delegate {
    fn update_depth_texture_size(&mut self, size: f32x2) {
        let desc = TextureDescriptor::new();
        desc.set_width(size[0] as _);
        desc.set_height(size[1] as _);
        desc.set_pixel_format(DEPTH_TEXTURE_FORMAT);
        desc.set_storage_mode(MTLStorageMode::Memoryless);
        desc.set_usage(MTLTextureUsage::RenderTarget);
        self.depth_texture = Some(self.device.new_texture(&desc));
    }

    #[inline]
    fn calc_matrix_camera_to_projection(&self, aspect_ratio: f32) -> f32x4x4 {
        let n = 0.1;
        let f = 1000.0;
        let perspective_matrix = f32x4x4::new(
            [n, 0., 0., 0.],
            [0., n, 0., 0.],
            [0., 0., n + f, -n * f],
            [0., 0., 1., 0.],
        );
        let w = 2. * n * self.max_bound / INITIAL_CAMERA_DISTANCE;
        let h = aspect_ratio * w;
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
    launch_application::<Delegate>("Project 3 - Shading");
}
