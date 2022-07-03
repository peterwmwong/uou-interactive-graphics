#![feature(array_zip)]
#![feature(portable_simd)]
#![feature(slice_as_chunks)]
mod shader_bindings;

use metal_app::components::{Camera, ShadingModeSelector};
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
    depth_texture: Option<Texture>,
    device: Device,
    library: Library,
    light: Camera,
    light_pipeline: RenderPipelineState,
    light_world_position: float4,
    matrix_model_to_world: f32x4x4,
    model: Model<{ VertexBufferIndex::Geometry as _ }, NO_MATERIALS_ID>,
    model_pipeline: RenderPipelineState,
    needs_render: bool,
    shading_mode: ShadingModeSelector,
}

fn create_model_pipeline(
    device: &Device,
    library: &Library,
    shading_mode: ShadingModeSelector,
) -> RenderPipelineState {
    let p = create_render_pipeline(
        &device,
        &new_render_pipeline_descriptor(
            "Render Teapot Pipeline",
            &library,
            Some((DEFAULT_PIXEL_FORMAT, false)),
            Some(DEPTH_TEXTURE_FORMAT),
            Some(&shading_mode.encode(
                FunctionConstantValues::new(),
                ShadingMode::HasAmbient as _,
                ShadingMode::HasDiffuse as _,
                ShadingMode::HasSpecular as _,
                ShadingMode::OnlyNormals as _,
            )),
            Some((&"main_vertex", VertexBufferIndex::LENGTH as _)),
            Some((&"main_fragment", FragBufferIndex::LENGTH as _)),
        ),
    );
    use debug_assert_pipeline_function_arguments::*;
    debug_assert_render_pipeline_function_arguments(
        &p,
        &[
            value_arg::<Geometry>(VertexBufferIndex::Geometry as _),
            value_arg::<ModelSpace>(VertexBufferIndex::ModelSpace as _),
        ],
        Some(&[
            value_arg::<ProjectedSpace>(FragBufferIndex::CameraSpace as _),
            value_arg::<float4>(FragBufferIndex::LightPosition as _),
        ]),
    );
    p.pipeline_state
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
            NO_MATERIALS_ENCODER,
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
            depth_texture: None,
            light: Camera::new(
                LIGHT_DISTANCE,
                INITIAL_LIGHT_ROTATION,
                ModifierKeys::CONTROL,
                true,
                0.,
            ),
            light_pipeline: create_render_pipeline(
                &device,
                &new_render_pipeline_descriptor(
                    "Render Light Pipeline",
                    &library,
                    Some((DEFAULT_PIXEL_FORMAT, false)),
                    Some(DEPTH_TEXTURE_FORMAT),
                    None,
                    Some((&"light_vertex", LightVertexBufferIndex::LENGTH as _)),
                    Some((&"light_fragment", 0)),
                ),
            )
            .pipeline_state,
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
        let encoder = command_buffer.new_render_command_encoder(new_render_pass_descriptor(
            Some(render_target),
            self.depth_texture
                .as_ref()
                .map(|d| (d, MTLStoreAction::DontCare)),
        ));
        encoder.set_label("Model and Light");

        self.model.encode_use_resources(encoder);
        encoder.set_render_pipeline_state(&self.model_pipeline);
        encoder.set_depth_stencil_state(&self.depth_state);

        // Render Teapot
        {
            let matrix_normal_to_world: float3x3 = self.matrix_model_to_world.into();
            encode_vertex_bytes(
                &encoder,
                VertexBufferIndex::ModelSpace as _,
                &ModelSpace {
                    matrix_model_to_projection: (self.camera_space.matrix_world_to_projection
                        * self.matrix_model_to_world),
                    matrix_normal_to_world,
                },
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex::CameraSpace as _,
                &self.camera_space,
            );
            encode_fragment_bytes(
                &encoder,
                FragBufferIndex::LightPosition as _,
                &self.light_world_position,
            );
            self.model.encode_draws(encoder);
        }

        // Render Light
        {
            encoder.set_render_pipeline_state(&self.light_pipeline);
            encode_vertex_bytes(
                &encoder,
                LightVertexBufferIndex::CameraSpace as _,
                &self.camera_space,
            );
            encode_vertex_bytes(
                &encoder,
                LightVertexBufferIndex::LightPosition as _,
                &self.light_world_position,
            );
            encoder.draw_primitives(MTLPrimitiveType::Point, 0, 1);
        }
        encoder.end_encoding();
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
        match event {
            UserEvent::WindowFocusedOrResized { size, .. } => {
                let desc = TextureDescriptor::new();
                desc.set_width(size[0] as _);
                desc.set_height(size[1] as _);
                desc.set_pixel_format(DEPTH_TEXTURE_FORMAT);
                desc.set_storage_mode(MTLStorageMode::Memoryless);
                desc.set_usage(MTLTextureUsage::RenderTarget);
                self.depth_texture = Some(self.device.new_texture(&desc));
                self.needs_render = true;
            }
            _ => {}
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
