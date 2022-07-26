#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::components::{Camera, DepthTexture, ShadingModeSelector};
use metal_app::{metal::*, metal_types::*, pipeline::*, *};
use shader_bindings::*;
use std::ops::Neg;
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, f32x4, SimdFloat},
};

const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_DISTANCE: f32 = 1.;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 4., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_DISTANCE: f32 = INITIAL_CAMERA_DISTANCE / 2.;

struct Delegate {
    camera: Camera,
    camera_space: ProjectedSpace,
    command_queue: CommandQueue,
    depth_state: DepthStencilState,
    depth_texture: DepthTexture,
    device: Device,
    library: Library,
    light: Camera,
    light_pipeline: RenderPipeline<1, light_vertex, light_fragment, (Depth, NoStencil)>,
    light_world_position: float4,
    matrix_model_to_world: f32x4x4,
    model: Model<Geometry, NoMaterial>,
    model_pipeline: RenderPipeline<1, main_vertex, main_fragment, (Depth, NoStencil)>,
    model_space: ModelSpace,
    needs_render: bool,
    shading_mode: ShadingModeSelector,
}

fn create_model_pipeline(
    device: &Device,
    library: &Library,
    shading_mode: ShadingModeSelector,
) -> RenderPipeline<1, main_vertex, main_fragment, (Depth, NoStencil)> {
    RenderPipeline::new(
        "Render Teapot Pipeline",
        device,
        library,
        [(DEFAULT_PIXEL_FORMAT, BlendMode::NoBlend)],
        main_vertex,
        main_fragment {
            HasAmbient: shading_mode.contains(ShadingModeSelector::HAS_AMBIENT),
            HasDiffuse: shading_mode.contains(ShadingModeSelector::HAS_DIFFUSE),
            OnlyNormals: shading_mode.contains(ShadingModeSelector::ONLY_NORMALS),
            HasSpecular: shading_mode.contains(ShadingModeSelector::HAS_SPECULAR),
        },
        (Depth(DEPTH_TEXTURE_FORMAT), NoStencil),
    )
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let teapot_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("common-assets")
            .join("teapot")
            .join("teapot.obj");
        let model = Model::from_file(
            teapot_file,
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
            NoMaterial,
        );
        let matrix_model_to_world = {
            let MaxBounds { center, size } = model.geometry_max_bounds;
            let [cx, cy, cz, _] = center.neg().to_array();
            let scale = 1. / size.reduce_max();
            f32x4x4::scale(scale, scale, scale, 1.)
                * f32x4x4::x_rotate(PI / 2.)
                * f32x4x4::translate(cx, cy, cz)
        };

        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        let shading_mode = ShadingModeSelector::DEFAULT;

        // Setup Render Pipeline Descriptor used for rendering the teapot and light
        Self {
            camera: Camera::new(
                INITIAL_CAMERA_DISTANCE,
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
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: DepthTexture::new("Depth", DEPTH_TEXTURE_FORMAT),
            light: Camera::new(
                LIGHT_DISTANCE,
                INITIAL_LIGHT_ROTATION,
                ModifierKeys::CONTROL,
                true,
                0.,
            ),
            light_pipeline: RenderPipeline::new(
                "Render Light Pipeline",
                &device,
                &library,
                [(DEFAULT_PIXEL_FORMAT, BlendMode::NoBlend)],
                light_vertex,
                light_fragment,
                (Depth(DEPTH_TEXTURE_FORMAT), NoStencil),
            ),
            light_world_position: f32x4::default().into(),
            matrix_model_to_world,
            model,
            model_pipeline: create_model_pipeline(&device, &library, shading_mode),
            model_space: ModelSpace {
                matrix_model_to_projection: f32x4x4::identity(),
                matrix_normal_to_world: matrix_model_to_world.into(),
            },
            needs_render: false,
            shading_mode,
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
                            camera: Bind::Value(&self.camera_space),
                            light_pos: Bind::Value(&self.light_world_position),
                        },
                    );
                    for DrawItemNoMaterial {
                        vertex_count,
                        geometry,
                        ..
                    } in self.model.draws()
                    {
                        p.draw_primitives_with_bind(
                            main_vertex_binds {
                                geometry: Bind::Buffer(BindBuffer::buffer_with_rolling_offset(
                                    geometry,
                                )),
                                model: Bind::Skip,
                            },
                            main_fragment_binds {
                                camera: Bind::Skip,
                                light_pos: Bind::Skip,
                            },
                            MTLPrimitiveType::Triangle,
                            0,
                            vertex_count as _,
                        );
                    }
                });
                p.into_subpass("Light", &self.light_pipeline, None, |p| {
                    p.draw_primitives_with_bind(
                        light_vertex_binds {
                            camera: Bind::Value(&self.camera_space),
                            light_pos: Bind::Value(&self.light_world_position),
                        },
                        NoBinds,
                        MTLPrimitiveType::Point,
                        0,
                        1,
                    )
                });
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
        if let Some(update) = self.light.on_event(event) {
            self.light_world_position = update.position_world.into();
            self.needs_render = true;
        }
        if self.shading_mode.on_event(event) {
            self.model_pipeline =
                create_model_pipeline(&self.device, &self.library, self.shading_mode);
            self.needs_render = true;
        }
        if self.depth_texture.on_event(event, &self.device) {
            self.needs_render = true;
        }
    }

    #[inline]
    fn needs_render(&self) -> bool {
        self.needs_render
    }

    fn device(&self) -> &Device {
        &self.device
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 3 - Shading");
}
