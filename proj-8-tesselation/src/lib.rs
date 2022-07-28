#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::{Camera, CameraUpdate, DepthTexture, ShadingModeSelector},
    math_helpers::round_up_pow_of_2,
    metal::*,
    metal_types::*,
    pipeline::*,
    typed_buffer::TypedBuffer,
    *,
};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    ops::Deref,
    path::PathBuf,
    simd::{f32x2, u32x2},
};

const DEPTH_COMPARISON_BIAS: f32 = 4e-3;
const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 16., 0.]);
const INITIAL_DISPLACEMENT_SCALE: f32 = 0.1;
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 3.5, PI / 3.]);
const INITIAL_TESSELATION_FACTOR: u16 = 32;
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const MAX_DISPLACEMENT_SCALE: f32 = 1.;
const MAX_TESSELATION_FACTOR: u16 = 64;
const MAX_TEXTURE_SIZE: u16 = 16384;
const SHADOW_MAP_DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;

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
    depth_texture: DepthTexture,
    device: Device,
    displacement_scale: f32,
    displacement_texture: Option<Texture>,
    library: Library,
    light_matrix_model_to_world: f32x4x4,
    light_matrix_world_to_projection: f32x4x4,
    light_model: Model<Geometry, HasMaterial<Material>>,
    light_pipeline: RenderPipeline<1, light_vertex, light_fragment, (Depth, NoStencil)>,
    light_space: ProjectedSpace,
    light: Camera,
    needs_render: bool,
    normal_texture: Texture,
    render_pipeline: TesselationRenderPipeline<1, main_vertex, main_fragment, (Depth, NoStencil)>,
    shading_mode: ShadingModeSelector,
    shadow_map_pipeline:
        TesselationRenderPipeline<0, main_vertex, NoFragmentFunction, (Depth, NoStencil)>,
    shadow_map_texture: Option<Texture>,
    show_triangulation: bool,
    tessellation_factor: u16,
    tessellation_factors_buffer: TypedBuffer<MTLQuadTessellationFactorsHalf>,
}

fn create_pipeline(
    device: &Device,
    library: &Library,
    mode: ShadingModeSelector,
) -> TesselationRenderPipeline<1, main_vertex, main_fragment, (Depth, NoStencil)> {
    return TesselationRenderPipeline::new(
        "Plane",
        &device,
        &library,
        [(DEFAULT_PIXEL_FORMAT, BlendMode::NoBlend)],
        main_vertex,
        main_fragment {
            HasAmbient: mode.contains(ShadingModeSelector::HAS_AMBIENT),
            HasDiffuse: mode.contains(ShadingModeSelector::HAS_DIFFUSE),
            OnlyNormals: mode.contains(ShadingModeSelector::ONLY_NORMALS),
            HasSpecular: mode.contains(ShadingModeSelector::HAS_SPECULAR),
        },
        (Depth(DEPTH_TEXTURE_FORMAT), NoStencil),
    );
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
            depth_texture: DepthTexture::new("Depth", DEPTH_TEXTURE_FORMAT),
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
                HasMaterial(|arg: &mut Material, mat| {
                    arg.ambient_texture = mat.ambient_texture;
                    arg.diffuse_texture = mat.diffuse_texture;
                    arg.specular_texture = mat.specular_texture;
                    arg.specular_shineness = mat.specular_shineness;
                    arg.ambient_amount = 0.8;
                }),
            ),
            light_pipeline: RenderPipeline::new(
                "Light",
                &device,
                &library,
                [(DEFAULT_PIXEL_FORMAT, BlendMode::NoBlend)],
                light_vertex,
                light_fragment,
                (Depth(DEPTH_TEXTURE_FORMAT), NoStencil),
            ),
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
                TesselationRenderPipeline::new(
                    "Shadow Map",
                    &device,
                    &library,
                    [],
                    main_vertex,
                    NoFragmentFunction,
                    (Depth(DEPTH_TEXTURE_FORMAT), NoStencil),
                )
            },
            shadow_map_texture: None,
            show_triangulation: false,
            tessellation_factor: INITIAL_TESSELATION_FACTOR,
            tessellation_factors_buffer: TypedBuffer::from_data(
                "Tessellation Factors",
                device.deref(),
                &[MTLQuadTessellationFactorsHalf::new(
                    INITIAL_TESSELATION_FACTOR,
                )],
                MTLResourceOptions::StorageModeShared,
            ),
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

        // Render Shadow Map
        let usage_tesselation_factors_buffer: &dyn ResourceUsage = &BufferUsage(
            &self.tessellation_factors_buffer,
            MTLResourceUsage::Read,
            MTLRenderStages::Vertex | MTLRenderStages::Fragment,
        );
        let usage_displacement_texture;
        self.shadow_map_pipeline.new_pass(
            "Shadow Map",
            command_buffer,
            [],
            (
                self.shadow_map_texture.as_deref().unwrap(),
                1.,
                MTLLoadAction::Clear,
                MTLStoreAction::Store,
            ),
            NoStencil,
            &self.tessellation_factors_buffer,
            &self.depth_state,
            // TODO: See RenderPipeline::new todo on making this more like Binds w/skip, should make
            //       this bit of ugliness go away.
            &(if let Some(displacement_texture) = &self.displacement_texture {
                usage_displacement_texture = TextureUsage(
                    displacement_texture.deref(),
                    MTLResourceUsage::Sample,
                    MTLRenderStages::Vertex,
                );
                [
                    &usage_displacement_texture,
                    usage_tesselation_factors_buffer,
                ]
            } else {
                [usage_tesselation_factors_buffer; 2]
            }),
            |p| {
                p.draw_patches_with_bind(
                    main_vertex_binds {
                        matrix_world_to_projection: Bind::Value(
                            &self.light_matrix_world_to_projection,
                        ),
                        displacement_scale: Bind::Value(&self.displacement_scale),
                        disp_tx: if let Some(displacement_texture) = &self.displacement_texture {
                            BindTexture::Texture(displacement_texture)
                        } else {
                            BindTexture::Skip
                        },
                    },
                    NoBinds,
                    4,
                )
            },
        );
        // Render Plane and Light
        self.light_pipeline.new_pass(
            "Light",
            command_buffer,
            [(
                render_target,
                (0., 0., 0., 0.),
                MTLLoadAction::Clear,
                MTLStoreAction::Store,
            )],
            (
                self.depth_texture.texture(),
                1.,
                MTLLoadAction::Clear,
                MTLStoreAction::DontCare,
            ),
            NoStencil,
            &self.depth_state,
            &[
                &HeapUsage(
                    &self.light_model.heap,
                    MTLRenderStages::Vertex | MTLRenderStages::Fragment,
                ),
                // &TextureUsage(
                //     &self.displacement_texture.as_deref().unwrap(),
                //     MTLResourceUsage::Sample,
                //     MTLRenderStages::Vertex,
                // ),
                &TextureUsage(
                    &self.normal_texture,
                    MTLResourceUsage::Sample,
                    MTLRenderStages::Fragment,
                ),
                &TextureUsage(
                    &self.normal_texture,
                    MTLResourceUsage::Sample,
                    MTLRenderStages::Fragment,
                ),
            ],
            |p| {
                p.bind(
                    light_vertex_binds {
                        matrix_model_to_projection: Bind::Value(
                            &(self.camera_space.matrix_world_to_projection
                                * self.light_matrix_model_to_world),
                        ),
                        ..Binds::SKIP
                    },
                    Binds::SKIP,
                );
                for draw in self.light_model.draws() {
                    p.draw_primitives_with_bind(
                        light_vertex_binds {
                            geometry: Bind::buffer_with_rolling_offset(draw.geometry),
                            ..Binds::SKIP
                        },
                        light_fragment_binds {
                            material: Bind::iterating_buffer_offset(draw.geometry.1, draw.material),
                        },
                        MTLPrimitiveType::Triangle,
                        0,
                        draw.vertex_count,
                    );
                }
                p.into_tesselation_subpass(
                    "Model",
                    &self.render_pipeline,
                    None,
                    Some(&self.tessellation_factors_buffer),
                    |p| {
                        p.draw_patches_with_bind(
                            main_vertex_binds {
                                matrix_world_to_projection: Bind::Value(
                                    &self.camera_space.matrix_world_to_projection,
                                ),
                                displacement_scale: Bind::Value(&self.displacement_scale),
                                disp_tx: if let Some(displacement_texture) =
                                    &self.displacement_texture
                                {
                                    BindTexture::Texture(displacement_texture)
                                } else {
                                    BindTexture::Skip
                                },
                            },
                            main_fragment_binds {
                                camera: Bind::Value(&self.camera_space),
                                light: Bind::Value(&self.light_space),
                                shade_tri: Bind::Value(&false),
                                normal_tx: BindTexture::Texture(&self.normal_texture),
                                shadow_tx: BindTexture::Texture(
                                    self.shadow_map_texture.as_deref().unwrap(),
                                ),
                            },
                            4,
                        );
                        // IMPORTANT: This does *NOT* meet the project requirements, but accomplishes the
                        //            same thing!
                        // - The requirements ask for a Geometry Shader to render the triangulation.
                        // - Unfortunately (fortunately?), the Metal API does not support/have Geometry
                        //   Shaders.
                        if self.show_triangulation {
                            p.set_triangle_fill_mode(MTLTriangleFillMode::Lines);
                            p.draw_patches_with_bind(
                                Binds::SKIP,
                                main_fragment_binds {
                                    shade_tri: Bind::Value(&true),
                                    ..Binds::SKIP
                                },
                                4,
                            );
                        }
                    },
                );
            },
        );
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
        if self.depth_texture.on_event(event, &self.device) {
            let t = self.depth_texture.texture();
            let xy = u32x2::from_array([t.width() as _, t.height() as _]);
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

            let desc = TextureDescriptor::new();
            desc.set_width(new_xy[0] as _);
            desc.set_height(new_xy[1] as _);
            desc.set_pixel_format(SHADOW_MAP_DEPTH_TEXTURE_FORMAT);
            desc.set_storage_mode(MTLStorageMode::Private);
            desc.set_usage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
            let shadow_map_texture = self.device.new_texture(&desc);
            shadow_map_texture.set_label("Shadow Map Depth");
            self.shadow_map_texture = Some(shadow_map_texture);
            self.needs_render = true;
        }
        match event {
            UserEvent::KeyDown { key_code, .. } => {
                match key_code {
                    49  /* Space */ => self.show_triangulation = !self.show_triangulation,
                    126 /* Up    */ => self.displacement_scale = (self.displacement_scale + 0.01).min(MAX_DISPLACEMENT_SCALE),
                    125 /* Down  */ => self.displacement_scale = (self.displacement_scale - 0.01).max(0.),
                    124 | 123 /* Right | Left */=> {
                        self.tessellation_factor = match key_code {
                            124 /* Right */ => (self.tessellation_factor + 1).min(MAX_TESSELATION_FACTOR),
                            123 /* Left  */ => (self.tessellation_factor - 1).max(1),
                            _ => panic!()
                        };
                        self.tessellation_factors_buffer.get_mut()[0] = MTLQuadTessellationFactorsHalf::new(self.tessellation_factor);
                    }
                    _ => {return;}
                }
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

pub fn run() {
    launch_application::<Delegate>("Project 8 - Tesselation");
}
