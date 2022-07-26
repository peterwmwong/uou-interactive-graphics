#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::{Camera, CameraUpdate, DepthTexture, ShadingModeSelector},
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

const DEFAULT_AMBIENT_AMOUNT: f32 = 0.15;
const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 4., 0.]);
const INITIAL_MODE: ShadingModeSelector = ShadingModeSelector::DEFAULT;
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_DISTANCE: f32 = 0.5;

pub struct Delegate<const RENDER_LIGHT: bool> {
    camera_space: ProjectedSpace,
    camera: Camera,
    command_queue: CommandQueue,
    depth_state: DepthStencilState,
    depth_texture: DepthTexture,
    device: Device,
    library: Library,
    light_pipeline: RenderPipeline<1, light_vertex, light_fragment, (Depth, NoStencil)>,
    light_position: float4,
    light: Camera,
    matrix_model_to_world: f32x4x4,
    model_pipeline: RenderPipeline<1, main_vertex, main_fragment, (Depth, NoStencil)>,
    model_space: ModelSpace,
    model: Model<Geometry, HasMaterial<Material>>,
    needs_render: bool,
    shading_mode: ShadingModeSelector,
}

fn create_model_pipeline(
    device: &Device,
    library: &Library,
    mode: ShadingModeSelector,
) -> RenderPipeline<1, main_vertex, main_fragment, (Depth, NoStencil)> {
    RenderPipeline::new(
        "Model",
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
    )
}

impl<const RENDER_LIGHT: bool> RendererDelgate for Delegate<RENDER_LIGHT> {
    fn new(device: Device) -> Self {
        let executable_name = std::env::args()
            .nth(0)
            .expect("Failed to access command line executable name");
        let model_file_path = std::env::args().nth(1).expect(&format!(
            "Usage: {executable_name} [Path to Wavefront OBJ file]"
        ));
        let model_file = PathBuf::from(model_file_path);
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        let mode = INITIAL_MODE;
        let model_pipeline = create_model_pipeline(&device, &library, mode);
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
            HasMaterial(
                |arg: &mut Material,
                 MaterialToEncode {
                     ambient_texture,
                     diffuse_texture,
                     specular_texture,
                     specular_shineness,
                 }| {
                    arg.ambient_texture = ambient_texture;
                    arg.diffuse_texture = diffuse_texture;
                    arg.specular_texture = specular_texture;
                    arg.specular_shineness = specular_shineness;
                    arg.ambient_amount = DEFAULT_AMBIENT_AMOUNT;
                },
            ),
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

        // TODO: DO IT. This generates an immense amount of code!
        // - It's the matrix multiplications we're unable to avoid with const evaluation (currently not supported in rust for floating point operations)
        // - We can create combo helpers, see f32x4x4::scale_translate()
        let model_to_world_scale_rot = f32x4x4::scale(scale, scale, scale, 1.)
            * (f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.));
        let matrix_model_to_world = model_to_world_scale_rot * f32x4x4::translate(cx, cy, cz);

        Self {
            camera: Camera::new_with_default_distance(
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            camera_space: ProjectedSpace {
                matrix_world_to_projection: f32x4x4::identity().into(),
                matrix_screen_to_world: f32x4x4::identity().into(),
                position_world: f32x4::default().into(),
            },
            command_queue: device.new_command_queue(),
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: DepthTexture::new("Depth", DEPTH_TEXTURE_FORMAT),
            light_position: f32x4::default().into(),
            light: Camera::new(
                LIGHT_DISTANCE,
                INITIAL_LIGHT_ROTATION,
                ModifierKeys::CONTROL,
                true,
                0.,
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
            matrix_model_to_world,
            model,
            model_space: ModelSpace {
                matrix_model_to_projection: f32x4x4::identity(),
                // IMPORTANT: Not a mistake, using Model-to-World 4x4 Matrix for Normal-to-World 3x3
                // Matrix. Conceptually, we want a matrix that ONLY applies rotation (no
                // translation). Since normals are directions (not positions, relative to a point on
                // a surface), translations are meaningless.
                matrix_normal_to_world: matrix_model_to_world.into(),
            },
            model_pipeline,
            needs_render: false,
            shading_mode: mode,
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
        self.model_pipeline.new_pass(
            "Render",
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
            &[&HeapUsage(
                &self.model.heap,
                MTLRenderStages::Vertex | MTLRenderStages::Fragment,
            )],
            |p| {
                p.debug_group("Model", || {
                    p.bind(
                        main_vertex_binds {
                            geometry: Bind::Skip,
                            model: Bind::Value(&self.model_space),
                        },
                        main_fragment_binds {
                            material: Bind::Skip,
                            camera: Bind::Value(&self.camera_space),
                            light_pos: Bind::Value(&self.light_position),
                        },
                    );
                    for DrawItem {
                        name,
                        vertex_count,
                        geometry,
                        material,
                    } in self.model.draws()
                    {
                        p.debug_group(name, || {
                            p.draw_primitives_with_bind(
                                main_vertex_binds {
                                    geometry: Bind::Buffer(BindBuffer::buffer_with_rolling_offset(
                                        geometry,
                                    )),
                                    model: Bind::Skip,
                                },
                                main_fragment_binds {
                                    material: Bind::Buffer(BindBuffer::iterating_buffer_offset(
                                        geometry.1, material,
                                    )),
                                    camera: Bind::Skip,
                                    light_pos: Bind::Skip,
                                },
                                MTLPrimitiveType::Triangle,
                                0,
                                vertex_count as _,
                            );
                        });
                    }
                });
                if RENDER_LIGHT {
                    p.into_subpass("Light", &self.light_pipeline, None, |p| {
                        p.draw_primitives_with_bind(
                            light_vertex_binds {
                                camera: Bind::Value(&self.camera_space),
                                light_pos: Bind::Value(&self.light_position),
                            },
                            NoBinds,
                            MTLPrimitiveType::Point,
                            0,
                            1,
                        )
                    });
                }
            },
        );
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        if let Some(u) = self.camera.on_event(event) {
            self.camera_space = ProjectedSpace {
                matrix_world_to_projection: u.matrix_world_to_projection,
                matrix_screen_to_world: u.matrix_screen_to_world,
                position_world: u.position_world.into(),
            };
            self.model_space.matrix_model_to_projection =
                self.camera_space.matrix_world_to_projection * self.matrix_model_to_world;
            self.needs_render = true;
        }
        if let Some(CameraUpdate { position_world, .. }) = self.light.on_event(event) {
            self.light_position = position_world.into();
            self.needs_render = true;
        };
        if self.shading_mode.on_event(event) {
            self.model_pipeline =
                create_model_pipeline(&self.device, &self.library, self.shading_mode);
            self.needs_render = true;
        }
        if self.depth_texture.on_event(event, &self.device) {
            self.needs_render = true;
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
    launch_application::<Delegate<true>>("Project 4 - Textures");
}
