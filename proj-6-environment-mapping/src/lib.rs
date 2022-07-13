#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::{Camera, ShadingModeSelector},
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

const BG_STENCIL_REF_VALUE: u32 = 0;
const MIRROR_PLANE_STENCIL_REF_VALUE: u32 = 1;
const MODEL_STENCIL_REF_VALUE: u32 = 2;

const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const STENCIL_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Stencil8;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 32., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_POSITION: f32x4 = f32x4::from_array([0., 1., -1., 1.]);

struct Delegate {
    bg_depth_state: DepthStencilState,
    bg_render_pipeline: RenderPipelineState,
    camera: Camera,
    camera_space: ProjectedSpace,
    command_queue: CommandQueue,
    cubemap_texture: Texture,
    depth_texture: Option<Texture>,
    device: Device,
    library: Library,
    main_render_pipeline: RenderPipelineState,
    matrix_model_to_world: f32x4x4,
    matrix_mirror_plane_model_to_world: f32x4x4,
    matrix_world_to_mirror_world: f32x4x4,
    mirrored_model_depth_state: DepthStencilState,
    mirror_plane_depth_state: DepthStencilState,
    mirror_plane_model: Model<{ VertexBufferIndex::Geometry as _ }, { NO_MATERIALS_ID }>,
    model_depth_state: DepthStencilState,
    model: Model<{ VertexBufferIndex::Geometry as _ }, { NO_MATERIALS_ID }>,
    needs_render: bool,
    shading_mode: ShadingModeSelector,
    stencil_texture: Option<Texture>,
}

fn create_main_render_pipeline(
    device: &Device,
    library: &Library,
    mode: ShadingModeSelector,
) -> RenderPipelineState {
    let function_constants = mode.encode(
        FunctionConstantValues::new(),
        ShadingMode::HasAmbient as _,
        ShadingMode::HasDiffuse as _,
        ShadingMode::HasSpecular as _,
        ShadingMode::OnlyNormals as _,
    );
    // Model Pipeline
    let p = create_render_pipeline(
        &device,
        &new_render_pipeline_descriptor_with_stencil(
            "Model",
            &library,
            Some((DEFAULT_PIXEL_FORMAT, false)),
            Some(DEPTH_TEXTURE_FORMAT),
            Some(STENCIL_TEXTURE_FORMAT),
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
            value_arg::<f32>(FragBufferIndex::Darken as _),
            texture_arg(FragTextureIndex::EnvTexture as _, MTLTextureType::Cube),
        ]),
    );
    p.pipeline_state
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let executable_name = std::env::args()
            .nth(0)
            .expect("Failed to access command line executable name");
        let model_file_path = std::env::args().nth(1).expect(&format!(
            "Usage: {executable_name} [Path to Wavefront OBJ file]"
        ));
        let encode_geometry_arg = |arg: &mut Geometry,
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
        };
        let model = Model::from_file(
            PathBuf::from(model_file_path),
            &device,
            encode_geometry_arg,
            NO_MATERIALS_ENCODER,
        );
        let mirror_plane_model = Model::from_file(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("common-assets")
                .join("plane")
                .join("plane.obj"),
            &device,
            encode_geometry_arg,
            NO_MATERIALS_ENCODER,
        );
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
        let matrix_mirror_plane_model_to_world =
            f32x4x4::translate(0., mirror_plane_y_world, 0.) * f32x4x4::scale(0.9, 0.9, 0.9, 1.);

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

        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        let shading_mode = ShadingModeSelector::DEFAULT;
        Self {
            bg_depth_state: {
                let ds = DepthStencilDescriptor::new();
                ds.set_depth_write_enabled(false);
                ds.set_depth_compare_function(MTLCompareFunction::LessEqual);
                {
                    let s = StencilDescriptor::new();
                    s.set_stencil_compare_function(MTLCompareFunction::Equal);
                    s.set_depth_stencil_pass_operation(MTLStencilOperation::Keep);
                    ds.set_front_face_stencil(Some(&s));
                    ds.set_back_face_stencil(Some(&s));
                }
                device.new_depth_stencil_state(&ds)
            },
            bg_render_pipeline: {
                let p = create_render_pipeline(
                    &device,
                    &new_render_pipeline_descriptor_with_stencil(
                        "BG",
                        &library,
                        Some((DEFAULT_PIXEL_FORMAT, false)),
                        Some(DEPTH_TEXTURE_FORMAT),
                        Some(STENCIL_TEXTURE_FORMAT),
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
                p.pipeline_state
            },
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
            cubemap_texture: debug_time("proj6 - Load Environment Cube Texture", || {
                asset_compiler::cube_texture::load_cube_texture_asset_dir(
                    &device,
                    &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("assets")
                        .join("cubemap.asset"),
                )
            }),
            depth_texture: None,
            main_render_pipeline: create_main_render_pipeline(&device, &library, shading_mode),
            matrix_model_to_world,
            matrix_world_to_mirror_world,
            matrix_mirror_plane_model_to_world,
            mirrored_model_depth_state: {
                let ds = DepthStencilDescriptor::new();
                ds.set_depth_write_enabled(true);
                ds.set_depth_compare_function(MTLCompareFunction::LessEqual);
                {
                    let s = StencilDescriptor::new();
                    s.set_stencil_compare_function(MTLCompareFunction::Equal);
                    s.set_depth_stencil_pass_operation(MTLStencilOperation::Keep);
                    ds.set_front_face_stencil(Some(&s));
                    ds.set_back_face_stencil(Some(&s));
                }
                device.new_depth_stencil_state(&ds)
            },
            mirror_plane_model,
            mirror_plane_depth_state: {
                let ds = DepthStencilDescriptor::new();
                ds.set_depth_write_enabled(false);
                ds.set_depth_compare_function(MTLCompareFunction::LessEqual);
                {
                    let s = StencilDescriptor::new();
                    s.set_stencil_compare_function(MTLCompareFunction::Always);
                    s.set_depth_stencil_pass_operation(MTLStencilOperation::Replace);
                    ds.set_front_face_stencil(Some(&s));
                    ds.set_back_face_stencil(Some(&s));
                }
                device.new_depth_stencil_state(&ds)
            },
            model_depth_state: {
                let ds = DepthStencilDescriptor::new();
                ds.set_depth_write_enabled(true);
                ds.set_depth_compare_function(MTLCompareFunction::LessEqual);
                {
                    let s = StencilDescriptor::new();
                    s.set_stencil_compare_function(MTLCompareFunction::Always);
                    s.set_depth_stencil_pass_operation(MTLStencilOperation::Replace);
                    ds.set_front_face_stencil(Some(&s));
                    ds.set_back_face_stencil(Some(&s));
                }
                device.new_depth_stencil_state(&ds)
            },
            model,
            needs_render: false,
            shading_mode,
            stencil_texture: None,
            device,
            library,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.needs_render = false;

        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        command_buffer.set_label("Renderer Command Buffer");
        let encoder = command_buffer.new_render_command_encoder(new_render_pass_descriptor(
            Some((
                render_target,
                (0., 0., 0., 0.),
                MTLLoadAction::Clear,
                MTLStoreAction::Store,
            )),
            self.depth_texture
                .as_ref()
                .map(|d| (d, 1., MTLLoadAction::Clear, MTLStoreAction::DontCare)),
            self.stencil_texture
                .as_ref()
                .map(|d| (d, 0, MTLLoadAction::Clear, MTLStoreAction::DontCare)),
        ));
        self.model.encode_use_resources(encoder);
        self.mirror_plane_model.encode_use_resources(encoder);
        {
            encoder.push_debug_group("Model");
            encoder.set_render_pipeline_state(&self.main_render_pipeline);
            encoder.set_depth_stencil_state(&self.model_depth_state);
            encoder.set_stencil_reference_value(MODEL_STENCIL_REF_VALUE);
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
                &LIGHT_POSITION,
            );
            encode_fragment_bytes(
                encoder,
                FragBufferIndex::MatrixEnvironment as _,
                &Into::<float3x3>::into(f32x4x4::identity()),
            );
            encode_fragment_bytes(encoder, FragBufferIndex::Darken as _, &0_f32);
            encoder.set_fragment_texture(
                FragTextureIndex::EnvTexture as _,
                Some(&self.cubemap_texture),
            );
            self.model.encode_draws(encoder);
            encoder.pop_debug_group();
        }
        {
            encoder.push_debug_group("Plane");
            encoder.set_depth_stencil_state(&self.mirror_plane_depth_state);
            encoder.set_stencil_reference_value(MIRROR_PLANE_STENCIL_REF_VALUE);
            encode_vertex_bytes(
                encoder,
                VertexBufferIndex::Model as _,
                &ModelSpace {
                    matrix_model_to_projection: self.camera_space.matrix_world_to_projection
                        * self.matrix_mirror_plane_model_to_world,
                    matrix_normal_to_world: self.matrix_mirror_plane_model_to_world.into(),
                },
            );
            self.mirror_plane_model.encode_draws(encoder);
            encoder.pop_debug_group();
        }
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
            encoder.set_depth_stencil_state(&self.mirrored_model_depth_state);
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
            encode_fragment_bytes(encoder, FragBufferIndex::Camera as _, &mirror_camera_space);
            encode_fragment_bytes(encoder, FragBufferIndex::Darken as _, &0.5_f32);
            encode_fragment_bytes(
                encoder,
                FragBufferIndex::LightPosition as _,
                &(self.matrix_world_to_mirror_world * LIGHT_POSITION),
            );
            encode_fragment_bytes(
                encoder,
                FragBufferIndex::MatrixEnvironment as _,
                &Into::<float3x3>::into(self.matrix_world_to_mirror_world),
            );
            self.model.encode_draws(encoder);
            encoder.pop_debug_group();
        }
        {
            encoder.push_debug_group("BG");
            encoder.set_render_pipeline_state(&self.bg_render_pipeline);
            encoder.set_depth_stencil_state(&self.bg_depth_state);
            encoder.set_stencil_reference_value(BG_STENCIL_REF_VALUE);
            encode_fragment_bytes(encoder, FragBufferIndex::Camera as _, &self.camera_space);
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
        desc.set_storage_mode(MTLStorageMode::Memoryless);
        desc.set_usage(MTLTextureUsage::RenderTarget);

        self.depth_texture = Some({
            desc.set_pixel_format(DEPTH_TEXTURE_FORMAT);
            let texture = self.device.new_texture(&desc);
            texture.set_label("Depth");
            texture
        });

        self.stencil_texture = Some({
            desc.set_pixel_format(STENCIL_TEXTURE_FORMAT);
            let texture = self.device.new_texture(&desc);
            texture.set_label("Stencil");
            texture
        });
    }

    #[inline]
    fn update_mode(&mut self) {
        self.main_render_pipeline =
            create_main_render_pipeline(&self.device, &self.library, self.shading_mode);
        self.needs_render = true;
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 6 - Environment Mapping");
}
