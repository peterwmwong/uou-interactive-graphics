#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::{Camera, DepthTexture, ShadingModeSelector},
    math_helpers::round_up_pow_of_2,
    metal::*,
    metal_types::*,
    pipeline::*,
    *,
};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    ops::Neg,
    path::{Path, PathBuf},
    simd::{f32x2, u32x2, SimdFloat},
};

const DEFAULT_AMBIENT_AMOUNT: u32 = 15;
const DEPTH_COMPARISON_BIAS: f32 = 4e-4;
const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 5., PI / 16.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const MAX_TEXTURE_SIZE: u16 = 16384;
const SHADOW_MAP_DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const USAGE_RENDER_STAGES: MTLRenderStages = unsafe {
    MTLRenderStages::from_bits_unchecked(
        MTLRenderStages::Vertex.bits() | MTLRenderStages::Fragment.bits(),
    )
};

struct ModelInstance {
    matrix_model_to_world: f32x4x4,
    model: Model<Geometry, HasMaterial<Material>>,
    model_space: ModelSpace,
    name: &'static str,
}

impl ModelInstance {
    #[inline]
    fn new<const AMBIENT_AMOUNT: u32, T: AsRef<Path>>(
        name: &'static str,
        device: &Device,
        model_file: T,
        init_matrix_model_to_world: impl FnOnce(&MaxBounds) -> f32x4x4,
    ) -> Self {
        let model = Model::from_file(
            model_file,
            device,
            |arg: &mut Geometry, geo| {
                arg.indices = geo.indices_buffer;
                arg.positions = geo.positions_buffer;
                arg.normals = geo.normals_buffer;
                arg.tx_coords = geo.tx_coords_buffer;
            },
            HasMaterial(|arg: &mut Material, mat: MaterialToEncode| {
                arg.ambient_texture = mat.ambient_texture;
                arg.diffuse_texture = mat.diffuse_texture;
                arg.specular_texture = mat.specular_texture;
                arg.specular_shineness = mat.specular_shineness;
                arg.ambient_amount = (AMBIENT_AMOUNT as f32) / 100.;
            }),
        );
        Self {
            matrix_model_to_world: init_matrix_model_to_world(&model.geometry_max_bounds),
            model,
            model_space: ModelSpace::default(),
            name,
        }
    }

    fn on_camera_update(&mut self, camera_matrix_world_to_projection: f32x4x4) {
        self.model_space = ModelSpace {
            matrix_model_to_projection: (camera_matrix_world_to_projection
                * self.matrix_model_to_world),
            matrix_normal_to_world: self.matrix_model_to_world.into(),
        };
    }
}

struct Delegate {
    camera_space: ProjectedSpace,
    camera: Camera,
    command_queue: CommandQueue,
    depth_state: DepthStencilState,
    depth_texture: DepthTexture,
    device: Device,
    library: Library,
    light_matrix_world_to_projection: f32x4x4,
    light_space: ProjectedSpace,
    light: Camera,
    model_light: ModelInstance,
    model_pipeline: RenderPipeline<1, main_vertex, main_fragment, (Depth, NoStencil)>,
    model_plane: ModelInstance,
    model: ModelInstance,
    needs_render: bool,
    needs_render_shadow_map: bool,
    shading_mode: ShadingModeSelector,
    shadow_map_pipeline: RenderPipeline<0, main_vertex, NoFragmentFunction, (Depth, NoStencil)>,
    shadow_map_texture: Option<Texture>,
}

fn create_pipeline(
    device: &Device,
    library: &Library,
    mode: ShadingModeSelector,
) -> RenderPipeline<1, main_vertex, main_fragment, (Depth, NoStencil)> {
    RenderPipeline::new(
        "Model",
        device,
        library,
        [(DEFAULT_PIXEL_FORMAT, BlendMode::NoBlend)],
        main_vertex,
        main_fragment {
            HasAmbient: mode.contains(ShadingModeSelector::HAS_AMBIENT),
            HasDiffuse: mode.contains(ShadingModeSelector::HAS_DIFFUSE),
            OnlyNormals: mode.contains(ShadingModeSelector::ONLY_NORMALS),
            HasSpecular: mode.contains(ShadingModeSelector::HAS_SPECULAR),
        },
        (Depth(DEPTH_TEXTURE_FORMAT), NoStencil),
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
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");

        let mut plane_y = 0_f32;
        let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("common-assets");
        let shading_mode = ShadingModeSelector::DEFAULT;
        Self {
            camera: Camera::new_with_default_distance(
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            camera_space: Default::default(),
            command_queue: device.new_command_queue(),
            depth_texture: DepthTexture::new("Depth", DEPTH_TEXTURE_FORMAT),
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            light: Camera::new_with_default_distance(
                INITIAL_LIGHT_ROTATION,
                ModifierKeys::CONTROL,
                true,
                1.,
            ),
            light_matrix_world_to_projection: f32x4x4::identity(),
            light_space: Default::default(),
            model: ModelInstance::new::<DEFAULT_AMBIENT_AMOUNT, _>(
                "Model",
                &device,
                model_file_path,
                #[inline]
                |&MaxBounds { center, size }| {
                    let &[cx, cy, cz, _] = center.neg().as_array();

                    // IMPORTANT: Normalize the world coordinates to a reasonable range ~[0, 1].
                    // 1. Camera distance is invariant of the model's coordinate range
                    // 2. Dramatically reduces precision errors (compared to ranges >1000, like in Yoda model)
                    //    - In the Vertex Shader, z-fighting in the depth buffer, even with Depth32Float.
                    //    - In the Fragment Shader, diffuse and specular lighting is no longer smooth and
                    //      exhibit a weird triangal-ish pattern.
                    let scale = 1. / size.reduce_max();

                    // Put the plane (subsequent RenderableModelObject) right below the model.
                    // Store the floating point value as a u32 normalized ([0,1] -> [0,u32::MAX]).
                    plane_y = 0.5 * scale * size[2];
                    assert!(plane_y >= 0.0 && plane_y <= 1.0, "Calculated Y-coordinate of the Plane is invalid. Calculation is based on the bounding box size of model.");

                    (f32x4x4::scale(scale, scale, scale, 1.)
                        * (f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.)))
                        * f32x4x4::translate(cx, cy, cz)
                },
            ),
            model_light: ModelInstance::new::<80, _>(
                "Light",
                &device,
                assets_dir.join("light").join("light.obj"),
                |_| f32x4x4::translate(0., 0., 0.),
            ),
            model_plane: ModelInstance::new::<DEFAULT_AMBIENT_AMOUNT, _>(
                "Plane",
                &device,
                assets_dir.join("plane").join("plane.obj"),
                |_| f32x4x4::translate(0., -plane_y, 0.),
            ),
            // TODO: Change create_pipline to take a Function objects and create helper for getting many
            // function in one shot.
            // - There's alot of reusing of vertex and fragment functions
            //    - `main_vertex` x 2
            // - Alternatively (maybe even better), we extract the descriptor and allow callers to mutate
            //   and reuse the descriptor.
            //    1. Create descriptor
            //    2. Create Pipeline 1
            //    3. Change fragment function
            //    4. Create Pipeline 2
            //    5. etc.
            model_pipeline: create_pipeline(&device, &library, shading_mode),
            // TODO: How much instruction reduction do we get if we reorder/group like things together.
            // - Pipeline Creation
            // - Depth State Creation
            // - Camera and Light Space Creation
            shadow_map_pipeline: RenderPipeline::new(
                "Shadow Map",
                &device,
                &library,
                [],
                main_vertex,
                NoFragmentFunction,
                (Depth(DEPTH_TEXTURE_FORMAT), NoStencil),
            ),
            shading_mode,
            shadow_map_texture: None,
            needs_render: false,
            needs_render_shadow_map: true,
            library,
            device,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        let needs_render_shadow_map = std::mem::replace(&mut self.needs_render_shadow_map, false);
        self.needs_render = false;

        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        command_buffer.set_label("Renderer Command Buffer");

        // Render Shadow Map
        if needs_render_shadow_map {
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
                &self.depth_state,
                &[&HeapUsage(
                    &self.model.model.heap,
                    MTLRenderStages::Vertex | MTLRenderStages::Fragment,
                )],
                |p| {
                    p.bind(
                        main_vertex_binds {
                            model: Bind::Value(&ModelSpace {
                                matrix_model_to_projection: (self.light_matrix_world_to_projection
                                    * self.model.matrix_model_to_world),
                                matrix_normal_to_world: self.model.matrix_model_to_world.into(),
                            }),
                            geometry: Bind::Skip,
                        },
                        NoBinds,
                    );
                    for draw in self.model.model.draws() {
                        p.draw_primitives_with_bind(
                            main_vertex_binds {
                                model: Bind::Skip,
                                geometry: Bind::buffer_with_rolling_offset(draw.geometry),
                            },
                            NoBinds,
                            MTLPrimitiveType::Triangle,
                            0,
                            draw.vertex_count,
                        )
                    }
                },
            );
        }
        // Render Models
        let shadow_map_texture = self
            .shadow_map_texture
            .as_deref()
            .expect("Failed to access Shadow Map texture");
        self.model_pipeline.new_pass(
            "Render Models",
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
                &HeapUsage(&self.model.model.heap, USAGE_RENDER_STAGES),
                &HeapUsage(&self.model_plane.model.heap, USAGE_RENDER_STAGES),
                &HeapUsage(&self.model_light.model.heap, USAGE_RENDER_STAGES),
                &TextureUsage(
                    shadow_map_texture,
                    MTLResourceUsage::Sample,
                    MTLRenderStages::Fragment,
                ),
            ],
            |p| {
                p.bind(
                    main_vertex_binds::SKIP,
                    main_fragment_binds {
                        camera: Bind::Value(&self.camera_space),
                        light: Bind::Value(&self.light_space),
                        shadow_tx: BindTexture::Texture(shadow_map_texture),
                        ..Binds::SKIP
                    },
                );
                for m in [&self.model_light, &self.model, &self.model_plane] {
                    p.debug_group(m.name, || {
                        p.bind(
                            main_vertex_binds {
                                model: Bind::Value(&m.model_space),
                                ..Binds::SKIP
                            },
                            Binds::SKIP,
                        );
                        for draw in m.model.draws() {
                            p.draw_primitives_with_bind(
                                main_vertex_binds {
                                    geometry: Bind::buffer_with_rolling_offset(draw.geometry),
                                    ..Binds::SKIP
                                },
                                main_fragment_binds {
                                    material: Bind::iterating_buffer_offset(
                                        draw.geometry.1,
                                        draw.material,
                                    ),
                                    ..Binds::SKIP
                                },
                                MTLPrimitiveType::Triangle,
                                0,
                                draw.vertex_count,
                            );
                        }
                    });
                }
            },
        );
        command_buffer
    }

    #[inline]
    fn on_event(&mut self, event: UserEvent) {
        if let Some(update) = self.camera.on_event(event) {
            self.camera_space = ProjectedSpace {
                matrix_world_to_projection: update.matrix_world_to_projection,
                matrix_screen_to_world: update.matrix_screen_to_world,
                position_world: update.position_world.into(),
            };
            for m in [
                &mut self.model,
                &mut self.model_light,
                &mut self.model_plane,
            ] {
                m.on_camera_update(update.matrix_world_to_projection);
            }
            self.needs_render = true;
        }
        if let Some(update) = self.light.on_event(event) {
            self.model_light.matrix_model_to_world = update.matrix_camera_to_world
                * f32x4x4::y_rotate(PI)
                * f32x4x4::scale(0.1, 0.1, 0.1, 1.0);
            self.model_light
                .on_camera_update(self.camera_space.matrix_world_to_projection);
            self.light_matrix_world_to_projection = update.matrix_world_to_projection;
            self.light_space = ProjectedSpace {
                //
                // IMPORTANT: Projecting to a Texture, NOT to the screen.
                // Used to sample Shadow Map Depth Texture during shading to produce shadows.
                //
                // This projected coordinate space differs from the screen coordinate space (Metal
                // Normalized Device Coordinates), in the following ways:
                // - XY dimension range:      [-1,1] -> [0,1]
                // - Y dimension is inverted: +Y     -> -Y
                // - Z includes a bias for better depth comparison
                //
                matrix_world_to_projection: {
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
                } * update.matrix_world_to_projection,
                matrix_screen_to_world: update.matrix_screen_to_world,
                position_world: update.position_world.into(),
            };
            self.needs_render = true;
            self.needs_render_shadow_map = true;
        }
        if self.shading_mode.on_event(event) {
            self.model_pipeline = create_pipeline(&self.device, &self.library, self.shading_mode);
            self.needs_render = true;
        }
        if self.depth_texture.on_event(event, &self.device) {
            let t = self.depth_texture.texture();
            // Make sure the shadow map texture is atleast 2x and no more than 4x, of the
            // screen size. Round up to the nearest power of 2 of each dimension.
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
            let texture = self.device.new_texture(&desc);
            texture.set_label("Shadow Map Depth");
            self.shadow_map_texture = Some(texture);
            self.needs_render;
        }
    }

    #[inline(always)]
    fn needs_render(&self) -> bool {
        self.needs_render
    }

    #[inline]
    fn device(&self) -> &Device {
        &self.device
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 7 - Shadow Mapping");
}
