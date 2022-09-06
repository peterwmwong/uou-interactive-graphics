#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::components::{Camera, ScreenSizeTexture, ShadingModeSelector};
use metal_app::{metal::*, metal_types::*, pipeline::*, *};
use shader_bindings::*;
use std::ops::Neg;
use std::simd::f32x4;
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, SimdFloat},
};

type GBufPipelineType = RenderPipeline<3, gbuf_vertex, gbuf_fragment, (Depth, Stencil)>;
type LightingPipelineType = RenderPipeline<3, lighting_vertex, lighting_fragment, (Depth, Stencil)>;
type DrawLightPipelineType = RenderPipeline<3, light_vertex, light_fragment, (Depth, Stencil)>;

const PIPELINE_COLORS: [(MTLPixelFormat, BlendMode); 3] = [
    (DEFAULT_COLOR_FORMAT, BlendMode::NoBlend),
    (GBUF_NORMAL_PIXEL_FORMAT, BlendMode::NoBlend),
    (GBUF_DEPTH_PIXEL_FORMAT, BlendMode::NoBlend),
];

const GBUF_NORMAL_PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::RGBA16Snorm;
const GBUF_DEPTH_PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::R16Float;

const INITIAL_CAMERA_DISTANCE: f32 = 1.;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([0., -PI / 3.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([0., PI / 2.1]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_DISTANCE: f32 = INITIAL_CAMERA_DISTANCE / 2.;

const STENCIL_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Stencil8;
const STENCIL_VALUE_CLEAR: u32 = 0;
const STENCIL_VALUE_GBUFFER: u32 = 128;

struct Delegate {
    camera: Camera,
    command_queue: CommandQueue,
    depth_write_less_stencil_write_always: DepthStencilState,
    depth_keep_always_stencil_keep_equal: DepthStencilState,
    depth_keep_less_stencil_keep_always: DepthStencilState,
    depth: ScreenSizeTexture,
    device: Device,
    gbuf_depth: ScreenSizeTexture,
    gbuf_normal: ScreenSizeTexture,
    gbuf_pipeline: GBufPipelineType,
    library: Library,
    light_pipeline: DrawLightPipelineType,
    light: Camera,
    lighting_pipeline: LightingPipelineType,
    m_model_to_world: f32x4x4,
    model_space: ModelSpace2,
    model: Model<GeometryNoTxCoords, NoMaterial>,
    needs_render: bool,
    shading_mode: ShadingModeSelector,
    stencil: ScreenSizeTexture,
}

fn create_gbuf_pipeline(device: &Device, library: &Library) -> GBufPipelineType {
    RenderPipeline::new(
        "GBuf",
        device,
        library,
        PIPELINE_COLORS,
        gbuf_vertex,
        gbuf_fragment,
        (Depth(DEFAULT_DEPTH_FORMAT), Stencil(STENCIL_TEXTURE_FORMAT)),
    )
}

fn create_lighting_pipeline(
    device: &Device,
    library: &Library,
    shading_mode: ShadingModeSelector,
) -> LightingPipelineType {
    RenderPipeline::new(
        "Lighting",
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
        (Depth(DEFAULT_DEPTH_FORMAT), Stencil(STENCIL_TEXTURE_FORMAT)),
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
        let ds = DepthStencilDescriptor::new();
        let s = StencilDescriptor::new();
        Self {
            camera: Camera::new(
                INITIAL_CAMERA_DISTANCE,
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            command_queue: device.new_command_queue(),
            depth: ScreenSizeTexture::new_depth(),
            depth_write_less_stencil_write_always: {
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
            depth_keep_always_stencil_keep_equal: {
                ds.set_depth_write_enabled(false);
                ds.set_depth_compare_function(MTLCompareFunction::Always);
                {
                    s.set_stencil_compare_function(MTLCompareFunction::Equal);
                    s.set_depth_stencil_pass_operation(MTLStencilOperation::Keep);
                    ds.set_front_face_stencil(Some(&s));
                    ds.set_back_face_stencil(Some(&s));
                }
                device.new_depth_stencil_state(&ds)
            },
            depth_keep_less_stencil_keep_always: {
                ds.set_depth_write_enabled(false);
                ds.set_depth_compare_function(MTLCompareFunction::Less);
                {
                    s.set_stencil_compare_function(MTLCompareFunction::Always);
                    s.set_depth_stencil_pass_operation(MTLStencilOperation::Keep);
                    ds.set_front_face_stencil(Some(&s));
                    ds.set_back_face_stencil(Some(&s));
                }
                device.new_depth_stencil_state(&ds)
            },
            gbuf_depth: ScreenSizeTexture::new_memoryless_render_target(
                "gbuf_depth",
                GBUF_DEPTH_PIXEL_FORMAT,
            ),
            gbuf_normal: ScreenSizeTexture::new_memoryless_render_target(
                "gbuf_normal",
                GBUF_NORMAL_PIXEL_FORMAT,
            ),
            gbuf_pipeline: create_gbuf_pipeline(&device, &library),
            light: Camera::new(
                LIGHT_DISTANCE,
                INITIAL_LIGHT_ROTATION,
                ModifierKeys::CONTROL,
                true,
                0.,
            ),
            light_pipeline: RenderPipeline::new(
                "Draw Light",
                &device,
                &library,
                PIPELINE_COLORS,
                light_vertex,
                light_fragment,
                (Depth(DEFAULT_DEPTH_FORMAT), Stencil(STENCIL_TEXTURE_FORMAT)),
            ),
            lighting_pipeline: create_lighting_pipeline(&device, &library, shading_mode),
            m_model_to_world,
            model,
            model_space: ModelSpace2 {
                m_model_to_projection: f32x4x4::identity(),
                m_model_to_camera: f32x4x4::identity(),
            },
            needs_render: false,
            shading_mode,
            stencil: ScreenSizeTexture::new(
                "Stencil",
                STENCIL_TEXTURE_FORMAT,
                MTLResourceOptions::StorageModeMemoryless
                    | MTLResourceOptions::HazardTrackingModeUntracked,
                MTLTextureUsage::RenderTarget,
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
            (
                self.stencil.texture(),
                STENCIL_VALUE_CLEAR,
                MTLLoadAction::Clear,
                MTLStoreAction::DontCare,
            ),
            (
                &self.depth_write_less_stencil_write_always,
                STENCIL_VALUE_GBUFFER,
                STENCIL_VALUE_GBUFFER,
            ),
            MTLCullMode::Back,
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
                        },
                        NoBinds,
                        MTLPrimitiveType::Triangle,
                        0,
                        draw.vertex_count,
                    );
                }
                p.into_subpass(
                    "Lighting",
                    &self.lighting_pipeline,
                    Some((
                        &self.depth_keep_always_stencil_keep_equal,
                        STENCIL_VALUE_GBUFFER,
                        STENCIL_VALUE_GBUFFER,
                    )),
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
                            MTLPrimitiveType::Triangle,
                            0,
                            3,
                        );
                        p.into_subpass(
                            "Draw Light",
                            &self.light_pipeline,
                            Some((
                                &self.depth_keep_less_stencil_keep_always,
                                STENCIL_VALUE_GBUFFER,
                                STENCIL_VALUE_GBUFFER,
                            )),
                            None,
                            |p| {
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
                            },
                        );
                    },
                );
            },
        );
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        if self.camera.on_event(event) {
            self.model_space = ModelSpace2 {
                m_model_to_projection: self.camera.projected_space.m_world_to_projection
                    * self.m_model_to_world,
                m_model_to_camera: self.camera.get_world_to_camera_transform()
                    * self.m_model_to_world,
            };
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
        if self.stencil.on_event(event, &self.device) {
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
