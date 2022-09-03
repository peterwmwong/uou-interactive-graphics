#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::{Camera, DepthTexture, ShadingModeSelector, ShadowMapTexture},
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
    simd::{f32x2, SimdFloat},
};

const DEFAULT_AMBIENT_AMOUNT: u32 = 15;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 5., PI / 16.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const USAGE_RENDER_STAGES: MTLRenderStages = unsafe {
    MTLRenderStages::from_bits_unchecked(
        MTLRenderStages::Vertex.bits() | MTLRenderStages::Fragment.bits(),
    )
};

struct ModelInstance {
    m_model_to_world: f32x4x4,
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
        init_m_model_to_world: impl FnOnce(&MaxBounds) -> f32x4x4,
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
            m_model_to_world: init_m_model_to_world(&model.geometry_max_bounds),
            model,
            model_space: ModelSpace::default(),
            name,
        }
    }

    fn on_camera_update(&mut self, camera_m_world_to_projection: f32x4x4) {
        self.model_space = ModelSpace {
            m_model_to_projection: (camera_m_world_to_projection * self.m_model_to_world),
            m_normal_to_world: self.m_model_to_world.into(),
        };
    }
}

struct Delegate {
    camera: Camera,
    command_queue: CommandQueue,
    depth_state: DepthStencilState,
    depth_texture: DepthTexture,
    device: Device,
    library: Library,
    light_space: ProjectedSpace,
    light: Camera,
    model_light: ModelInstance,
    model_pipeline: RenderPipeline<1, main_vertex, main_fragment, (Depth, NoStencil)>,
    model_plane: ModelInstance,
    model: ModelInstance,
    model_shadow_space: ModelSpace,
    needs_render: bool,
    needs_render_shadow_map: bool,
    shading_mode: ShadingModeSelector,
    shadow_map_pipeline: RenderPipeline<0, main_vertex, NoFragmentFunction, (Depth, NoStencil)>,
    shadow_map_texture: ShadowMapTexture,
}

fn create_pipeline(
    device: &Device,
    library: &Library,
    shading_mode: ShadingModeSelector,
) -> RenderPipeline<1, main_vertex, main_fragment, (Depth, NoStencil)> {
    RenderPipeline::new(
        "Model",
        device,
        library,
        [(DEFAULT_COLOR_FORMAT, BlendMode::NoBlend)],
        main_vertex,
        main_fragment {
            HasAmbient: shading_mode.has_ambient(),
            HasDiffuse: shading_mode.has_diffuse(),
            OnlyNormals: shading_mode.only_normals(),
            HasSpecular: shading_mode.has_specular(),
        },
        (Depth(DEFAULT_DEPTH_FORMAT), NoStencil),
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
            command_queue: device.new_command_queue(),
            depth_texture: DepthTexture::new("Depth", DEFAULT_DEPTH_FORMAT),
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
            model_shadow_space: ModelSpace::default(),
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
            model_pipeline: create_pipeline(&device, &library, shading_mode),
            shadow_map_pipeline: RenderPipeline::new(
                "Shadow Map",
                &device,
                &library,
                [],
                main_vertex,
                NoFragmentFunction,
                (Depth(DEFAULT_DEPTH_FORMAT), NoStencil),
            ),
            shading_mode,
            shadow_map_texture: ShadowMapTexture::new("Shadow Map", DEFAULT_DEPTH_FORMAT),
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
        let shadow_tx = self.shadow_map_texture.texture();
        let depth_tx = self.depth_texture.texture();
        if needs_render_shadow_map {
            self.shadow_map_pipeline.new_pass(
                "Shadow Map",
                command_buffer,
                [],
                (shadow_tx, 1., MTLLoadAction::Clear, MTLStoreAction::Store),
                NoStencil,
                &self.depth_state,
                MTLCullMode::None,
                &[&HeapUsage(&self.model.model.heap, USAGE_RENDER_STAGES)],
                |p| {
                    p.set_depth_bias(1.0, 15.0, 0.0);
                    p.bind(
                        main_vertex_binds {
                            model: Bind::Value(&self.model_shadow_space),
                            geometry: Bind::Skip,
                        },
                        NoBinds,
                    );
                    for draw in self.model.model.draws() {
                        p.draw_primitives_with_binds(
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
        self.model_pipeline.new_pass(
            "Model, Plane, and Light",
            command_buffer,
            [(
                render_target,
                (0., 0., 0., 0.),
                MTLLoadAction::Clear,
                MTLStoreAction::Store,
            )],
            (depth_tx, 1., MTLLoadAction::Clear, MTLStoreAction::DontCare),
            NoStencil,
            &self.depth_state,
            MTLCullMode::None,
            &[
                &HeapUsage(&self.model.model.heap, USAGE_RENDER_STAGES),
                &HeapUsage(&self.model_plane.model.heap, USAGE_RENDER_STAGES),
                &HeapUsage(&self.model_light.model.heap, USAGE_RENDER_STAGES),
                &TextureUsage(
                    shadow_tx,
                    MTLResourceUsage::Sample,
                    MTLRenderStages::Fragment,
                ),
            ],
            |p| {
                p.bind(
                    main_vertex_binds::SKIP,
                    main_fragment_binds {
                        camera: Bind::Value(&self.camera.projected_space),
                        light: Bind::Value(&self.light_space),
                        shadow_tx: BindTexture(shadow_tx),
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
                            p.draw_primitives_with_binds(
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
        if self.camera.on_event(event) {
            for m in [
                &mut self.model,
                &mut self.model_light,
                &mut self.model_plane,
            ] {
                m.on_camera_update(self.camera.projected_space.m_world_to_projection);
            }
            self.needs_render = true;
        }
        if self.light.on_event(event) {
            self.model_light.m_model_to_world = self.light.get_camera_to_world_transform()
                * f32x4x4::y_rotate(PI)
                * f32x4x4::scale(0.1, 0.1, 0.1, 1.0);
            self.model_light
                .on_camera_update(self.camera.projected_space.m_world_to_projection);
            self.model_shadow_space = ModelSpace {
                m_model_to_projection: (self.light.projected_space.m_world_to_projection
                    * self.model.m_model_to_world),
                m_normal_to_world: self.model.m_model_to_world.into(),
            };
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
                m_world_to_projection: {
                    // Performance: Bake all the transform at compile time.
                    // - Currently Rust does not allow floating-point operations in constant
                    //   expressions.
                    // - Reduces the amount of code generated/executed, >150 bytes of instructions
                    //   saved.
                    const PROJECTION_TO_TEXTURE_COORDINATE_SPACE: f32x4x4 = f32x4x4::new(
                        [0.5, 0.0, 0.0, 0.5],
                        [0.0, -0.5, 0.0, 0.5],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    );
                    #[cfg(debug_assertions)]
                    {
                        // Invert Y
                        let projection_to_texture_coordinate_space_derived =
                            f32x4x4::scale_translate(1., -1., 1., 0., 1., 0.)
                                * f32x4x4::scale_translate(
                                    // Convert from [-1, 1] -> [0, 1] for XY dimensions
                                    0.5, 0.5, 1.0, 0.5, 0.5, // Add Depth Comparison Bias
                                    0.0,
                                );
                        assert_eq!(
                            projection_to_texture_coordinate_space_derived.columns,
                            PROJECTION_TO_TEXTURE_COORDINATE_SPACE.columns
                        );
                    }
                    PROJECTION_TO_TEXTURE_COORDINATE_SPACE
                } * self.light.projected_space.m_world_to_projection,
                m_screen_to_world: self.light.projected_space.m_screen_to_world,
                position_world: self.light.projected_space.position_world.into(),
            };
            self.needs_render = true;
            self.needs_render_shadow_map = true;
        }
        if self.shading_mode.on_event(event) {
            self.model_pipeline = create_pipeline(&self.device, &self.library, self.shading_mode);
            self.needs_render = true;
        }
        if self.depth_texture.on_event(event, &self.device) {
            self.needs_render;
        }
        if self.shadow_map_texture.on_event(event, &self.device) {
            self.needs_render_shadow_map = true;
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

fn main() {
    launch_application::<Delegate>("Project 7 - Shadow Mapping");
}
