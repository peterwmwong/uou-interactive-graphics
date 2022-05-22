#![feature(portable_simd)]
#![feature(slice_as_chunks)]
mod shader_bindings;

use crate::shader_bindings::{
    LightVertexBufferIndex_LightVertexBufferIndexLightPosition,
    VertexBufferIndex_VertexBufferIndexLENGTH,
    VertexBufferIndex_VertexBufferIndexMatrixNormalToWorld,
};
use metal_app::{
    allocate_new_buffer_with_data, encode_fragment_bytes, encode_vertex_bytes, f32x4x4,
    launch_application, metal::*, unwrap_option_dcheck, unwrap_result_dcheck, ModifierKeys,
    RendererDelgate, UserEvent,
};
use shader_bindings::{
    packed_float4, FragBufferIndex_FragBufferIndexCameraPosition,
    FragBufferIndex_FragBufferIndexFragMode, FragBufferIndex_FragBufferIndexLENGTH,
    FragBufferIndex_FragBufferIndexLightPosition,
    FragBufferIndex_FragBufferIndexMatrixProjectionToWorld,
    FragBufferIndex_FragBufferIndexScreenSize, FragMode, FragMode_FragModeAmbient,
    FragMode_FragModeAmbientDiffuse, FragMode_FragModeAmbientDiffuseSpecular,
    FragMode_FragModeNormals, FragMode_FragModeSpecular,
    LightVertexBufferIndex_LightVertexBufferIndexMatrixWorldToProjection,
    VertexBufferIndex_VertexBufferIndexIndices,
    VertexBufferIndex_VertexBufferIndexMatrixModelToProjection,
    VertexBufferIndex_VertexBufferIndexNormals, VertexBufferIndex_VertexBufferIndexPositions,
};
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, f32x4, Simd},
};
use tobj::{LoadOptions, Mesh};

const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth32Float;
const INITIAL_CAMERA_DISTANCE: f32 = 50.;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 4., 0.]);
const LIGHT_DISTANCE: f32 = INITIAL_CAMERA_DISTANCE / 2.;
const INITIAL_MODE: FragMode = FragMode_FragModeAmbientDiffuseSpecular;
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

struct Delegate {
    camera_distance: f32,
    camera_rotation: f32x2,
    depth_state: DepthStencilState,
    depth_texture: Option<Texture>,
    device: Device,
    light_xy_rotation: f32x2,
    max_bound: f32,
    matrix_model_to_projection: f32x4x4,
    matrix_model_to_world: f32x4x4,
    matrix_projection_to_world: f32x4x4,
    matrix_world_to_projection: f32x4x4,
    matrix_world_to_view: f32x4x4,
    mode: FragMode,
    num_triangles: usize,
    render_pipeline_state: RenderPipelineState,
    render_light_pipeline_state: RenderPipelineState,
    screen_size: f32x2,
    vertex_buffer_indices: Buffer,
    vertex_buffer_normals: Buffer,
    vertex_buffer_positions: Buffer,
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
    fn calc_matrix_view_to_projection(&self, aspect_ratio: f32) -> f32x4x4 {
        let n = 0.1;
        let f = 1000.0;
        let camera_distance = INITIAL_CAMERA_DISTANCE;

        let perspective_matrix = f32x4x4::new(
            [n, 0., 0., 0.],
            [0., n, 0., 0.],
            [0., 0., n + f, -n * f],
            [0., 0., 1., 0.],
        );
        // TODO: Simplify these calculations knowing b = -t and l = -r.
        let b = n * -self.max_bound / camera_distance;
        let t = n * self.max_bound / camera_distance;
        let l = aspect_ratio * b;
        let r = aspect_ratio * t;
        let orthographic_matrix = {
            f32x4x4::new(
                [2. / (r - l), 0., 0., -(r + l) / (r - l)],
                [0., 2. / (t - b), 0., -(t + b) / (t - b)],
                // IMPORTANT: Metal's NDC coordinate space has a z range of [0.,1], **NOT [-1,1]** (OpenGL).
                [0., 0., 1. / (f - n), -n / (f - n)],
                [0., 0., 0., 1.],
            )
        };
        orthographic_matrix * perspective_matrix
    }

    // TODO: Consider mass renaming everyting with "view" to "camera"
    // - Really view and camera are the same (view space = camera space)
    // - This would reduce cognitive load (view? camera? oh right their the same?)
    fn update_view(&mut self, screen_size: f32x2, camera_rotation: f32x2, camera_distance: f32) {
        self.camera_rotation = camera_rotation;
        self.camera_distance = camera_distance;
        self.matrix_world_to_view = f32x4x4::translate(0., 0., self.camera_distance)
            * f32x4x4::rotate(-self.camera_rotation[0], -self.camera_rotation[1], 0.);

        self.screen_size = screen_size;
        let aspect_ratio = screen_size[0] / screen_size[1];
        self.matrix_world_to_projection =
            self.calc_matrix_view_to_projection(aspect_ratio) * self.matrix_world_to_view;
        self.matrix_model_to_projection =
            self.matrix_world_to_projection * self.matrix_model_to_world;
        self.matrix_projection_to_world = self.matrix_world_to_projection.inverse();
    }
}

// TODO: Display light
impl RendererDelgate for Delegate {
    fn new(device: metal_app::metal::Device) -> Self {
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
        let mut mins: f32x4 = Simd::splat(f32::MAX);
        let mut maxs: f32x4 = Simd::splat(f32::MIN);
        for &[x, y, z] in positions3 {
            let input = Simd::from_array([x, y, z, 0.0]);
            mins = mins.min(input);
            maxs = maxs.max(input);
        }
        let max_bound = mins.reduce_min().abs().max(maxs.reduce_max());
        let matrix_model_to_world = {
            let height_of_teapot = maxs[2] - mins[2];
            f32x4x4::x_rotate(PI / 2.) * f32x4x4::translate(0., 0., -height_of_teapot / 2.0)
        };

        let mut delegate = Self {
            camera_distance: INITIAL_CAMERA_DISTANCE,
            camera_rotation: INITIAL_CAMERA_ROTATION,
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: None,
            light_xy_rotation: INITIAL_LIGHT_ROTATION,
            matrix_model_to_projection: f32x4x4::identity(),
            matrix_model_to_world,
            matrix_projection_to_world: f32x4x4::identity(),
            matrix_world_to_view: f32x4x4::identity(),
            matrix_world_to_projection: f32x4x4::identity(),
            max_bound,
            mode: INITIAL_MODE,
            num_triangles: indices.len() / 3,
            render_pipeline_state: {
                let library = device
                    .new_library_with_data(LIBRARY_BYTES)
                    .expect("Failed to import shader metal lib.");

                let pipeline_state_desc = RenderPipelineDescriptor::new();
                pipeline_state_desc.set_label("Render Pipeline");

                // Setup Vertex Shader
                {
                    let fun = library
                        .get_function(&"main_vertex", None)
                        .expect("Failed to access vertex shader function from metal library");
                    pipeline_state_desc.set_vertex_function(Some(&fun));

                    let buffers = pipeline_state_desc
                        .vertex_buffers()
                        .expect("Failed to access vertex buffers");
                    for buffer_index in 0..VertexBufferIndex_VertexBufferIndexLENGTH {
                        unwrap_option_dcheck(
                            buffers.object_at(buffer_index as _),
                            "Failed to access vertex buffer",
                        )
                        .set_mutability(MTLMutability::Immutable);
                    }
                }

                pipeline_state_desc.set_depth_attachment_pixel_format(DEPTH_TEXTURE_FORMAT);

                // Setup Fragment Shader
                {
                    let fun = unwrap_result_dcheck(
                        library.get_function(&"main_fragment", None),
                        "Failed to access fragment shader function from metal library",
                    );
                    pipeline_state_desc.set_fragment_function(Some(&fun));

                    let buffers = pipeline_state_desc
                        .fragment_buffers()
                        .expect("Failed to access fragment buffers");
                    for buffer_index in 0..FragBufferIndex_FragBufferIndexLENGTH {
                        unwrap_option_dcheck(
                            buffers.object_at(buffer_index as _),
                            "Failed to access fragment buffer",
                        )
                        .set_mutability(MTLMutability::Immutable);
                    }
                }

                // Setup Target Color Attachment
                {
                    let desc = &unwrap_option_dcheck(
                        pipeline_state_desc.color_attachments().object_at(0 as u64),
                        "Failed to access color attachment on pipeline descriptor",
                    );
                    // TODO: Maybe disable this, since we don't have any transparency
                    desc.set_blending_enabled(true);

                    desc.set_rgb_blend_operation(MTLBlendOperation::Add);
                    desc.set_source_rgb_blend_factor(MTLBlendFactor::SourceAlpha);
                    desc.set_destination_rgb_blend_factor(MTLBlendFactor::OneMinusSourceAlpha);

                    desc.set_alpha_blend_operation(MTLBlendOperation::Add);
                    desc.set_source_alpha_blend_factor(MTLBlendFactor::SourceAlpha);
                    desc.set_destination_alpha_blend_factor(MTLBlendFactor::OneMinusSourceAlpha);

                    desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
                }

                unwrap_result_dcheck(
                    device.new_render_pipeline_state(&pipeline_state_desc),
                    "Failed to create render pipeline",
                )
            },
            render_light_pipeline_state: {
                let library = device
                    .new_library_with_data(LIBRARY_BYTES)
                    .expect("Failed to import shader metal lib.");

                let pipeline_state_desc = RenderPipelineDescriptor::new();
                pipeline_state_desc.set_label("Render Light Pipeline");

                // Setup Vertex Shader
                {
                    let fun = library
                        .get_function(&"light_vertex", None)
                        .expect("Failed to access vertex shader function from metal library");
                    pipeline_state_desc.set_vertex_function(Some(&fun));

                    let buffers = pipeline_state_desc
                        .vertex_buffers()
                        .expect("Failed to access vertex buffers");
                    for buffer_index in [LightVertexBufferIndex_LightVertexBufferIndexLightPosition]
                    {
                        unwrap_option_dcheck(
                            buffers.object_at(buffer_index as _),
                            "Failed to access vertex buffer",
                        )
                        .set_mutability(MTLMutability::Immutable);
                    }
                }

                pipeline_state_desc.set_depth_attachment_pixel_format(DEPTH_TEXTURE_FORMAT);

                // Setup Fragment Shader
                {
                    let fun = unwrap_result_dcheck(
                        library.get_function(&"light_fragment", None),
                        "Failed to access fragment shader function from metal library",
                    );
                    pipeline_state_desc.set_fragment_function(Some(&fun));
                }

                // Setup Target Color Attachment
                {
                    let desc = &unwrap_option_dcheck(
                        pipeline_state_desc.color_attachments().object_at(0 as u64),
                        "Failed to access color attachment on pipeline descriptor",
                    );
                    desc.set_blending_enabled(true);

                    desc.set_rgb_blend_operation(MTLBlendOperation::Add);
                    desc.set_source_rgb_blend_factor(MTLBlendFactor::SourceAlpha);
                    desc.set_destination_rgb_blend_factor(MTLBlendFactor::OneMinusSourceAlpha);

                    desc.set_alpha_blend_operation(MTLBlendOperation::Add);
                    desc.set_source_alpha_blend_factor(MTLBlendFactor::SourceAlpha);
                    desc.set_destination_alpha_blend_factor(MTLBlendFactor::OneMinusSourceAlpha);

                    desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
                }

                unwrap_result_dcheck(
                    device.new_render_pipeline_state(&pipeline_state_desc),
                    "Failed to create render pipeline",
                )
            },
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
        delegate.update_view(
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
        encoder.set_depth_stencil_state(&self.depth_state);

        // TODO: START HERE 2
        // TODO: START HERE 2
        // TODO: START HERE 2
        // TODO: Only do this when there's a change (mouse drag + control key)
        let light_world_position = packed_float4::from(
            f32x4x4::rotate(self.light_xy_rotation[0], self.light_xy_rotation[1], 0.)
                * f32x4::from_array([0., 0., -LIGHT_DISTANCE, 1.]),
        );

        // Render Teapot
        {
            encoder.set_vertex_buffer(
                VertexBufferIndex_VertexBufferIndexIndices as _,
                Some(&self.vertex_buffer_indices),
                0,
            );
            encoder.set_vertex_buffer(
                VertexBufferIndex_VertexBufferIndexPositions as _,
                Some(&self.vertex_buffer_positions),
                0,
            );
            encoder.set_vertex_buffer(
                VertexBufferIndex_VertexBufferIndexNormals as _,
                Some(&self.vertex_buffer_normals),
                0,
            );
            encode_vertex_bytes(
                &encoder,
                VertexBufferIndex_VertexBufferIndexMatrixNormalToWorld,
                // IMPORTANT: In the shader, this maps to a float3x3. This works because...
                // 1. Conceptually, we want a matrix that ONLY applies rotation (no translation)
                //   - Since normals are directions (not positions), translations are meaningless and
                //     should not be applied.
                // 2. Memory layout-wise, float3x3 and float4x4 have the same size and alignment.
                //
                // TODO: Although this performs great (compare assembly running "asm proj-3-shading"
                //       task), this may be wayyy to tricky/error-prone/assumes-metal-ignores-the-extra-stuff.
                &self.matrix_model_to_world,
            );
            encode_vertex_bytes(
                &encoder,
                VertexBufferIndex_VertexBufferIndexMatrixModelToProjection,
                self.matrix_model_to_projection.metal_float4x4(),
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex_FragBufferIndexFragMode,
                &self.mode,
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex_FragBufferIndexMatrixProjectionToWorld,
                self.matrix_projection_to_world.metal_float4x4(),
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex_FragBufferIndexLightPosition,
                &light_world_position,
            );
            let camera_world_position = packed_float4::from(
                self.matrix_world_to_view.inverse() * f32x4::from_array([0., 0., 0., 1.]),
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex_FragBufferIndexCameraPosition,
                &camera_world_position,
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex_FragBufferIndexScreenSize,
                &self.screen_size,
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
            encoder.set_render_pipeline_state(&self.render_light_pipeline_state);
            encoder.set_vertex_buffers(
                0,
                &[None; VertexBufferIndex_VertexBufferIndexLENGTH as _],
                &[0; VertexBufferIndex_VertexBufferIndexLENGTH as _],
            );
            encoder.set_fragment_buffers(
                0,
                &[None; FragBufferIndex_FragBufferIndexLENGTH as _],
                &[0; FragBufferIndex_FragBufferIndexLENGTH as _],
            );
            encode_vertex_bytes(
                &encoder,
                LightVertexBufferIndex_LightVertexBufferIndexMatrixWorldToProjection,
                &self.matrix_world_to_projection,
            );
            encode_vertex_bytes(
                &encoder,
                LightVertexBufferIndex_LightVertexBufferIndexLightPosition,
                &light_world_position,
            );
            // TODO: Figure out a better way to unset this buffers from the previous draw call
            encoder.set_fragment_buffers(0, &[None; 4], &[0; 4]);
            encoder.draw_primitives(MTLPrimitiveType::Point, 0, 1);
        }
        encoder.end_encoding();
        command_buffer.present_drawable(drawable);
        command_buffer.commit();
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
                                Simd::from_array([
                                    ratio[1].atan(), // Rotation on x-axis
                                    ratio[0].atan(), // Rotation on y-axis
                                ])
                            }
                        }
                        Right => camera_distance += -drag_amount[1] / 8.0,
                    }
                    self.update_view(self.screen_size, camera_rotation, camera_distance);
                } else if modifier_keys.contains(ModifierKeys::CONTROL) {
                    match button {
                        Left => {
                            let adjacent = Simd::splat(self.camera_distance);
                            let opposite = -drag_amount / f32x2::splat(16.);
                            let ratio = opposite / adjacent;
                            self.light_xy_rotation += Simd::from_array([
                                ratio[1].atan(), // Rotation on x-axis
                                ratio[0].atan(), // Rotation on y-axis
                            ])
                        }
                        _ => {}
                    }
                }
            }
            KeyDown { key_code, .. } => {
                self.mode = match key_code {
                    29 /* 0 */ => FragMode_FragModeAmbientDiffuseSpecular,
                    18 /* 1 */ => FragMode_FragModeNormals,
                    19 /* 2 */ => FragMode_FragModeAmbient,
                    20 /* 3 */ => FragMode_FragModeAmbientDiffuse,
                    21 /* 4 */ => FragMode_FragModeSpecular,
                    _ => self.mode
                };
            }
            WindowResize { size, .. } => {
                self.update_depth_texture_size(size);
                self.update_view(size, self.camera_rotation, self.camera_distance);
            }
            _ => {}
        }
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 3 - Shading");
}
