#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::{Camera, DepthTexture, ShadingModeSelector},
    metal::*,
    metal_types::*,
    model_acceleration_structure::ModelAccelerationStructure,
    pipeline::*,
    typed_buffer::TypedBuffer,
    *,
};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    ops::{Deref, Neg},
    path::PathBuf,
    simd::{f32x2, f32x4, SimdFloat},
};

const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 8., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_POSITION: f32x4 = f32x4::from_array([0., 1., -1., 1.]);

struct Delegate {
    bg_depth_state: DepthStencilState,
    bg_render_pipeline: RenderPipeline<1, bg_vertex, bg_fragment, (Depth, NoStencil)>,
    dbg_render_pipeline: RenderPipeline<1, dbg_vertex, dbg_fragment, (Depth, NoStencil)>,
    camera: Camera,
    command_queue: CommandQueue,
    cubemap_texture: Texture,
    depth_texture: DepthTexture,
    debug_path: TypedBuffer<DebugPath>,
    has_debug_path: bool,
    device: Device,
    library: Library,
    main_render_pipeline: RenderPipeline<1, main_vertex, main_fragment, (Depth, NoStencil)>,
    m_model_to_world: f32x4x4,
    m_mirror_plane_model_to_world: f32x4x4,
    model_depth_state: DepthStencilState,
    model_space: ModelSpace,
    mirror_plane_space: ModelSpace,
    model: Model<GeometryNoTxCoords, NoMaterial>,
    mirror_plane_model: Model<GeometryNoTxCoords, NoMaterial>,
    needs_render: bool,
    shading_mode: ShadingModeSelector,
    world_as: ModelAccelerationStructure,
}

fn create_main_render_pipeline(
    device: &Device,
    library: &Library,
    shading_mode: ShadingModeSelector,
    has_debug_path: bool,
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
            HasDebugPath: has_debug_path,
        },
        (Depth(DEFAULT_DEPTH_FORMAT), NoStencil),
    )
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let cubemap_texture = debug_time("proj6 - Load Environment Cube Texture", || {
            asset_compiler::cube_texture::load_cube_texture_asset_dir(
                &device,
                &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../proj-6-environment-mapping/assets/cubemap.asset"),
            )
        });
        let executable_name = std::env::args()
            .nth(0)
            .expect("Failed to access command line executable name");
        let model_file_path = PathBuf::from(std::env::args().nth(1).expect(&format!(
            "Usage: {executable_name} [Path to Wavefront OBJ file]"
        )));
        let mirror_plane_file_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../common-assets/plane/plane.obj");
        let encode_geometry_arg = |arg: &mut GeometryNoTxCoords, g: GeometryToEncode| {
            arg.indices = g.indices_buffer;
            arg.positions = g.positions_buffer;
            arg.normals = g.normals_buffer;
        };
        let model = Model::from_file(&model_file_path, &device, encode_geometry_arg, NoMaterial);
        let mirror_plane_model = Model::from_file(
            &mirror_plane_file_path,
            &device,
            encode_geometry_arg,
            NoMaterial,
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
        let m_model_to_world = f32x4x4::scale(scale, scale, scale, 1.)
            * f32x4x4::y_rotate(PI)
            * f32x4x4::x_rotate(PI / 2.)
            * f32x4x4::translate(cx, cy, cz);

        let mirror_plane_y_world = -0.5 * scale * size[2];
        let m_mirror_plane_model_to_world =
            f32x4x4::translate(0., mirror_plane_y_world, 0.) * f32x4x4::scale(0.9, 0.9, 0.9, 1.);

        let command_queue = device.new_command_queue();
        let world_as = ModelAccelerationStructure::from_files(
            &[&model_file_path, &mirror_plane_file_path],
            &device,
            &command_queue,
            |_, i| {
                if i == 0 {
                    m_model_to_world
                } else {
                    m_mirror_plane_model_to_world
                }
            },
        );

        let shading_mode = ShadingModeSelector::DEFAULT;
        let has_debug_path = false;
        let ds = DepthStencilDescriptor::new();
        ds.set_depth_compare_function(MTLCompareFunction::LessEqual);
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        Self {
            bg_depth_state: {
                ds.set_depth_write_enabled(false);
                device.new_depth_stencil_state(&ds)
            },
            model_depth_state: {
                ds.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&ds)
            },
            cubemap_texture,
            bg_render_pipeline: RenderPipeline::new(
                "BG",
                &device,
                &library,
                [(DEFAULT_COLOR_FORMAT, BlendMode::NoBlend)],
                bg_vertex,
                bg_fragment,
                (Depth(DEFAULT_DEPTH_FORMAT), NoStencil),
            ),
            dbg_render_pipeline: RenderPipeline::new(
                "Debug",
                &device,
                &library,
                [(DEFAULT_COLOR_FORMAT, BlendMode::NoBlend)],
                dbg_vertex,
                dbg_fragment,
                (Depth(DEFAULT_DEPTH_FORMAT), NoStencil),
            ),
            main_render_pipeline: create_main_render_pipeline(
                &device,
                &library,
                shading_mode,
                has_debug_path,
            ),
            m_model_to_world,
            m_mirror_plane_model_to_world,
            command_queue,
            camera: Camera::new_with_default_distance(
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            depth_texture: DepthTexture::new("Depth", DEFAULT_DEPTH_FORMAT),
            model_space: ModelSpace {
                m_model_to_projection: f32x4x4::identity().into(),
                m_normal_to_world: m_model_to_world.into(),
            },
            mirror_plane_space: ModelSpace {
                m_model_to_projection: f32x4x4::identity().into(),
                m_normal_to_world: m_mirror_plane_model_to_world.into(),
            },
            needs_render: false,
            shading_mode,
            world_as,
            model,
            mirror_plane_model,
            has_debug_path,
            debug_path: TypedBuffer::from_data(
                "DebugPath",
                device.deref(),
                &[DebugPath::default()],
                MTLResourceOptions::StorageModeShared,
            ),
            device,
            library,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        let draw_model = |p: &RenderPass<1, main_vertex, main_fragment, (Depth, NoStencil)>,
                          model: &Model<GeometryNoTxCoords, NoMaterial>| {
            for draw in model.draws() {
                p.debug_group(draw.name, || {
                    p.draw_primitives_with_binds(
                        main_vertex_binds {
                            geometry: Bind::buffer_with_rolling_offset(draw.geometry),
                            ..main_vertex_binds::SKIP
                        },
                        main_fragment_binds::SKIP,
                        MTLPrimitiveType::Triangle,
                        0,
                        draw.vertex_count,
                    )
                })
            }
        };
        self.needs_render = false;
        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        command_buffer.set_label("Renderer Command Buffer");
        let depth_tx = self.depth_texture.texture();
        self.main_render_pipeline.new_pass(
            "Model",
            command_buffer,
            [(
                render_target,
                (0., 0., 0., 0.),
                MTLLoadAction::Clear,
                MTLStoreAction::Store,
            )],
            (depth_tx, 1., MTLLoadAction::Clear, MTLStoreAction::DontCare),
            NoStencil,
            &self.model_depth_state,
            &[
                &HeapUsage(
                    &self.model.heap,
                    MTLRenderStages::Vertex | MTLRenderStages::Fragment,
                ),
                &self.world_as.resource(),
            ],
            |p| {
                p.bind(
                    main_vertex_binds {
                        camera: Bind::Value(&self.camera.projected_space),
                        model: Bind::Value(&self.model_space),
                        ..main_vertex_binds::SKIP
                    },
                    main_fragment_binds {
                        camera: Bind::Value(&self.camera.projected_space),
                        light_pos: Bind::Value(&LIGHT_POSITION.into()),
                        accel_struct: self.world_as.bind(),
                        env_texture: BindTexture(&self.cubemap_texture),
                        m_model_to_worlds: BindMany::buffer(
                            &self.world_as.m_model_to_worlds_buffer,
                        ),
                        dbg_path: Bind::buffer(&self.debug_path),
                    },
                );
                draw_model(&p, &self.model);
                p.bind(
                    main_vertex_binds {
                        model: Bind::Value(&self.mirror_plane_space),
                        ..main_vertex_binds::SKIP
                    },
                    main_fragment_binds::SKIP,
                );
                draw_model(&p, &self.mirror_plane_model);
                p.into_subpass(
                    "BG",
                    &self.bg_render_pipeline,
                    Some(&self.bg_depth_state),
                    |p| {
                        p.draw_primitives_with_binds(
                            NoBinds,
                            bg_fragment_binds::SKIP,
                            MTLPrimitiveType::Triangle,
                            0,
                            3,
                        );
                        if self.has_debug_path {
                            p.into_subpass("DebugPath", &self.dbg_render_pipeline, None, |p| {
                                p.draw_primitives_with_binds(
                                    dbg_vertex_binds {
                                        dbg_path: Bind::buffer(&self.debug_path),
                                        ..dbg_vertex_binds::SKIP
                                    },
                                    NoBinds,
                                    MTLPrimitiveType::LineStrip,
                                    0,
                                    DEBUG_PATH_MAX_NUM_POINTS as _,
                                )
                            });
                        }
                    },
                );
            },
        );
        command_buffer
    }

    #[inline]
    fn on_event(&mut self, event: UserEvent) {
        if self.camera.on_event(event) {
            self.model_space.m_model_to_projection =
                self.camera.projected_space.m_world_to_projection * self.m_model_to_world;
            self.mirror_plane_space.m_model_to_projection =
                self.camera.projected_space.m_world_to_projection
                    * self.m_mirror_plane_model_to_world;
            self.needs_render = true;
        }
        if self.shading_mode.on_event(event) {
            self.main_render_pipeline = create_main_render_pipeline(
                &self.device,
                &self.library,
                self.shading_mode,
                self.has_debug_path,
            );
            self.needs_render = true;
        }
        if self.depth_texture.on_event(event, &self.device) {
            self.needs_render = true;
        }
        // TODO: START HERE
        // TODO: START HERE
        // TODO: START HERE
        // Extract event handling and rendering into a component.
        let debug_path = &mut self.debug_path.get_mut()[0];
        match event {
            UserEvent::MouseDrag { position, .. } | UserEvent::MouseMoved { position }
                if !debug_path.update_disabled =>
            {
                let position = position / f32x2::splat(1.0);
                debug_path.screen_pos = position.into();
                self.needs_render = true;
            }
            UserEvent::KeyDown {
                key_code,
                modifier_keys,
                ..
            } if key_code == UserEvent::KEY_CODE_P => {
                if modifier_keys.contains(ModifierKeys::SHIFT) {
                    debug_path.update_disabled = !debug_path.update_disabled;
                } else if modifier_keys.is_empty() {
                    self.has_debug_path = !self.has_debug_path;
                    self.main_render_pipeline = create_main_render_pipeline(
                        &self.device,
                        &self.library,
                        self.shading_mode,
                        self.has_debug_path,
                    );
                    self.needs_render = true;
                }
            }
            _ => {}
        }
    }

    #[inline(always)]
    fn needs_render(&self) -> bool {
        self.needs_render
    }

    #[inline(always)]
    fn device(&self) -> &Device {
        &self.device
    }
}

fn main() {
    launch_application::<Delegate>("Project 6 - Environment Mapping");
}
