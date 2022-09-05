#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::components::{Camera, DepthTexture, ScreenSizeTexture, ShadingModeSelector};
use metal_app::{metal::*, metal_types::*, pipeline::*, *};
use shader_bindings::*;
use std::ops::Neg;
use std::simd::f32x4;
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, SimdFloat},
};

type GBufPipelineType = RenderPipeline<3, gbuf_vertex, gbuf_fragment, (Depth, NoStencil)>;
type LightingPipelineType =
    RenderPipeline<3, lighting_vertex, lighting_fragment, (Depth, NoStencil)>;
type DrawLightPipelineType = RenderPipeline<3, light_vertex, light_fragment, (Depth, NoStencil)>;

const PIPELINE_COLORS: [(MTLPixelFormat, BlendMode); 3] = [
    (DEFAULT_COLOR_FORMAT, BlendMode::NoBlend),
    (GBUF_NORMAL_PIXEL_FORMAT, BlendMode::NoBlend),
    (GBUF_DEPTH_PIXEL_FORMAT, BlendMode::NoBlend),
];

const GBUF_NORMAL_PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::RGBA16Snorm;
const GBUF_DEPTH_PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::R32Float;

// const INITIAL_CAMERA_DISTANCE: f32 = 0.868125;
const INITIAL_CAMERA_DISTANCE: f32 = 1.;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([0., -PI / 3.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([0., PI / 2.1]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_DISTANCE: f32 = INITIAL_CAMERA_DISTANCE / 2.;

struct Delegate {
    camera: Camera,
    command_queue: CommandQueue,
    depth_state: DepthStencilState,
    depth_state_nowrite_allow_less: DepthStencilState,
    depth: DepthTexture,
    device: Device,
    gbuf_depth: ScreenSizeTexture,
    gbuf_normal: ScreenSizeTexture,
    gbuf_pipeline: GBufPipelineType,
    library: Library,
    light: Camera,
    light_pipeline: DrawLightPipelineType,
    lighting_pipeline: LightingPipelineType,
    m_model_to_world: f32x4x4,
    model: Model<GeometryNoTxCoords, NoMaterial>,
    model_space: ModelSpace,
    needs_render: bool,
    shading_mode: ShadingModeSelector,
}

fn create_gbuf_pipeline(device: &Device, library: &Library) -> GBufPipelineType {
    RenderPipeline::new(
        "GBuf Pipeline",
        device,
        library,
        PIPELINE_COLORS,
        gbuf_vertex,
        gbuf_fragment,
        (Depth(DEFAULT_DEPTH_FORMAT), NoStencil),
    )
}

fn create_lighting_pipeline(
    device: &Device,
    library: &Library,
    shading_mode: ShadingModeSelector,
) -> LightingPipelineType {
    RenderPipeline::new(
        "Lighting Pipeline",
        device,
        library,
        PIPELINE_COLORS,
        lighting_vertex,
        lighting_fragment {
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
        let teapot_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("common-assets")
            .join("plane")
            .join("plane.obj");
        let model = Model::from_file(
            teapot_file,
            &device,
            |arg: &mut GeometryNoTxCoords,
             GeometryToEncode {
                 indices_buffer,
                 positions_buffer,
                 normals_buffer,
                 ..
             }| {
                arg.indices = indices_buffer;
                arg.positions = positions_buffer;
                arg.normals = normals_buffer;
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
        let desc = DepthStencilDescriptor::new();
        desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
        Self {
            camera: Camera::new(
                INITIAL_CAMERA_DISTANCE,
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            command_queue: device.new_command_queue(),
            depth_state: {
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_state_nowrite_allow_less: {
                desc.set_depth_write_enabled(false);
                device.new_depth_stencil_state(&desc)
            },
            depth: DepthTexture::new("Depth", DEFAULT_DEPTH_FORMAT),
            gbuf_normal: ScreenSizeTexture::new_memoryless_render_target(
                "gbuf_normal",
                GBUF_NORMAL_PIXEL_FORMAT,
            ),
            gbuf_depth: ScreenSizeTexture::new_memoryless_render_target(
                "gbuf_depth",
                GBUF_DEPTH_PIXEL_FORMAT,
            ),
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
                PIPELINE_COLORS,
                light_vertex,
                light_fragment,
                (Depth(DEFAULT_DEPTH_FORMAT), NoStencil),
            ),
            m_model_to_world,
            model,
            gbuf_pipeline: create_gbuf_pipeline(&device, &library),
            lighting_pipeline: create_lighting_pipeline(&device, &library, shading_mode),
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
        let depth_tx = self.depth.texture();
        self.gbuf_pipeline.new_pass(
            "GBuf",
            command_buffer,
            [
                (
                    render_target,
                    (0., 0., 0., 0.),
                    MTLLoadAction::Clear,
                    MTLStoreAction::Store,
                ),
                (
                    self.gbuf_normal.texture(),
                    (0., 0., 0., 0.),
                    MTLLoadAction::Clear,
                    MTLStoreAction::DontCare,
                ),
                (
                    self.gbuf_depth.texture(),
                    (0., 0., 0., 0.),
                    MTLLoadAction::Clear,
                    MTLStoreAction::DontCare,
                ),
            ],
            (depth_tx, 1., MTLLoadAction::Clear, MTLStoreAction::DontCare),
            NoStencil,
            &self.depth_state,
            MTLCullMode::None,
            &[&HeapUsage(
                &self.model.heap,
                MTLRenderStages::Vertex | MTLRenderStages::Fragment,
            )],
            |p| {
                p.bind(
                    gbuf_vertex_binds {
                        model: Bind::Value(&self.model_space),
                        ..Binds::SKIP
                    },
                    NoBinds,
                );
                for draw in self.model.draws() {
                    p.draw_primitives_with_binds(
                        gbuf_vertex_binds {
                            geometry: Bind::buffer_with_rolling_offset(draw.geometry),
                            model: Bind::Skip,
                            m_model_to_world: Bind::Value(&self.m_model_to_world),
                            m_world_to_camera: Bind::Value(
                                &self.camera.get_world_to_camera_transform(),
                            ),
                        },
                        NoBinds,
                        MTLPrimitiveType::Triangle,
                        0,
                        draw.vertex_count,
                    );
                }
                // TODO: Need a depth/stencil strategy so lighting does NOT run on every pixel (ex. bg)
                // - Look at the Apple Deferred Lighting sample.
                p.into_subpass(
                    "Lighting",
                    &self.lighting_pipeline,
                    Some(&self.depth_state_nowrite_allow_less),
                    None,
                    |p| {
                        let light_pos_in_camera_space = self.camera.get_world_to_camera_transform()
                            * f32x4::from(self.light.projected_space.position_world);
                        p.draw_primitives_with_binds(
                            lighting_vertex_binds {
                                m_projection_to_camera: Bind::Value(
                                    &self.camera.m_projection_to_camera,
                                ),
                            },
                            lighting_fragment_binds {
                                camera: Bind::Value(&self.camera.projected_space),
                                light_pos_cam: Bind::Value(&light_pos_in_camera_space.into()),
                            },
                            MTLPrimitiveType::TriangleStrip,
                            0,
                            4,
                        );
                        p.into_subpass("Draw Light", &self.light_pipeline, None, None, |p| {
                            p.draw_primitives_with_binds(
                                light_vertex_binds {
                                    camera: Bind::Value(&self.camera.projected_space),
                                    light_pos: Bind::Value(
                                        &self.light.projected_space.position_world,
                                    ),
                                },
                                NoBinds,
                                MTLPrimitiveType::Point,
                                0,
                                1,
                            )
                        });
                    },
                );
            },
        );
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        if self.camera.on_event(event) {
            self.model_space.m_model_to_projection =
                self.camera.projected_space.m_world_to_projection * self.m_model_to_world;
            self.needs_render = true;
        }
        if self.light.on_event(event) {
            self.needs_render = true;
        }
        if self.gbuf_normal.on_event(event, &self.device) {
            self.needs_render = true;
        }
        if self.gbuf_depth.on_event(event, &self.device) {
            self.needs_render = true;
        }
        if self.shading_mode.on_event(event) {
            self.lighting_pipeline =
                create_lighting_pipeline(&self.device, &self.library, self.shading_mode);
            self.needs_render = true;
        }
        if self.depth.on_event(event, &self.device) {
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
