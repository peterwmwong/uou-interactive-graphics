#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::{Camera, ShadingModeSelector},
    image_helpers,
    metal::*,
    metal_types::*,
    *,
};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    ops::Neg,
    path::PathBuf,
    simd::{f32x2, f32x4},
};

const CUBEMAP_TEXTURE_BYTES_PER_PIXEL: u32 = 4; // Assumed to be 4-component (ex. RGBA)
const CUBEMAP_TEXTURE_WIDTH: u32 = 2048;
const CUBEMAP_TEXTURE_HEIGHT: u32 = CUBEMAP_TEXTURE_WIDTH;
const CUBEMAP_TEXTURE_BYTES_PER_ROW: u32 = CUBEMAP_TEXTURE_WIDTH * CUBEMAP_TEXTURE_BYTES_PER_PIXEL;
const CUBEMAP_TEXTURE_BYTES_PER_FACE: u32 = CUBEMAP_TEXTURE_HEIGHT * CUBEMAP_TEXTURE_BYTES_PER_ROW;
const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 32., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

struct Delegate {
    bg_depth_state: DepthStencilState,
    bg_pipeline_state: RenderPipelineState,
    camera: Camera,
    camera_space: ProjectedSpace,
    command_queue: CommandQueue,
    cubemap_texture: Texture,
    depth_texture: Option<Texture>,
    device: Device,
    library: Library,
    matrix_model_to_world: f32x4x4,
    matrix_world_to_mirror_world: f32x4x4,
    mirrored_model_texture: Option<Texture>,
    mirror_plane_pipeline: RenderPipelineState,
    mirror_plane_y_world: f32,
    model_depth_state: DepthStencilState,
    model_pipeline: RenderPipelineState,
    model: Model<{ VertexBufferIndex::Geometry as _ }, { NO_MATERIALS_ID }>,
    needs_render: bool,
    shading_mode: ShadingModeSelector,
}

fn create_pipelines(
    device: &Device,
    library: &Library,
    mode: ShadingModeSelector,
) -> (RenderPipelineState, RenderPipelineState) {
    let function_constants = mode.encode(
        FunctionConstantValues::new(),
        ShadingMode::HasAmbient as _,
        ShadingMode::HasDiffuse as _,
        ShadingMode::HasSpecular as _,
        ShadingMode::OnlyNormals as _,
    );
    (
        // Model Pipeline
        {
            let p = create_render_pipeline(
                &device,
                &new_render_pipeline_descriptor(
                    "Model",
                    &library,
                    Some((DEFAULT_PIXEL_FORMAT, false)),
                    Some(DEPTH_TEXTURE_FORMAT),
                    Some(&function_constants),
                    Some((&"main_vertex", VertexBufferIndex::LENGTH as _)),
                    Some((&"main_fragment", FragBufferIndex::LENGTH as _)),
                ),
            );
            use debug_assert_pipeline_function_arguments::*;
            debug_assert_render_pipeline_function_arguments(
                &p,
                &[
                    value_arg::<Geometry>(VertexBufferIndex::Geometry as _),
                    value_arg::<ProjectedSpace>(VertexBufferIndex::Camera as _),
                    value_arg::<ModelSpace>(VertexBufferIndex::Model as _),
                ],
                Some(&[
                    value_arg::<ProjectedSpace>(FragBufferIndex::Camera as _),
                    value_arg::<float4>(FragBufferIndex::LightPosition as _),
                    value_arg::<float3x3>(FragBufferIndex::MatrixEnvironment as _),
                    texture_arg(FragTextureIndex::EnvTexture as _, MTLTextureType::Cube),
                ]),
            );
            p.pipeline_state
        },
        // Plane Pipeline
        {
            let p = create_render_pipeline(
                &device,
                &new_render_pipeline_descriptor(
                    "Plane",
                    &library,
                    Some((DEFAULT_PIXEL_FORMAT, false)),
                    Some(DEPTH_TEXTURE_FORMAT),
                    Some(&function_constants),
                    Some((&"plane_vertex", VertexBufferIndex::LENGTH as _)),
                    Some((&"plane_fragment", FragBufferIndex::LENGTH as _)),
                ),
            );
            use debug_assert_pipeline_function_arguments::*;
            debug_assert_render_pipeline_function_arguments(
                &p,
                &[
                    value_arg::<ProjectedSpace>(VertexBufferIndex::Camera as _),
                    value_arg::<f32>(VertexBufferIndex::PlaneY as _),
                ],
                Some(&[
                    value_arg::<ProjectedSpace>(FragBufferIndex::Camera as _),
                    value_arg::<float4>(FragBufferIndex::LightPosition as _),
                    texture_arg(FragTextureIndex::EnvTexture as _, MTLTextureType::Cube),
                    texture_arg(FragTextureIndex::ModelTexture as _, MTLTextureType::D2),
                ]),
            );
            p.pipeline_state
        },
    )
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let executable_name = std::env::args()
            .nth(0)
            .expect("Failed to access command line executable name");
        let model_file_path = std::env::args().nth(1).expect(&format!(
            "Usage: {executable_name} [Path to Wavefront OBJ file]"
        ));

        // Load Environment Map (Cube Map)
        let env_texture = {
            let desc = TextureDescriptor::new();
            desc.set_pixel_format(MTLPixelFormat::RGBA8Unorm);
            desc.set_texture_type(MTLTextureType::Cube);
            desc.set_compression_type(MTLTextureCompressionType::Lossless);
            desc.set_resource_options(DEFAULT_RESOURCE_OPTIONS);
            desc.set_usage(MTLTextureUsage::ShaderRead);
            // TODO: Remove hardcoded values, use PNG dimensions
            desc.set_width(CUBEMAP_TEXTURE_WIDTH as _);
            desc.set_height(CUBEMAP_TEXTURE_HEIGHT as _);
            desc.set_depth(1);

            let texture = device.new_texture(&desc);
            let cubemap_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("cubemap");
            let mut buffer = vec![];
            debug_time("proj6 - Load Environment Cube Texture", || {
                for (face_index, filename) in [
                    "cubemap_posx.png",
                    "cubemap_negx.png",
                    "cubemap_posy.png",
                    "cubemap_negy.png",
                    "cubemap_posz.png",
                    "cubemap_negz.png",
                ]
                .iter()
                .enumerate()
                {
                    let (bytes, (width, height)) = image_helpers::read_png_pixel_bytes_into(
                        cubemap_path.join(filename),
                        &mut buffer,
                    );
                    assert_eq!(width, CUBEMAP_TEXTURE_WIDTH);
                    assert_eq!(height, CUBEMAP_TEXTURE_HEIGHT);
                    assert_eq!(
                        bytes, CUBEMAP_TEXTURE_BYTES_PER_FACE as _,
                        "Unexpected number of bytes read for cube map texture"
                    );
                    texture.replace_region_in_slice(
                        MTLRegion {
                            origin: MTLOrigin { x: 0, y: 0, z: 0 },
                            size: MTLSize {
                                width: CUBEMAP_TEXTURE_WIDTH as _,
                                height: CUBEMAP_TEXTURE_WIDTH as _,
                                depth: 1,
                            },
                        },
                        0,
                        face_index as _,
                        buffer.as_ptr() as _,
                        CUBEMAP_TEXTURE_BYTES_PER_ROW as _,
                        CUBEMAP_TEXTURE_BYTES_PER_FACE as _,
                    );
                }
            });
            texture
        };

        let model_file = PathBuf::from(model_file_path);
        let model = Model::from_file(
            model_file,
            &device,
            |arg: &mut Geometry,
             GeometryToEncode {
                 indices_buffer,
                 positions_buffer,
                 normals_buffer,
                 tx_coords_buffer,
             }| {
                arg.indices = indices_buffer;
                arg.positions = positions_buffer;
                arg.normals = normals_buffer;
                arg.tx_coords = tx_coords_buffer;
            },
            NO_MATERIALS_ENCODER,
        );
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");

        let bg_pipeline = {
            let p = create_render_pipeline(
                &device,
                &new_render_pipeline_descriptor(
                    "BG",
                    &library,
                    Some((DEFAULT_PIXEL_FORMAT, false)),
                    Some(DEPTH_TEXTURE_FORMAT),
                    Some(&FunctionConstantValues::new()),
                    Some((&"bg_vertex", 0)),
                    Some((&"bg_fragment", FragBufferIndex::LENGTH as _)),
                ),
            );
            use debug_assert_pipeline_function_arguments::*;
            debug_assert_render_pipeline_function_arguments(
                &p,
                &[],
                Some(&[
                    value_arg::<ProjectedSpace>(FragBufferIndex::Camera as _),
                    texture_arg(FragTextureIndex::EnvTexture as _, MTLTextureType::Cube),
                ]),
            );
            p
        };
        let shading_mode = ShadingModeSelector::DEFAULT;
        let (model_pipeline, mirror_plane_pipeline) =
            create_pipelines(&device, &library, shading_mode);

        let &MaxBounds { center, size } = &model.geometry_max_bounds;
        let &[cx, cy, cz, _] = center.neg().as_array();

        // IMPORTANT: Normalize the world coordinates to a reasonable range ~[0, 1].
        // 1. Camera distance is invariant of the model's coordinate range
        // 2. Dramatically reduces precision errors (compared to ranges >1000, like in Yoda model)
        //    - In the Vertex Shader, z-fighting in the depth buffer, even with Depth32Float.
        //    - In the Fragment Shader, diffuse and specular lighting is no longer smooth and
        //      exhibit a weird triangal-ish pattern.
        let scale = 1. / size.reduce_max();

        // TODO: This generates an immense amount of code!
        // - It's the matrix multiplications we're unable to avoid with const evaluation (currently not supported in rust for floating point operations)
        // - We can create combo helpers, see f32x4x4::scale_translate()
        let matrix_model_to_world = (f32x4x4::scale(scale, scale, scale, 1.)
            * (f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.)))
            * f32x4x4::translate(cx, cy, cz);

        let mirror_plane_y_world = -0.5 * scale * size[2];
        let matrix_world_to_mirror_world =
            // 3. Put the object back into world coordinate space (mirror world)
            f32x4x4::translate(0., mirror_plane_y_world, 0.)
            // 2. Mirror
            * f32x4x4::scale(1., -1., 1., 1.)
            // 1. Move objects into mirror plane coordinate space (aka origin is mirror plane)
            * f32x4x4::translate(0., -mirror_plane_y_world, 0.);

        // Interesting observation: This transformation is an involutory matrix?
        // TODO: How does this work? Does this hold up when we rotate the mirror plane?
        // let matrix_mirror_world_to_world = matrix_world_to_mirror_world.inverse();
        // assert_eq!(matrix_world_to_mirror_world, matrix_mirror_world_to_world);

        Self {
            bg_depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(false);
                device.new_depth_stencil_state(&desc)
            },
            bg_pipeline_state: bg_pipeline.pipeline_state,
            camera: Camera::new_with_default_distance(
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            camera_space: ProjectedSpace {
                matrix_world_to_projection: f32x4x4::identity(),
                matrix_screen_to_world: f32x4x4::identity(),
                position_world: f32x4::default().into(),
            },
            command_queue: device.new_command_queue(),
            cubemap_texture: env_texture,
            depth_texture: None,
            library,
            matrix_model_to_world,
            matrix_world_to_mirror_world,
            mirrored_model_texture: None,
            mirror_plane_pipeline,
            mirror_plane_y_world,
            model_depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            model_pipeline,
            model,
            needs_render: false,
            shading_mode,
            device,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.needs_render = false;

        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        command_buffer.set_label("Renderer Command Buffer");
        let light_position = f32x4::from_array([0., 1., -1., 1.]);
        {
            let encoder = command_buffer.new_render_command_encoder(new_render_pass_descriptor(
                self.mirrored_model_texture.as_deref(),
                self.depth_texture
                    .as_ref()
                    .map(|d| (d, MTLStoreAction::DontCare)),
            ));
            {
                let mirror_camera_space = ProjectedSpace {
                    matrix_world_to_projection: self.camera_space.matrix_world_to_projection
                        * self.matrix_world_to_mirror_world,
                    // TODO: I'm not sure this is right, shouldn't it be matrix_mirror_world_to_world, not matrix_world_to_mirror_world
                    //                                                          aaaaaaaaaaaa    bbbbb             bbbbb    aaaaaaaaaaaa
                    // - I think this only works because matrix_world_to_mirror_world is involution (inverse is the same).
                    // - `matrix_world_to_projection` (Metal than transforms to screen)
                    //      world -> mirror world -> projection -> screen
                    // - Also, does the Fragment Shader need a mirror world or world coordinate?
                    //      - `matrix_screen_to_world`
                    //          screen -> projection -> world -> mirror world (current)
                    //          VS.
                    //          screen -> projection -> mirror world -> world
                    matrix_screen_to_world: self.matrix_world_to_mirror_world
                        * self.camera_space.matrix_screen_to_world,
                    position_world: self.camera_space.position_world,
                };
                encoder.push_debug_group("Model (mirrored)");
                self.model.encode_use_resources(encoder);
                encoder.set_render_pipeline_state(&self.model_pipeline);
                encoder.set_depth_stencil_state(&self.model_depth_state);
                encode_vertex_bytes(
                    encoder,
                    VertexBufferIndex::Camera as _,
                    &mirror_camera_space,
                );
                encode_vertex_bytes(
                    encoder,
                    VertexBufferIndex::Model as _,
                    &ModelSpace {
                        matrix_model_to_projection: mirror_camera_space.matrix_world_to_projection
                            * self.matrix_model_to_world,
                        matrix_normal_to_world: (self.matrix_world_to_mirror_world
                            * self.matrix_model_to_world)
                            .into(),
                    },
                );
                encode_vertex_bytes(
                    encoder,
                    VertexBufferIndex::PlaneY as _,
                    &self.mirror_plane_y_world,
                );
                encode_fragment_bytes(encoder, FragBufferIndex::Camera as _, &mirror_camera_space);
                encode_fragment_bytes(
                    encoder,
                    FragBufferIndex::LightPosition as _,
                    &(self.matrix_world_to_mirror_world * light_position),
                );
                encode_fragment_bytes(
                    encoder,
                    FragBufferIndex::MatrixEnvironment as _,
                    &Into::<float3x3>::into(self.matrix_world_to_mirror_world),
                );
                encoder.set_fragment_texture(
                    FragTextureIndex::EnvTexture as _,
                    Some(&self.cubemap_texture),
                );
                self.model.encode_draws(encoder);
                encoder.pop_debug_group();
            }
            encoder.end_encoding();
        }
        let encoder = command_buffer.new_render_command_encoder(new_render_pass_descriptor(
            Some(render_target),
            self.depth_texture
                .as_ref()
                .map(|d| (d, MTLStoreAction::DontCare)),
        ));
        {
            encoder.push_debug_group("Model");
            self.model.encode_use_resources(encoder);
            encoder.set_render_pipeline_state(&self.model_pipeline);
            encoder.set_depth_stencil_state(&self.model_depth_state);
            encode_vertex_bytes(encoder, VertexBufferIndex::Camera as _, &self.camera_space);
            encode_vertex_bytes(
                encoder,
                VertexBufferIndex::Model as _,
                &ModelSpace {
                    matrix_model_to_projection: self.camera_space.matrix_world_to_projection
                        * self.matrix_model_to_world,
                    matrix_normal_to_world: self.matrix_model_to_world.into(),
                },
            );
            encode_fragment_bytes(encoder, FragBufferIndex::Camera as _, &self.camera_space);
            encode_fragment_bytes(
                encoder,
                FragBufferIndex::LightPosition as _,
                &light_position,
            );
            encode_fragment_bytes(
                encoder,
                FragBufferIndex::MatrixEnvironment as _,
                &Into::<float3x3>::into(f32x4x4::identity()),
            );
            encoder.set_fragment_texture(
                FragTextureIndex::EnvTexture as _,
                Some(&self.cubemap_texture),
            );
            self.model.encode_draws(encoder);
            encoder.pop_debug_group();
        }
        {
            encoder.push_debug_group("Plane");
            encoder.set_render_pipeline_state(&self.mirror_plane_pipeline);
            encode_vertex_bytes(
                encoder,
                VertexBufferIndex::PlaneY as _,
                &self.mirror_plane_y_world,
            );
            encoder.set_fragment_texture(
                FragTextureIndex::ModelTexture as _,
                self.mirrored_model_texture.as_deref(),
            );
            encoder.draw_primitives(MTLPrimitiveType::TriangleStrip, 0, 4);
            encoder.pop_debug_group();
        }
        {
            encoder.push_debug_group("BG");
            encoder.set_render_pipeline_state(&self.bg_pipeline_state);
            encoder.set_depth_stencil_state(&self.bg_depth_state);
            encoder.draw_primitives(MTLPrimitiveType::TriangleStrip, 0, 3);
            encoder.pop_debug_group();
        }
        encoder.end_encoding();
        command_buffer
    }

    #[inline]
    fn on_event(&mut self, event: UserEvent) {
        if let Some(update) = self.camera.on_event(event) {
            self.camera_space.matrix_world_to_projection = update.matrix_world_to_projection;
            self.camera_space.matrix_screen_to_world = update.matrix_screen_to_world;
            self.camera_space.position_world = update.position_world.into();

            self.needs_render = true;
        }

        if self.shading_mode.on_event(event) {
            self.update_mode();
        }

        match event {
            UserEvent::WindowFocusedOrResized { size } => {
                self.update_textures_size(size);
                self.needs_render = true;
            }
            _ => {}
        }
    }

    #[inline(always)]
    fn needs_render(&self) -> bool {
        self.needs_render
    }

    fn device(&self) -> &Device {
        &self.device
    }
}

impl Delegate {
    #[inline]
    fn update_textures_size(&mut self, size: f32x2) {
        let desc = TextureDescriptor::new();
        let &[x, y] = size.as_array();
        desc.set_width(x as _);
        desc.set_height(y as _);
        desc.set_pixel_format(DEPTH_TEXTURE_FORMAT);
        desc.set_storage_mode(MTLStorageMode::Private);
        desc.set_usage(MTLTextureUsage::RenderTarget);
        let texture = self.device.new_texture(&desc);
        texture.set_label("Depth");
        self.depth_texture = Some(texture);

        desc.set_pixel_format(DEFAULT_PIXEL_FORMAT);
        desc.set_usage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
        let texture = self.device.new_texture(&desc);
        texture.set_label("Model (mirrored)");
        self.mirrored_model_texture = Some(texture);
    }

    #[inline]
    fn update_mode(&mut self) {
        (self.model_pipeline, self.mirror_plane_pipeline) =
            create_pipelines(&self.device, &self.library, self.shading_mode);
        self.needs_render = true;
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 6 - Environment Mapping");
}
