#![feature(portable_simd)]
#![feature(slice_as_chunks)]
mod shader_bindings;

use bitflags::bitflags;
use metal_app::metal::*;
use metal_app::*;
use shader_bindings::*;
use std::{
    f32::consts::PI,
    path::{Path, PathBuf},
    simd::{f32x2, f32x4},
};
use tobj::{LoadOptions, Mesh};

bitflags! {
    struct Mode: u16 {
        const HAS_AMBIENT = 1 << FC_FC_HAS_AMBIENT;
        const HAS_DIFFUSE = 1 << FC_FC_HAS_DIFFUSE;
        const HAS_NORMAL = 1 << FC_FC_HAS_NORMAL;
        const HAS_SPECULAR = 1 << FC_FC_HAS_SPECULAR;
        const DEFAULT = Self::HAS_AMBIENT.bits | Self::HAS_DIFFUSE.bits | Self::HAS_SPECULAR.bits;
    }
}

const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth32Float;
const INITIAL_CAMERA_DISTANCE: f32 = 50.;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 4., 0.]);
const INITIAL_MODE: Mode = Mode::DEFAULT;
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_DISTANCE: f32 = INITIAL_CAMERA_DISTANCE / 2.;

fn load_texture_from_png<T: AsRef<Path>>(label: &str, path_to_png: T, device: &Device) -> Texture {
    use png::ColorType::*;
    use std::fs::File;
    let mut decoder = png::Decoder::new(File::open(path_to_png).unwrap());
    decoder.set_transformations(png::Transformations::normalize_to_color8());

    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let width = info.width as _;
    let height = info.height as _;
    assert_eq!(
        info.color_type, Rgba,
        "Unexpected PNG format, expected RGBA"
    );

    let desc = TextureDescriptor::new();

    desc.set_width(width);
    desc.set_height(height);
    desc.set_pixel_format(MTLPixelFormat::RGBA8Unorm);
    desc.set_storage_mode(MTLStorageMode::Shared);
    desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
    desc.set_usage(MTLTextureUsage::ShaderRead);

    let texture = device.new_texture(&desc);
    texture.set_label(label);
    texture.replace_region(
        MTLRegion {
            origin: MTLOrigin { x: 0, y: 0, z: 0 },
            size: MTLSize {
                width,
                height,
                depth: 1,
            },
        },
        0,
        buf.as_ptr() as _,
        width * 4,
    );
    texture
}

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
    max_bound: f32,
    mode: Mode,
    num_triangles: usize,
    render_light_pipeline_state: RenderPipelineState,
    render_pipeline_state: RenderPipelineState,
    screen_size: f32x2,
    specular_shineness: f32,
    vertex_buffer_indices: Buffer,
    vertex_buffer_normals: Buffer,
    vertex_buffer_positions: Buffer,
    vertex_buffer_texcoords: Buffer,
    texture_specular: Texture,
    texture_ambient_diffuse: Texture,
}

fn create_pipelines(
    device: &Device,
    library: &Library,
    mode: Mode,
    specular_shineness: f32,
) -> (RenderPipelineState, RenderPipelineState) {
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
        FC_FC_HAS_AMBIENT,
        FC_FC_HAS_DIFFUSE,
        FC_FC_HAS_SPECULAR,
        FC_FC_HAS_NORMAL,
    ] {
        function_constants.set_constant_value_at_index(
            (&mode.contains(Mode::from_bits_truncate(1 << index)) as *const _) as _,
            MTLDataType::Bool,
            index as _,
        );
    }
    function_constants.set_constant_value_at_index(
        (&specular_shineness as *const _) as _,
        MTLDataType::Float,
        FC_FC_SPECULAR_SHINENESS as _,
    );
    (
        create_pipeline_with_constants(
            &device,
            &library,
            &base_pipeline_desc,
            "Teapot",
            Some(&function_constants),
            &"main_vertex",
            VertexBufferIndex_VertexBufferIndex_LENGTH,
            &"main_fragment",
            FragBufferIndex_FragBufferIndex_LENGTH,
        ),
        create_pipeline_with_constants(
            &device,
            &library,
            &base_pipeline_desc,
            "Light",
            Some(&function_constants),
            &"light_vertex",
            LightVertexBufferIndex_LightVertexBufferIndex_LENGTH,
            &"light_fragment",
            0,
        ),
    )
}

impl RendererDelgate for Delegate {
    fn new(device: Device, _command_queue: &CommandQueue) -> Self {
        let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
        let teapot_file = assets_dir.join("teapot.obj");
        let (mut models, materials) = tobj::load_obj(
            teapot_file,
            &LoadOptions {
                single_index: true,
                triangulate: true,
                ignore_points: true,
                ignore_lines: true,
            },
        )
        .expect("Failed to load OBJ file");

        let material = materials
            .expect("Failed to load materials data")
            .pop()
            .expect("Failed to load material, expected atleast one material");
        let specular_shineness = material.shininess;
        let texture_ambient_diffuse = load_texture_from_png(
            "Ambient/Diffuse",
            assets_dir.join(material.ambient_texture),
            &device,
        );
        let texture_specular = load_texture_from_png(
            "Specular",
            assets_dir.join(material.specular_texture),
            &device,
        );

        let model = models
            .pop()
            .expect("Failed to parse model, expecting atleast one model (teapot)");
        let Mesh {
            positions,
            indices,
            normals,
            texcoords,
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
        debug_assert_eq!(
            texcoords.len() % 2,
            0,
            "`mesh.texcoords` should contain pairs (UV coordinates)"
        );
        debug_assert_eq!(
            texcoords.len() / 2,
            positions.len() / 3,
            "`mesh.texcoords` shoud contain UV coordinate for each position"
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

        let mode = INITIAL_MODE;
        let (render_pipeline_state, render_light_pipeline_state) =
            create_pipelines(&device, &library, mode, specular_shineness);
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
            matrix_model_to_world,
            matrix_projection_to_world: f32x4x4::identity(),
            matrix_world_to_camera: f32x4x4::identity(),
            matrix_world_to_projection: f32x4x4::identity(),
            max_bound,
            mode,
            num_triangles: indices.len() / 3,
            render_pipeline_state,
            render_light_pipeline_state,
            screen_size: f32x2::default(),
            specular_shineness,
            texture_specular,
            texture_ambient_diffuse,
            vertex_buffer_indices: allocate_new_buffer_with_data(&device, "Indices", &indices),
            vertex_buffer_normals: allocate_new_buffer_with_data(&device, "Normals", &normals),
            vertex_buffer_positions: allocate_new_buffer_with_data(
                &device,
                "Positions",
                &positions,
            ),
            vertex_buffer_texcoords: allocate_new_buffer_with_data(
                &device,
                "Texcoords",
                &texcoords,
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

        let light_world_position = float4::from(self.light_world_position);

        // Render Teapot
        {
            encoder.set_vertex_buffer(
                VertexBufferIndex_VertexBufferIndex_Indices as _,
                Some(&self.vertex_buffer_indices),
                0,
            );
            encoder.set_vertex_buffer(
                VertexBufferIndex_VertexBufferIndex_Positions as _,
                Some(&self.vertex_buffer_positions),
                0,
            );
            encoder.set_vertex_buffer(
                VertexBufferIndex_VertexBufferIndex_Normals as _,
                Some(&self.vertex_buffer_normals),
                0,
            );
            encoder.set_vertex_buffer(
                VertexBufferIndex_VertexBufferIndex_Texcoords as _,
                Some(&self.vertex_buffer_texcoords),
                0,
            );
            encode_vertex_bytes(
                &encoder,
                VertexBufferIndex_VertexBufferIndex_MatrixNormalToWorld,
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
                VertexBufferIndex_VertexBufferIndex_MatrixModelToProjection,
                self.matrix_model_to_projection.metal_float4x4(),
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex_FragBufferIndex_MatrixProjectionToWorld,
                self.matrix_projection_to_world.metal_float4x4(),
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex_FragBufferIndex_ScreenSize,
                &float2::from(self.screen_size),
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex_FragBufferIndex_LightPosition,
                // IMPORTANT: In the shader, this maps to a float3. This works because the float4
                // and float3 have the same size and alignment.
                &light_world_position,
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex_FragBufferIndex_CameraPosition,
                // IMPORTANT: In the shader, this maps to a float3. This works because the float4
                // and float3 have the same size and alignment.
                &float4::from(self.camera_world_position),
            );
            encoder.set_fragment_texture(
                FragBufferIndex_FragBufferIndex_AmbientTexture as _,
                Some(&self.texture_ambient_diffuse),
            );
            encoder.set_fragment_texture(
                FragBufferIndex_FragBufferIndex_Specular as _,
                Some(&self.texture_specular),
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
                &[None; VertexBufferIndex_VertexBufferIndex_LENGTH as _],
                &[0; VertexBufferIndex_VertexBufferIndex_LENGTH as _],
            );
            encoder.set_fragment_buffers(
                0,
                &[None; FragBufferIndex_FragBufferIndex_LENGTH as _],
                &[0; FragBufferIndex_FragBufferIndex_LENGTH as _],
            );
            encoder.set_render_pipeline_state(&self.render_light_pipeline_state);
            encode_vertex_bytes(
                &encoder,
                LightVertexBufferIndex_LightVertexBufferIndex_MatrixWorldToProjection,
                self.matrix_world_to_projection.metal_float4x4(),
            );
            encode_vertex_bytes(
                &encoder,
                LightVertexBufferIndex_LightVertexBufferIndex_LightPosition,
                &light_world_position,
            );
            encoder.draw_primitives(MTLPrimitiveType::Point, 0, 1);
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
            (self.render_pipeline_state, self.render_light_pipeline_state) =
                create_pipelines(&self.device, &self.library, mode, self.specular_shineness);
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
        let n = 0.1;
        let f = 1000.0;
        let perspective_matrix = f32x4x4::new(
            [n, 0., 0., 0.],
            [0., n, 0., 0.],
            [0., 0., n + f, -n * f],
            [0., 0., 1., 0.],
        );
        let w = 2. * n * self.max_bound / INITIAL_CAMERA_DISTANCE;
        let h = w / aspect_ratio;
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
        let aspect_ratio = screen_size[0] / screen_size[1];
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
