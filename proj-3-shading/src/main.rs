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
    m_model_to_world: f32x4x4,
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
        [(DEFAULT_COLOR_FORMAT, BlendMode::NoBlend)],
        main_vertex,
        main_fragment {
            HasAmbient: shading_mode.contains(ShadingModeSelector::HAS_AMBIENT),
            HasDiffuse: shading_mode.contains(ShadingModeSelector::HAS_DIFFUSE),
            OnlyNormals: shading_mode.contains(ShadingModeSelector::ONLY_NORMALS),
            HasSpecular: shading_mode.contains(ShadingModeSelector::HAS_SPECULAR),
        },
        (Depth(DEFAULT_DEPTH_FORMAT), NoStencil),
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
                 ..
             }| {
                arg.indices = indices_buffer;
                arg.positions = positions_buffer;
                arg.normals = normals_buffer;
                arg.tx_coords = tx_coords_buffer;
            },
            NoMaterial,
        );
        let m_model_to_world = {
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
                m_world_to_projection: f32x4x4::identity(),
                m_screen_to_world: f32x4x4::identity(),
                position_world: f32x4::default().into(),
            },
            command_queue: device.new_command_queue(),
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: DepthTexture::new("Depth", DEFAULT_DEPTH_FORMAT),
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
                [(DEFAULT_COLOR_FORMAT, BlendMode::NoBlend)],
                light_vertex,
                light_fragment,
                (Depth(DEFAULT_DEPTH_FORMAT), NoStencil),
            ),
            light_world_position: f32x4::default().into(),
            m_model_to_world,
            model,
            model_pipeline: create_model_pipeline(&device, &library, shading_mode),
            model_space: ModelSpace {
                m_model_to_projection: f32x4x4::identity(),
                m_normal_to_world: m_model_to_world.into(),
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
        let depth_tx = self.depth_texture.texture();
        self.model_pipeline.new_pass(
            "Model and Light",
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
            &[&HeapUsage(
                &self.model.heap,
                MTLRenderStages::Vertex | MTLRenderStages::Fragment,
            )],
            |p| {
                p.bind(
                    main_vertex_binds {
                        model: Bind::Value(&self.model_space),
                        ..Binds::SKIP
                    },
                    main_fragment_binds {
                        camera: Bind::Value(&self.camera_space),
                        light_pos: Bind::Value(&self.light_world_position),
                    },
                );
                for draw in self.model.draws() {
                    p.draw_primitives_with_binds(
                        main_vertex_binds {
                            geometry: Bind::buffer_with_rolling_offset(draw.geometry),
                            ..Binds::SKIP
                        },
                        main_fragment_binds::SKIP,
                        MTLPrimitiveType::Triangle,
                        0,
                        draw.vertex_count,
                    );
                }
                p.into_subpass("Light", &self.light_pipeline, None, |p| {
                    p.draw_primitives_with_binds(
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
                m_world_to_projection: u.m_world_to_projection,
                m_screen_to_world: u.m_screen_to_world,
                position_world: u.position_world.into(),
            };
            self.model_space.m_model_to_projection =
                self.camera_space.m_world_to_projection * self.m_model_to_world;
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

fn main() {
    launch_application::<Delegate>("Project 3 - Shading");
}
