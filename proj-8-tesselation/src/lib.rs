#![feature(array_zip)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::{Camera, CameraUpdate, ShadingModeSelector},
    math_helpers::round_up_pow_of_2,
    metal::*,
    metal_types::*,
    *,
};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, u32x2},
};

const DEPTH_COMPARISON_BIAS: f32 = 4e-3;
const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 16., 0.]);
const INITIAL_DISPLACEMENT_SCALE: f32 = 0.1;
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 3.5, PI / 3.]);
const INITIAL_TESSELATION_FACTOR: f32 = 32.;
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const MAX_DISPLACEMENT_SCALE: f32 = 1.;
const MAX_TESSELATION_FACTOR: f32 = 64.;
const MAX_TEXTURE_SIZE: u16 = 16384;
const SHADOW_MAP_DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;

impl Default for ProjectedSpace {
    #[inline]
    fn default() -> Self {
        Self {
            matrix_world_to_projection: f32x4x4::identity(),
            matrix_screen_to_world: f32x4x4::identity(),
            position_world: float4 { xyzw: [0.; 4] },
        }
    }
}

impl From<&CameraUpdate> for ProjectedSpace {
    fn from(update: &CameraUpdate) -> Self {
        ProjectedSpace {
            matrix_world_to_projection: update.matrix_world_to_projection,
            matrix_screen_to_world: update.matrix_screen_to_world,
            position_world: update.position_world.into(),
        }
    }
}

struct Delegate {
    camera_space: ProjectedSpace,
    camera: Camera,
    command_queue: CommandQueue,
    depth_state: DepthStencilState,
    depth_texture: Option<Texture>,
    device: Device,
    displacement_scale: f32,
    displacement_texture: Option<Texture>,
    library: Library,
    light_matrix_model_to_world: f32x4x4,
    light_matrix_world_to_projection: f32x4x4,
    light_model:
        Model<{ LightVertexBufferIndex::Geometry as _ }, { LightFragBufferIndex::Material as _ }>,
    light_pipeline: RenderPipelineState,
    light_space: ProjectedSpace,
    light: Camera,
    needs_render: bool,
    normal_texture: Texture,
    render_pipeline: RenderPipelineState,
    shading_mode: ShadingModeSelector,
    shadow_map_pipeline: RenderPipelineState,
    shadow_map_texture: Option<Texture>,
    show_triangulation: bool,
    tessellation_compute_pipeline: ComputePipelineState,
    tessellation_factor: f32,
    tessellation_factors_buffer: Buffer,
}

fn create_pipeline(
    device: &Device,
    library: &Library,
    mode: ShadingModeSelector,
) -> RenderPipelineState {
    let mut desc = new_render_pipeline_descriptor(
        "Plane",
        &library,
        Some((DEFAULT_PIXEL_FORMAT, false)),
        Some(DEPTH_TEXTURE_FORMAT),
        Some(&mode.encode(
            FunctionConstantValues::new(),
            ShadingMode::HasAmbient as _,
            ShadingMode::HasDiffuse as _,
            ShadingMode::HasSpecular as _,
            ShadingMode::OnlyNormals as _,
        )),
        Some((&"main_vertex", VertexBufferIndex::LENGTH as _)),
        Some((&"main_fragment", FragBufferIndex::LENGTH as _)),
    );
    set_tessellation_config(&mut desc);
    let p = create_render_pipeline(&device, &desc);
    use debug_assert_pipeline_function_arguments::*;
    debug_assert_render_pipeline_function_arguments(
        &p,
        &[
            value_arg::<f32x4x4>(VertexBufferIndex::MatrixWorldToProjection as _),
            value_arg::<f32>(VertexBufferIndex::DisplacementScale as _),
            texture_arg(VertexTextureIndex::Displacement as _, MTLTextureType::D2),
        ],
        Some(&[
            value_arg::<ProjectedSpace>(FragBufferIndex::CameraSpace as _),
            value_arg::<ProjectedSpace>(FragBufferIndex::LightSpace as _),
            value_arg::<bool>(FragBufferIndex::ShadeTriangulation as _),
            texture_arg(FragTextureIndex::Normal as _, MTLTextureType::D2),
            texture_arg(FragTextureIndex::ShadowMap as _, MTLTextureType::D2),
        ]),
    );
    p.pipeline_state
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let (normal_image_path, displacement_image_path): (PathBuf, Option<PathBuf>) =
            match (std::env::args().nth(1), std::env::args().nth(2)) {
                // No images provided (executable only)
                (None, None) => {
                    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
                    (
                        assets_dir.join("teapot_normal.png"),
                        Some(assets_dir.join("teapot_disp.png")),
                    )
                }
                // Only Normal image provided
                (Some(normal_path), None) => (PathBuf::from(normal_path), None),
                // Normal and Displacement image provided
                (Some(normal_path), Some(displacement_path)) => (
                    PathBuf::from(normal_path),
                    Some(PathBuf::from(displacement_path)),
                ),
                _ => panic!("Illegal arguments provided"),
            };

        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        let shading_mode = ShadingModeSelector::DEFAULT;
        let render_pipeline_state = create_pipeline(&device, &library, shading_mode);
        let mut image_buffer = vec![];
        Self {
            camera_space: Default::default(),
            camera: Camera::new(
                2.5,
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            command_queue: device.new_command_queue(),
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                desc.set_label("Depth State");
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: None,
            displacement_scale: INITIAL_DISPLACEMENT_SCALE,
            displacement_texture: displacement_image_path
                .map(|p| new_texture_from_png(p, &device, &mut image_buffer)),
            light_matrix_model_to_world: f32x4x4::identity(),
            light_matrix_world_to_projection: f32x4x4::identity(),
            light_model: Model::from_file(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .join("common-assets")
                    .join("light")
                    .join("light.obj"),
                &device,
                |arg: &mut Geometry, geo| {
                    arg.indices = geo.indices_buffer;
                    arg.positions = geo.positions_buffer;
                    arg.normals = geo.normals_buffer;
                    arg.tx_coords = geo.tx_coords_buffer;
                },
                |arg: &mut Material, mat| {
                    arg.ambient_texture = mat.ambient_texture;
                    arg.diffuse_texture = mat.diffuse_texture;
                    arg.specular_texture = mat.specular_texture;
                    arg.specular_shineness = mat.specular_shineness;
                    arg.ambient_amount = 0.8;
                },
            ),
            light_pipeline: {
                let p = create_render_pipeline(
                    &device,
                    &new_render_pipeline_descriptor(
                        "Light",
                        &library,
                        Some((DEFAULT_PIXEL_FORMAT, false)),
                        Some(DEPTH_TEXTURE_FORMAT),
                        None,
                        Some((&"light_vertex", LightVertexBufferIndex::LENGTH as _)),
                        Some((&"light_fragment", LightFragBufferIndex::LENGTH as _)),
                    ),
                );
                use debug_assert_pipeline_function_arguments::*;
                debug_assert_render_pipeline_function_arguments(
                    &p,
                    &[
                        value_arg::<f32x4x4>(LightVertexBufferIndex::MatrixModelToProjection as _),
                        value_arg::<Geometry>(LightVertexBufferIndex::Geometry as _),
                    ],
                    Some(&[value_arg::<Material>(LightFragBufferIndex::Material as _)]),
                );
                p.pipeline_state
            },
            light_space: Default::default(),
            light: Camera::new(
                1.25,
                INITIAL_LIGHT_ROTATION,
                ModifierKeys::CONTROL,
                true,
                1.,
            ),
            needs_render: false,
            normal_texture: new_texture_from_png(normal_image_path, &device, &mut image_buffer),
            render_pipeline: render_pipeline_state,
            shading_mode,
            shadow_map_pipeline: {
                let mut desc = new_render_pipeline_descriptor(
                    "Shadow Map Pipeline",
                    &library,
                    None,
                    Some(DEPTH_TEXTURE_FORMAT),
                    None,
                    Some((&"main_vertex", VertexBufferIndex::LENGTH as _)),
                    None,
                );
                set_tessellation_config(&mut desc);
                let p = create_render_pipeline(&device, &desc);
                use debug_assert_pipeline_function_arguments::*;
                debug_assert_render_pipeline_function_arguments(
                    &p,
                    &[
                        value_arg::<f32x4x4>(VertexBufferIndex::MatrixWorldToProjection as _),
                        value_arg::<f32>(VertexBufferIndex::DisplacementScale as _),
                        texture_arg(VertexTextureIndex::Displacement as _, MTLTextureType::D2),
                    ],
                    None,
                );
                p.pipeline_state
            },
            shadow_map_texture: None,
            show_triangulation: false,
            tessellation_compute_pipeline: {
                let fun = library
                    .get_function(&"tessell_compute", None)
                    .expect("Failed to get tessellation compute function");
                device
                    .new_compute_pipeline_state_with_function(&fun)
                    .expect("Failed to create tessellation compute pipeline")
            },
            tessellation_factor: INITIAL_TESSELATION_FACTOR,
            tessellation_factors_buffer: {
                // TODO: What is the exact size?
                // - 256 was copied from Apple Metal Sample Code: https://developer.apple.com/library/archive/samplecode/MetalBasicTessellation/Introduction/Intro.html
                let buf = device.new_buffer(256, MTLResourceOptions::StorageModePrivate);
                buf.set_label("Tessellation Factors");
                buf
            },
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
        command_buffer.set_label("Command Buffer");

        // Tesselation Factors
        {
            let encoder = command_buffer.new_compute_command_encoder();
            encoder.set_label("Tesselation Factors");
            encoder.set_compute_pipeline_state(&self.tessellation_compute_pipeline);
            encoder.set_bytes(
                TesselComputeBufferIndex::TessellFactor as _,
                std::mem::size_of_val(&self.tessellation_factor) as _,
                (&self.tessellation_factor as *const f32) as _,
            );
            encoder.set_buffer(
                TesselComputeBufferIndex::OutputTessellFactors as _,
                Some(&self.tessellation_factors_buffer),
                0,
            );
            let size_one = MTLSize {
                width: 1,
                height: 1,
                depth: 1,
            };
            encoder.dispatch_thread_groups(size_one, size_one);
            encoder.end_encoding();
        }
        // Render Shadow Map
        {
            let encoder = command_buffer.new_render_command_encoder(new_render_pass_descriptor(
                None,
                self.shadow_map_texture
                    .as_ref()
                    .map(|d| (d, MTLStoreAction::Store)),
            ));
            encoder.set_label("Shadow Map");
            encoder.set_render_pipeline_state(&self.shadow_map_pipeline);
            encoder.set_depth_stencil_state(&self.depth_state);
            set_tesselation_factor_buffer(encoder, &self.tessellation_factors_buffer);
            encode_vertex_bytes(
                encoder,
                VertexBufferIndex::MatrixWorldToProjection as _,
                &self.light_matrix_world_to_projection,
            );
            encode_vertex_bytes(
                encoder,
                VertexBufferIndex::DisplacementScale as _,
                &self.displacement_scale,
            );
            encoder.set_vertex_texture(
                VertexTextureIndex::Displacement as _,
                self.displacement_texture.as_deref(),
            );
            draw_patches(encoder, 4);
            encoder.end_encoding()
        }
        // Render Plane
        {
            let encoder = command_buffer.new_render_command_encoder(new_render_pass_descriptor(
                Some(render_target),
                self.depth_texture
                    .as_ref()
                    .map(|d| (d, MTLStoreAction::DontCare)),
            ));
            encoder.set_label("Plane and Light");
            {
                encoder.push_debug_group("Light");
                encoder.set_render_pipeline_state(&self.light_pipeline);
                encoder.set_depth_stencil_state(&self.depth_state);
                encode_vertex_bytes(
                    encoder,
                    LightVertexBufferIndex::MatrixModelToProjection as _,
                    &(self.camera_space.matrix_world_to_projection
                        * self.light_matrix_model_to_world),
                );
                self.light_model.encode_draws(encoder);
                encoder.pop_debug_group();
            }
            {
                encoder.push_debug_group("Plane");
                encoder.set_render_pipeline_state(&self.render_pipeline);
                encoder.set_depth_stencil_state(&self.depth_state);
                set_tesselation_factor_buffer(encoder, &self.tessellation_factors_buffer);
                encode_vertex_bytes(
                    encoder,
                    VertexBufferIndex::MatrixWorldToProjection as _,
                    &self.camera_space.matrix_world_to_projection,
                );
                encode_vertex_bytes(
                    encoder,
                    VertexBufferIndex::DisplacementScale as _,
                    &self.displacement_scale,
                );
                encoder.set_vertex_texture(
                    VertexTextureIndex::Displacement as _,
                    self.displacement_texture.as_deref(),
                );
                encode_fragment_bytes(
                    encoder,
                    FragBufferIndex::CameraSpace as _,
                    &self.camera_space,
                );
                encode_fragment_bytes(encoder, FragBufferIndex::LightSpace as _, &self.light_space);
                encode_fragment_bytes(encoder, FragBufferIndex::ShadeTriangulation as _, &false);
                encoder.set_fragment_texture(
                    FragTextureIndex::Normal as _,
                    Some(&self.normal_texture),
                );
                encoder.set_fragment_texture(
                    FragTextureIndex::ShadowMap as _,
                    self.shadow_map_texture.as_deref(),
                );
                draw_patches(encoder, 4);
                // IMPORTANT: This does *NOT* meet the project requirements, but accomplishes the
                //            same thing!
                // - The requirements ask for a Geometry Shader to render the triangulation.
                // - Unfortunately (fortunately?), the Metal API does not support/have Geometry
                //   Shaders.
                if self.show_triangulation {
                    encoder.set_triangle_fill_mode(MTLTriangleFillMode::Lines);
                    encode_fragment_bytes(encoder, FragBufferIndex::ShadeTriangulation as _, &true);
                    draw_patches(encoder, 4);
                }
                encoder.pop_debug_group();
            }
            encoder.end_encoding();
        }
        command_buffer
    }

    #[inline]
    fn on_event(&mut self, event: UserEvent) {
        if let Some(update) = self.camera.on_event(event) {
            self.camera_space = ProjectedSpace::from(&update);
            self.needs_render = true;
        }
        if let Some(update) = self.light.on_event(event) {
            self.light_matrix_model_to_world = update.matrix_camera_to_world
                * f32x4x4::y_rotate(PI)
                * f32x4x4::scale(0.1, 0.1, 0.1, 1.0);
            self.light_matrix_world_to_projection = update.matrix_world_to_projection;
            self.light_space = ProjectedSpace::from(&update);
            //
            // IMPORTANT: Projecting to a Texture, NOT to the screen.
            // Used to **sample** Shadow Map Depth Texture during shading to produce shadows.
            // Rendering to the Shadow Map Depth Texture uses the
            // `light_matrix_world_to_projection`.
            //
            // This projected coordinate space differs from the screen coordinate space (Metal
            // Normalized Device Coordinates), in the following ways:
            // - XY dimension range:      [-1,1] -> [0,1]
            // - Y dimension is inverted: +Y     -> -Y
            // - Z includes a bias for better depth comparison
            //
            self.light_space.matrix_world_to_projection = {
                // Performance: Bake all the transform at compile time.
                // - Currently Rust does not allow floating-point operations in constant
                //   expressions.
                // - Reduces the amount of code generated/executed, >150 bytes of instructions
                //   saved.
                const PROJECTION_TO_TEXTURE_COORDINATE_SPACE: f32x4x4 = f32x4x4::new(
                    [0.5, 0.0, 0.0, 0.5],
                    [0.0, -0.5, 0.0, 0.5],
                    [0.0, 0.0, 1.0, -DEPTH_COMPARISON_BIAS],
                    [0.0, 0.0, 0.0, 1.0],
                );
                #[cfg(debug_assertions)]
                {
                    // Invert Y
                    let projection_to_texture_coordinate_space_derived =
                        f32x4x4::scale_translate(1., -1., 1., 0., 1., 0.)
                            * f32x4x4::scale_translate(
                                // Convert from [-1, 1] -> [0, 1] for XY dimensions
                                0.5,
                                0.5,
                                1.0,
                                0.5,
                                0.5,
                                // Add Depth Comparison Bias
                                -DEPTH_COMPARISON_BIAS,
                            );
                    assert_eq!(
                        projection_to_texture_coordinate_space_derived.columns,
                        PROJECTION_TO_TEXTURE_COORDINATE_SPACE.columns
                    );
                }
                PROJECTION_TO_TEXTURE_COORDINATE_SPACE
            } * update.matrix_world_to_projection;
            self.needs_render = true;
        }
        if self.shading_mode.on_event(event) {
            self.render_pipeline = create_pipeline(&self.device, &self.library, self.shading_mode);
            self.needs_render = true;
        }
        match event {
            UserEvent::KeyDown { key_code, .. } => {
                match key_code {
                    49  /* ProjectedSpace */ => self.show_triangulation = !self.show_triangulation,
                    126 /* Up    */ => self.displacement_scale = (self.displacement_scale + 0.01).min(MAX_DISPLACEMENT_SCALE),
                    125 /* Down  */ => self.displacement_scale = (self.displacement_scale - 0.01).max(0.),
                    124 /* Right */ => self.tessellation_factor = (self.tessellation_factor + 1.).min(MAX_TESSELATION_FACTOR),
                    123 /* Left  */ => self.tessellation_factor = (self.tessellation_factor - 1.).max(1.),
                    _ => {return;}
                }
                self.needs_render = true;
            }
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
        desc.set_storage_mode(MTLStorageMode::Memoryless);
        desc.set_usage(MTLTextureUsage::RenderTarget);
        let texture = self.device.new_texture(&desc);
        texture.set_label("Depth");
        self.depth_texture = Some(texture);

        // Make sure the shadow map texture is atleast 2x and no more than 4x, of the
        // screen size. Round up to the nearest power of 2 of each dimension.
        let xy = u32x2::from_array([size[0] as u32, size[1] as u32]);
        if let Some(tx) = &self.shadow_map_texture {
            #[inline(always)]
            fn is_shadow_map_correctly_sized(cur: NSUInteger, target: u32) -> bool {
                ((target << 1)..=(target << 2)).contains(&(cur as _))
            }
            if is_shadow_map_correctly_sized(tx.width(), xy[0])
                && is_shadow_map_correctly_sized(tx.height(), xy[1])
            {
                return;
            }
        }
        let new_xy =
            round_up_pow_of_2(xy << u32x2::splat(1)).min(u32x2::splat(MAX_TEXTURE_SIZE as _));

        #[cfg(debug_assertions)]
        println!("Allocating new Shadow Map {new_xy:?}");

        desc.set_width(new_xy[0] as _);
        desc.set_height(new_xy[1] as _);
        desc.set_pixel_format(SHADOW_MAP_DEPTH_TEXTURE_FORMAT);
        desc.set_storage_mode(MTLStorageMode::Private);
        desc.set_usage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
        let texture = self.device.new_texture(&desc);
        texture.set_label("Shadow Map Depth");
        self.shadow_map_texture = Some(texture);
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 8 - Tesselation");
}
