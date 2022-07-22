#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::components::{Camera, DepthTexture, ShadingModeSelector};
use metal_app::render_pipeline::{
    BindOne, BlendMode, HasDepth, NoBinds, NoStencil, RenderPipeline,
};
use metal_app::*;
use metal_app::{metal::*, metal_types::*};
use shader_bindings::*;
use std::ops::Neg;
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, f32x4},
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
    light_pipeline: RenderPipeline<1, light_vertex, light_fragment, HasDepth, NoStencil>,
    light_world_position: float4,
    matrix_model_to_world: f32x4x4,
    model: Model<Geometry, NoMaterial>,
    model_pipeline: RenderPipeline<1, main_vertex, main_fragment, HasDepth, NoStencil>,
    needs_render: bool,
    shading_mode: ShadingModeSelector,
}

fn create_model_pipeline(
    device: &Device,
    library: &Library,
    shading_mode: ShadingModeSelector,
) -> RenderPipeline<1, main_vertex, main_fragment, HasDepth, NoStencil> {
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
        HasDepth(DEPTH_TEXTURE_FORMAT),
        NoStencil,
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
                HasDepth(DEPTH_TEXTURE_FORMAT),
                NoStencil,
            ),
            light_world_position: f32x4::default().into(),
            matrix_model_to_world,
            model,
            model_pipeline: create_model_pipeline(&device, &library, shading_mode),
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
        {
            let encoder = self.model_pipeline.new_render_command_encoder(
                "Model and Light",
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
            );
            self.model.encode_use_resources(encoder);
            encoder.set_depth_stencil_state(&self.depth_state);

            // Render Teapot
            for DrawIteratorItem {
                num_vertices,
                geometry,
                ..
            } in self.model.get_draws()
            {
                // TODO: START HERE
                // TODO: START HERE
                // TODO: START HERE
                // Look at proj-4 for optimal binds (bind model, camera, light_pos once, outside of loop)
                self.model_pipeline.setup_binds(
                    encoder,
                    main_vertex_binds {
                        geometry: BindOne::buffer_with_rolling_offset(geometry),
                        model: BindOne::Bytes(&ModelSpace {
                            matrix_model_to_projection: (self
                                .camera_space
                                .matrix_world_to_projection
                                * self.matrix_model_to_world),
                            matrix_normal_to_world: self.matrix_model_to_world.into(),
                        }),
                    },
                    main_fragment_binds {
                        camera: BindOne::Bytes(&self.camera_space),
                        light_pos: BindOne::Bytes(&self.light_world_position),
                    },
                );
                encoder.draw_primitives(MTLPrimitiveType::Triangle, 0, num_vertices as _);
            }

            // Render Light
            encoder.set_render_pipeline_state(&self.light_pipeline.pipeline);
            self.light_pipeline.setup_binds(
                encoder,
                light_vertex_binds {
                    camera: BindOne::Bytes(&self.camera_space),
                    light_pos: BindOne::Bytes(&self.light_world_position),
                },
                NoBinds,
            );
            encoder.draw_primitives(MTLPrimitiveType::Point, 0, 1);
            encoder.end_encoding();
        }
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        if let Some(update) = self.camera.on_event(event) {
            self.camera_space = ProjectedSpace {
                matrix_world_to_projection: update.matrix_world_to_projection,
                matrix_screen_to_world: update.matrix_screen_to_world,
                position_world: update.position_world.into(),
            };
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
