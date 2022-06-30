#![feature(array_zip)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{components::camera, metal::*, metal_types::*, *};
use shader_bindings::*;
use std::{f32::consts::PI, path::PathBuf, simd::f32x2};

const INITIAL_TESSELATION_FACTOR: f32 = 64.;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([0., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 5., PI / 16.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

impl Default for Space {
    #[inline]
    fn default() -> Self {
        Self {
            matrix_world_to_projection: f32x4x4::identity(),
            matrix_screen_to_world: f32x4x4::identity(),
            position_world: float4 { xyzw: [0.; 4] },
        }
    }
}

impl From<camera::CameraUpdate> for Space {
    fn from(update: camera::CameraUpdate) -> Self {
        Space {
            matrix_world_to_projection: update.matrix_world_to_projection,
            matrix_screen_to_world: update.matrix_screen_to_world,
            position_world: update.position_world.into(),
        }
    }
}

struct Delegate {
    camera_space: Space,
    camera: camera::Camera,
    command_queue: CommandQueue,
    device: Device,
    light_space: Space,
    light: camera::Camera,
    needs_render: bool,
    normal_texture: Texture,
    render_pipeline_state: RenderPipelineState,
    tessellation_compute_state: ComputePipelineState,
    tessellation_factor: f32,
    tessellation_factors_buffer: Buffer,
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
        let mut image_buffer = vec![];
        Self {
            camera_space: Default::default(),
            camera: camera::Camera::new(INITIAL_CAMERA_ROTATION, ModifierKeys::empty(), false, 0.),
            command_queue: device.new_command_queue(),
            light_space: Default::default(),
            light: camera::Camera::new(INITIAL_LIGHT_ROTATION, ModifierKeys::CONTROL, true, 1.),
            needs_render: false,
            normal_texture: new_texture_from_png(
                assets_dir.join("teapot_normal.png"),
                &device,
                &mut image_buffer,
            ),
            render_pipeline_state: {
                let mut desc = new_render_pipeline_descriptor(
                    "Plane",
                    &library,
                    Some((DEFAULT_PIXEL_FORMAT, false)),
                    None,
                    None,
                    Some((&"main_vertex", VertexBufferIndex::LENGTH as _)),
                    Some((&"main_fragment", FragBufferIndex::LENGTH as _)),
                );
                set_tessellation_config(&mut desc);
                let p = create_render_pipeline(&device, &desc);
                debug_assert_argument_buffer_size::<{ VertexBufferIndex::CameraSpace as _ }, Space>(
                    &p,
                    FunctionType::Vertex,
                );
                debug_assert_argument_buffer_size::<{ FragBufferIndex::CameraSpace as _ }, Space>(
                    &p,
                    FunctionType::Fragment,
                );
                debug_assert_argument_buffer_size::<{ FragBufferIndex::LightSpace as _ }, Space>(
                    &p,
                    FunctionType::Fragment,
                );
                p.pipeline_state
            },
            tessellation_compute_state: {
                let fun = library
                    .get_function(&"tessell_compute", None)
                    .expect("Failed to get tessellation compute function");
                device
                    .new_compute_pipeline_state_with_function(&fun)
                    .expect("Failed to create tessellation compute pipeline")
            },
            tessellation_factor: INITIAL_TESSELATION_FACTOR,
            tessellation_factors_buffer: {
                // TODO: What is the exact size?
                // - 256 was copied from Apple Metal Sample Code: https://developer.apple.com/library/archive/samplecode/MetalBasicTessellation/Introduction/Intro.html
                let buf = device.new_buffer(256, MTLResourceOptions::StorageModePrivate);
                buf.set_label("Tessellation Factors");
                buf
            },
            device,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.needs_render = false;

        let command_buffer = self.command_queue.new_command_buffer();
        command_buffer.set_label("Command Buffer");

        // Compute Tesselation Factors
        {
            let encoder = command_buffer.new_compute_command_encoder();
            encoder.set_label("Compute Tesselation");
            encoder.push_debug_group("Compute Tesselation Factors");

            encoder.set_compute_pipeline_state(&self.tessellation_compute_state);
            encoder.set_bytes(
                TesselComputeBufferIndex::TessellFactor as _,
                std::mem::size_of_val(&self.tessellation_factor) as _,
                (&self.tessellation_factor as *const f32) as _,
            );
            encoder.set_buffer(
                TesselComputeBufferIndex::OutputTessellFactors as _,
                Some(&self.tessellation_factors_buffer),
                0,
            );
            let size_one = MTLSize {
                width: 1,
                height: 1,
                depth: 1,
            };
            encoder.dispatch_thread_groups(size_one, size_one);
            encoder.pop_debug_group();
            encoder.end_encoding();
        }
        // Render Plane
        {
            let encoder = command_buffer
                .new_render_command_encoder(new_render_pass_descriptor(Some(render_target), None));
            encoder.set_label("Render Plane");
            encoder.push_debug_group("Plane");
            encoder.set_render_pipeline_state(&self.render_pipeline_state);
            encode_vertex_bytes(
                encoder,
                VertexBufferIndex::CameraSpace as _,
                &self.camera_space,
            );
            encode_fragment_bytes::<Space>(
                encoder,
                FragBufferIndex::CameraSpace as _,
                &self.camera_space,
            );
            encode_fragment_bytes::<Space>(
                encoder,
                FragBufferIndex::LightSpace as _,
                &self.light_space,
            );
            encoder.set_fragment_texture(FragTextureIndex::Normal as _, Some(&self.normal_texture));
            // encoder.set_triangle_fill_mode(MTLTriangleFillMode::Lines);
            draw_patches_with_tesselation_factor_buffer(
                encoder,
                &self.tessellation_factors_buffer,
                4,
            );
            encoder.pop_debug_group();
            encoder.end_encoding();
        };
        command_buffer
    }

    #[inline]
    fn on_event(&mut self, event: UserEvent) {
        if let Some(update) = self.camera.on_event(event) {
            self.camera_space = update.into();
            self.needs_render = true;
        }
        if let Some(update) = self.light.on_event(event) {
            self.light_space = update.into();
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

impl Delegate {}

pub fn run() {
    launch_application::<Delegate>("Project 8 - Tesselation");
}
