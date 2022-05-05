#![feature(portable_simd)]
#![feature(slice_as_chunks)]
mod shader_bindings;

use metal_app::{
    allocate_new_buffer_with_data, encode_fragment_bytes, encode_vertex_bytes, f32x4x4,
    launch_application, metal::*, unwrap_option_dcheck, unwrap_result_dcheck, Position,
    RendererDelgate, Size, Unit, UserEvent,
};
use shader_bindings::{
    FragBufferIndex_FragBufferIndexFragMode, FragBufferIndex_FragBufferIndexInverseProjection,
    FragBufferIndex_FragBufferIndexScreenSize, FragMode, FragMode_FragModeAmbient,
    FragMode_FragModeAmbientDiffuse, FragMode_FragModeAmbientDiffuseSpecular,
    FragMode_FragModeNormals, FragMode_FragModeSpecular,
    VertexBufferIndex_VertexBufferIndexIndices,
    VertexBufferIndex_VertexBufferIndexModelViewProjection,
    VertexBufferIndex_VertexBufferIndexNormalTransform, VertexBufferIndex_VertexBufferIndexNormals,
    VertexBufferIndex_VertexBufferIndexPositions,
};
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, Simd},
};
use tobj::{LoadOptions, Mesh};

const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth32Float;
const INITIAL_CAMERA_DISTANCE: f32 = 50.0;
const INITIAL_MODE: FragMode = FragMode_FragModeAmbientDiffuseSpecular;

struct Delegate {
    aspect_ratio: f32,
    camera_distance: Unit,
    camera_rotation: f32x2,
    depth_state: DepthStencilState,
    depth_texture: Option<Texture>,
    device: Device,
    max_bound: f32,
    mode: FragMode,
    model_matrix: f32x4x4,
    model_view_projection_matrix: f32x4x4,
    normal_transform_matrix: f32x4x4,
    num_triangles: usize,
    projection_inverse_matrix: f32x4x4,
    render_pipeline_state: RenderPipelineState,
    screen_size: Size,
    vertex_buffer_indices: Buffer,
    vertex_buffer_normals: Buffer,
    vertex_buffer_positions: Buffer,
    view_matrix: f32x4x4,
}

impl Delegate {
    fn update_depth_texture_size(&mut self, size: Size) {
        let desc = TextureDescriptor::new();
        desc.set_width(size[0] as _);
        desc.set_height(size[1] as _);
        desc.set_pixel_format(DEPTH_TEXTURE_FORMAT);
        desc.set_storage_mode(MTLStorageMode::Memoryless);
        desc.set_usage(MTLTextureUsage::RenderTarget);
        self.depth_texture = Some(self.device.new_texture(&desc));
    }

    // // TODO: This doesn't allow for a full 360 degree rotation in one drag (atan is [-90, 90]).
    #[inline]
    fn calc_rotation_offset(&self, down_position: Position, position: Position) -> Position {
        let adjacent = Simd::splat(self.camera_distance);
        let offsets = down_position - position;
        let ratio = offsets / adjacent;
        Simd::from_array([
            ratio[1].atan(), // Rotation on x-axis
            ratio[0].atan(), // Rotation on y-axis
        ])
    }

    #[inline]
    fn on_camera_change(
        &mut self,
        camera_rotation: Size,
        camera_rotation_offset: Size,
        camera_distance: Unit,
        camera_distance_offset: Unit,
    ) {
        self.camera_rotation = camera_rotation;
        self.camera_distance = camera_distance;

        let rot = self.camera_rotation + camera_rotation_offset;
        let view_rotation_matrix = f32x4x4::rotate(-rot[0], -rot[1], 0.);
        let view_translate_matrix =
            f32x4x4::translate(0., 0., self.camera_distance + camera_distance_offset);

        self.normal_transform_matrix = view_rotation_matrix * self.model_matrix.zero_translate();
        self.view_matrix = view_translate_matrix * view_rotation_matrix;

        self.update_model_view_projection_matrix();
    }

    fn on_screen_size_change(&mut self, size: Size) {
        self.screen_size = size;
        self.aspect_ratio = size[0] / size[1];
        self.update_model_view_projection_matrix();
    }

    #[inline]
    fn calc_projection_matrix(&self, aspect_ratio: f32) -> f32x4x4 {
        let n = 0.1;
        let f = 1000.0;
        let camera_distance = INITIAL_CAMERA_DISTANCE;

        let perspective_matrix = f32x4x4::new(
            [n, 0., 0., 0.],
            [0., n, 0., 0.],
            [0., 0., n + f, -n * f],
            [0., 0., 1., 0.],
        );
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

    #[inline]
    fn update_model_view_projection_matrix(&mut self) {
        let projection_matrix = self.calc_projection_matrix(self.aspect_ratio);
        self.model_view_projection_matrix =
            projection_matrix * self.view_matrix * self.model_matrix;
        self.projection_inverse_matrix = projection_matrix.inverse();
    }
}

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
            r#"`mesh.indices` should contain triples (triangle vertices). Model should have been loaded with `triangulate`, guaranteeing all faces have 3 vertices."#
        );
        debug_assert_eq!(
            positions.len() % 3,
            0,
            r#"`mesh.positions` should contain triples (3D position)"#
        );
        debug_assert_eq!(
            normals.len(),
            positions.len(),
            r#"`mesh.normals` should contain triples (3D vector)"#
        );

        let (positions3, ..) = positions.as_chunks::<3>();
        let mut mins = Simd::splat(f32::MAX);
        let mut maxs = Simd::splat(f32::MIN);
        for &[x, y, z] in positions3 {
            let input = Simd::from_array([x, y, z, 0.0]);
            mins = mins.min(input);
            maxs = maxs.max(input);
        }
        let max_bound = mins.reduce_min().abs().max(maxs.reduce_max());

        let height_of_teapot = maxs[2] - mins[2];
        let model_matrix = {
            let r = f32x4x4::x_rotate(std::f32::consts::PI / 2.);
            let t = f32x4x4::translate(0., 0., -height_of_teapot / 2.0);
            r * t
        };

        let camera_distance = INITIAL_CAMERA_DISTANCE;
        let camera_rotation = Simd::from_array([-PI / 6.0, 0.0]);
        let mut delegate = Self {
            aspect_ratio: 1.0,
            camera_distance,
            camera_rotation,
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: None,
            max_bound,
            mode: INITIAL_MODE,
            model_matrix,
            model_view_projection_matrix: f32x4x4::identity(),
            normal_transform_matrix: f32x4x4::identity(),
            projection_inverse_matrix: f32x4x4::identity(),
            num_triangles: indices.len() / 3,
            render_pipeline_state: {
                let library = device
                    .new_library_with_data(include_bytes!(concat!(
                        env!("OUT_DIR"),
                        "/shaders.metallib"
                    )))
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
                    for &buffer_index in &[
                        VertexBufferIndex_VertexBufferIndexIndices,
                        VertexBufferIndex_VertexBufferIndexPositions,
                        VertexBufferIndex_VertexBufferIndexNormals,
                        VertexBufferIndex_VertexBufferIndexModelViewProjection,
                    ] {
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
            screen_size: Size::default(),
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
            view_matrix: f32x4x4::identity(),
            device,
        };

        delegate.on_camera_change(
            delegate.camera_rotation,
            Simd::splat(0.),
            delegate.camera_distance,
            0.,
        );
        delegate.on_resize(Size::splat(1.));
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
            VertexBufferIndex_VertexBufferIndexNormalTransform,
            &self.normal_transform_matrix.metal_float3x3_upper_left(),
        );
        encode_vertex_bytes(
            &encoder,
            VertexBufferIndex_VertexBufferIndexModelViewProjection,
            self.model_view_projection_matrix.metal_float4x4(),
        );
        encode_fragment_bytes(
            &encoder,
            FragBufferIndex_FragBufferIndexFragMode,
            &self.mode,
        );
        encode_fragment_bytes(
            &encoder,
            FragBufferIndex_FragBufferIndexInverseProjection,
            self.projection_inverse_matrix.metal_float4x4(),
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
        encoder.end_encoding();
        command_buffer.present_drawable(drawable);
        command_buffer.commit();
    }

    fn on_event(&mut self, event: UserEvent) {
        use metal_app::MouseButton::*;
        use UserEvent::*;
        fn calc_distance_offset(down_position: Position, position: Position) -> Unit {
            // Dragging up   => zooms in  (-offset)
            // Dragging down => zooms out (+offset)
            let screen_offset = position[1] - down_position[1];
            screen_offset / 8.0
        }
        let mut camera_rotation = self.camera_rotation;
        let mut camera_rotation_offset = Size::splat(0.);
        let mut camera_distance = self.camera_distance;
        let mut camera_distance_offset = 0.;
        match event {
            MouseDrag {
                button,
                position,
                down_position,
            } => match button {
                Left => {
                    camera_rotation_offset = self.calc_rotation_offset(down_position, position);
                }
                Right => {
                    camera_distance_offset = calc_distance_offset(down_position, position);
                }
            },
            MouseUp {
                button,
                position,
                down_position,
            } => match button {
                Left => {
                    camera_rotation_offset = Simd::default();
                    camera_rotation += self.calc_rotation_offset(down_position, position);
                }
                Right => {
                    camera_distance_offset = 0.0;
                    camera_distance += calc_distance_offset(down_position, position);
                }
            },
            KeyDown { key_code } => {
                self.mode = match key_code {
                    29 /* 0 */ => FragMode_FragModeAmbientDiffuseSpecular,
                    18 /* 1 */ => FragMode_FragModeNormals,
                    19 /* 2 */ => FragMode_FragModeAmbient,
                    20 /* 3 */ => FragMode_FragModeAmbientDiffuse,
                    21 /* 4 */ => FragMode_FragModeSpecular,
                    _ => self.mode
                };
            }
            _ => {}
        }
        self.on_camera_change(
            camera_rotation,
            camera_rotation_offset,
            camera_distance,
            camera_distance_offset,
        );
    }

    #[inline]
    fn on_resize(&mut self, size: Size) {
        self.update_depth_texture_size(size);
        self.on_screen_size_change(size);
        // self.update_model_view_projection_matrix();
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 3 - Shading");
}
