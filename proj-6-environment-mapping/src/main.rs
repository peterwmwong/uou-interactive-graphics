#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::{Camera, DepthTexture, ShadingModeSelector},
    metal::*,
    metal_types::*,
    pipeline::*,
    *,
};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    ops::Neg,
    path::PathBuf,
    simd::{f32x2, f32x4, SimdFloat},
};

const BG_STENCIL_REF_VALUE: u32 = 0;
const MIRROR_PLANE_STENCIL_REF_VALUE: u32 = 1;
const MODEL_STENCIL_REF_VALUE: u32 = 2;

const STENCIL_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Stencil8;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 32., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_POSITION: f32x4 = f32x4::from_array([0., 1., -1., 1.]);

struct Delegate {
    bg_depth_state: DepthStencilState,
    bg_render_pipeline: RenderPipeline<1, bg_vertex, bg_fragment, (Depth, Stencil)>,
    camera_space: ProjectedSpace,
    camera: Camera,
    command_queue: CommandQueue,
    cubemap_texture: Texture,
    depth_texture: DepthTexture,
    device: Device,
    library: Library,
    main_render_pipeline: RenderPipeline<1, main_vertex, main_fragment, (Depth, Stencil)>,
    m_mirror_plane_model_to_world: f32x4x4,
    m_model_to_world: f32x4x4,
    m_world_to_mirror_world: f32x4x4,
    mirror_camera_space: ProjectedSpace,
    mirror_light_position: f32x4,
    mirror_model_depth_state: DepthStencilState,
    mirror_model_space: ModelSpace,
    mirror_plane_depth_state: DepthStencilState,
    mirror_plane_model_space: ModelSpace,
    mirror_plane_model: Model<Geometry, NoMaterial>,
    model_depth_state: DepthStencilState,
    model_space: ModelSpace,
    model: Model<Geometry, NoMaterial>,
    needs_render: bool,
    shading_mode: ShadingModeSelector,
    stencil_texture: DepthTexture,
}

fn create_main_render_pipeline(
    device: &Device,
    library: &Library,
    mode: ShadingModeSelector,
) -> RenderPipeline<1, main_vertex, main_fragment, (Depth, Stencil)> {
    RenderPipeline::new(
        "Model",
        device,
        library,
        [(DEFAULT_COLOR_FORMAT, BlendMode::NoBlend)],
        main_vertex,
        main_fragment {
            HasAmbient: mode.contains(ShadingModeSelector::HAS_AMBIENT),
            HasDiffuse: mode.contains(ShadingModeSelector::HAS_DIFFUSE),
            OnlyNormals: mode.contains(ShadingModeSelector::ONLY_NORMALS),
            HasSpecular: mode.contains(ShadingModeSelector::HAS_SPECULAR),
        },
        (Depth(DEFAULT_DEPTH_FORMAT), Stencil(STENCIL_TEXTURE_FORMAT)),
    )
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let cubemap_texture = debug_time("proj6 - Load Environment Cube Texture", || {
            asset_compiler::cube_texture::load_cube_texture_asset_dir(
                &device,
                &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/cubemap.asset"),
            )
        });
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
            NoMaterial,
        );
        let mirror_plane_model = Model::from_file(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../common-assets/plane/plane.obj"),
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
        let m_model_to_world = (f32x4x4::scale(scale, scale, scale, 1.)
            * (f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.)))
            * f32x4x4::translate(cx, cy, cz);

        let mirror_plane_y_world = -0.5 * scale * size[2];
        let m_mirror_plane_model_to_world =
            f32x4x4::translate(0., mirror_plane_y_world, 0.) * f32x4x4::scale(0.9, 0.9, 0.9, 1.);

        let m_world_to_mirror_world =
            // 3. Put the object back into world coordinate space (mirror world)
            f32x4x4::translate(0., mirror_plane_y_world, 0.)
            // 2. Mirror
            * f32x4x4::scale(1., -1., 1., 1.)
            // 1. Move objects into mirror plane coordinate space (aka origin is mirror plane)
            * f32x4x4::translate(0., -mirror_plane_y_world, 0.);

        // Interesting observation: This transformation is an involutory matrix?
        // TODO: How does this work? Does this hold up when we rotate the mirror plane?
        // let m_mirror_world_to_world = m_world_to_mirror_world.inverse();
        // assert_eq!(m_world_to_mirror_world, m_mirror_world_to_world);
        let mirror_light_position = m_world_to_mirror_world * LIGHT_POSITION;

        let shading_mode = ShadingModeSelector::DEFAULT;
        let ds = DepthStencilDescriptor::new();
        let s = StencilDescriptor::new();
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        Self {
            bg_depth_state: {
                ds.set_depth_write_enabled(false);
                ds.set_depth_compare_function(MTLCompareFunction::LessEqual);
                {
                    s.set_stencil_compare_function(MTLCompareFunction::Equal);
                    s.set_depth_stencil_pass_operation(MTLStencilOperation::Keep);
                    ds.set_front_face_stencil(Some(&s));
                    ds.set_back_face_stencil(Some(&s));
                }
                device.new_depth_stencil_state(&ds)
            },
            mirror_model_depth_state: {
                ds.set_depth_write_enabled(true);
                ds.set_depth_compare_function(MTLCompareFunction::LessEqual);
                {
                    s.set_stencil_compare_function(MTLCompareFunction::Equal);
                    s.set_depth_stencil_pass_operation(MTLStencilOperation::Keep);
                    ds.set_front_face_stencil(Some(&s));
                    ds.set_back_face_stencil(Some(&s));
                }
                device.new_depth_stencil_state(&ds)
            },
            mirror_plane_depth_state: {
                ds.set_depth_write_enabled(false);
                ds.set_depth_compare_function(MTLCompareFunction::LessEqual);
                {
                    s.set_stencil_compare_function(MTLCompareFunction::Always);
                    s.set_depth_stencil_pass_operation(MTLStencilOperation::Replace);
                    ds.set_front_face_stencil(Some(&s));
                    ds.set_back_face_stencil(Some(&s));
                }
                device.new_depth_stencil_state(&ds)
            },
            model_depth_state: {
                ds.set_depth_write_enabled(true);
                ds.set_depth_compare_function(MTLCompareFunction::LessEqual);
                {
                    s.set_stencil_compare_function(MTLCompareFunction::Always);
                    s.set_depth_stencil_pass_operation(MTLStencilOperation::Replace);
                    ds.set_front_face_stencil(Some(&s));
                    ds.set_back_face_stencil(Some(&s));
                }
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
                (Depth(DEFAULT_DEPTH_FORMAT), Stencil(STENCIL_TEXTURE_FORMAT)),
            ),
            main_render_pipeline: create_main_render_pipeline(&device, &library, shading_mode),
            m_mirror_plane_model_to_world,
            m_model_to_world,
            m_world_to_mirror_world,
            mirror_light_position,
            command_queue: device.new_command_queue(),
            camera: Camera::new_with_default_distance(
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            depth_texture: DepthTexture::new("Depth", DEFAULT_DEPTH_FORMAT),
            camera_space: ProjectedSpace::default(),
            mirror_camera_space: ProjectedSpace::default(),
            mirror_model_space: ModelSpace::default(),
            mirror_plane_model_space: ModelSpace::default(),
            model_space: ModelSpace::default(),
            needs_render: false,
            shading_mode,
            stencil_texture: DepthTexture::new("Stencil", STENCIL_TEXTURE_FORMAT),
            mirror_plane_model,
            model,
            device,
            library,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        let draw_model = |p: &RenderPass<1, main_vertex, main_fragment, (Depth, Stencil)>,
                          model: &Model<Geometry, NoMaterial>| {
            for draw in model.draws() {
                p.debug_group(draw.name, || {
                    p.draw_primitives_with_bind(
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
        let stenc_tx = self.stencil_texture.texture();
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
            (stenc_tx, 0, MTLLoadAction::Clear, MTLStoreAction::DontCare),
            (
                &self.model_depth_state,
                MODEL_STENCIL_REF_VALUE,
                MODEL_STENCIL_REF_VALUE,
            ),
            &[
                &HeapUsage(
                    &self.model.heap,
                    MTLRenderStages::Vertex | MTLRenderStages::Fragment,
                ),
                &HeapUsage(
                    &self.mirror_plane_model.heap,
                    MTLRenderStages::Vertex | MTLRenderStages::Fragment,
                ),
            ],
            |p| {
                p.bind(
                    main_vertex_binds {
                        camera: Bind::Value(&self.camera_space),
                        model: Bind::Value(&self.model_space),
                        ..main_vertex_binds::SKIP
                    },
                    main_fragment_binds {
                        camera: Bind::Value(&self.camera_space),
                        light_pos: Bind::Value(&LIGHT_POSITION.into()),
                        m_env: Bind::Value(&f32x4x4::identity().into()),
                        darken: Bind::Value(&0_f32),
                        env_texture: BindTexture::Texture(&self.cubemap_texture),
                    },
                );
                draw_model(&p, &self.model);
                p.debug_group("Plane", || {
                    p.set_depth_stencil_state((
                        &self.mirror_plane_depth_state,
                        MIRROR_PLANE_STENCIL_REF_VALUE,
                        MIRROR_PLANE_STENCIL_REF_VALUE,
                    ));
                    p.bind(
                        main_vertex_binds {
                            model: Bind::Value(&self.mirror_plane_model_space),
                            ..main_vertex_binds::SKIP
                        },
                        main_fragment_binds::SKIP,
                    );
                    draw_model(&p, &self.mirror_plane_model);
                });
                p.debug_group("Model (mirrored)", || {
                    p.set_depth_stencil_state((
                        &self.mirror_model_depth_state,
                        MIRROR_PLANE_STENCIL_REF_VALUE,
                        MIRROR_PLANE_STENCIL_REF_VALUE,
                    ));
                    p.bind(
                        main_vertex_binds {
                            camera: Bind::Value(&self.mirror_camera_space),
                            model: Bind::Value(&self.mirror_model_space),
                            ..main_vertex_binds::SKIP
                        },
                        main_fragment_binds {
                            camera: Bind::Value(&self.mirror_camera_space),
                            darken: Bind::Value(&0.5),
                            light_pos: Bind::Value(&self.mirror_light_position.into()),
                            m_env: Bind::Value(&self.m_world_to_mirror_world.into()),
                            ..main_fragment_binds::SKIP
                        },
                    );
                    draw_model(&p, &self.model);
                });
                p.into_subpass(
                    "BG",
                    &self.bg_render_pipeline,
                    Some((
                        &self.bg_depth_state,
                        BG_STENCIL_REF_VALUE,
                        BG_STENCIL_REF_VALUE,
                    )),
                    |p| {
                        p.draw_primitives_with_bind(
                            NoBinds,
                            bg_fragment_binds {
                                camera: Bind::Value(&self.camera_space),
                                ..bg_fragment_binds::SKIP
                            },
                            MTLPrimitiveType::Triangle,
                            0,
                            3,
                        )
                    },
                );
            },
        );
        command_buffer
    }

    #[inline]
    fn on_event(&mut self, event: UserEvent) {
        if let Some(u) = self.camera.on_event(event) {
            self.camera_space = ProjectedSpace {
                m_world_to_projection: u.m_world_to_projection,
                m_screen_to_world: u.m_screen_to_world,
                position_world: u.position_world.into(),
            };
            self.mirror_camera_space = ProjectedSpace {
                m_world_to_projection: self.camera_space.m_world_to_projection
                    * self.m_world_to_mirror_world,
                // TODO: I'm not sure this is right, shouldn't it be m_mirror_world_to_world, not m_world_to_mirror_world
                //                                                     aaaaaaaaaaaa    bbbbb        bbbbb    aaaaaaaaaaaa
                // - I think this only works because m_world_to_mirror_world is involution (inverse is the same).
                // - `m_world_to_projection` (Metal than transforms to screen)
                //      world -> mirror world -> projection -> screen
                // - Also, does the Fragment Shader need a mirror world or world coordinate?
                //      - `m_screen_to_world`
                //          screen -> projection -> world -> mirror world (current)
                //          VS.
                //          screen -> projection -> mirror world -> world
                m_screen_to_world: self.m_world_to_mirror_world
                    * self.camera_space.m_screen_to_world,
                position_world: self.camera_space.position_world,
            };

            self.model_space = ModelSpace {
                m_model_to_projection: self.camera_space.m_world_to_projection
                    * self.m_model_to_world,
                m_normal_to_world: self.m_model_to_world.into(),
            };
            self.mirror_plane_model_space = ModelSpace {
                m_model_to_projection: self.camera_space.m_world_to_projection
                    * self.m_mirror_plane_model_to_world,
                m_normal_to_world: self.m_mirror_plane_model_to_world.into(),
            };
            self.mirror_model_space = ModelSpace {
                m_model_to_projection: self.mirror_camera_space.m_world_to_projection
                    * self.m_model_to_world,
                m_normal_to_world: (self.m_world_to_mirror_world * self.m_model_to_world).into(),
            };
            self.needs_render = true;
        }
        if self.shading_mode.on_event(event) {
            self.main_render_pipeline =
                create_main_render_pipeline(&self.device, &self.library, self.shading_mode);
            self.needs_render = true;
        }
        if self.depth_texture.on_event(event, &self.device) {
            self.needs_render = true;
        }
        if self.stencil_texture.on_event(event, &self.device) {
            self.needs_render = true;
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
