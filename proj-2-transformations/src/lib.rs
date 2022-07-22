#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;
use metal_app::{components::Camera, metal::*, render_pipeline::*, *};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    path::PathBuf,
    simd::{f32x2, f32x4},
};

const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

struct Delegate {
    camera: Camera<4>,
    command_queue: CommandQueue,
    device: Device,
    model: Model<Geometry, NoMaterial>,
    needs_render: bool,
    render_pipeline: RenderPipeline<1, main_vertex, main_fragment, NoDepth, NoStencil>,
    vertex_input: VertexInput,
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let model = Model::from_file(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("common-assets")
                .join("teapot")
                .join("teapot.obj"),
            &device,
            |arg: &mut Geometry, geo| {
                arg.indices = geo.indices_buffer;
                arg.positions = geo.positions_buffer;
            },
            NoMaterial,
        );
        let &MaxBounds { center, size } = &model.geometry_max_bounds;
        let half_size = size * f32x4::splat(0.5);

        Self {
            camera: Camera::new(
                INITIAL_CAMERA_DISTANCE,
                f32x2::from_array([-PI / 6.0, 0.0]),
                ModifierKeys::empty(),
                false,
                0.,
            ),
            command_queue: device.new_command_queue(),
            model,
            needs_render: false,
            render_pipeline: {
                let library = device
                    .new_library_with_data(LIBRARY_BYTES)
                    .expect("Failed to import shader metal lib.");
                RenderPipeline::new(
                    "Render Pipeline",
                    &device,
                    &library,
                    [(MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend)],
                    main_vertex,
                    main_fragment,
                    NoDepth,
                    NoStencil,
                )
            },
            vertex_input: VertexInput {
                mins: (center - half_size).into(),
                maxs: (center + half_size).into(),
                use_perspective: true,
                screen_size: f32x2::default().into(),
                camera_rotation: f32x2::default().into(),
                camera_distance: 0.,
            },
            device,
        }
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.needs_render = false;
        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        command_buffer.set_label("Renderer Command Buffer");
        let encoder = self.render_pipeline.new_render_command_encoder(
            "Render Teapot",
            command_buffer,
            [(
                render_target,
                (0., 0., 0., 0.),
                MTLLoadAction::Clear,
                MTLStoreAction::Store,
            )],
            NoDepth,
            NoStencil,
        );
        for d in self.model.get_draws() {
            self.render_pipeline.setup_binds(
                encoder,
                main_vertex_binds {
                    r#in: BindOne::Bytes(&self.vertex_input),
                    geometry: BindOne::rolling_buffer_offset(d.geometry),
                },
                NoBinds,
            );
            encoder.draw_primitives(MTLPrimitiveType::Point, 0, d.num_vertices as _);
        }
        encoder.end_encoding();
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        if self.camera.on_event(event).is_some() {
            self.vertex_input.camera_distance = self.camera.ray.distance_from_origin;
            self.vertex_input.camera_rotation = self.camera.ray.rotation_xy.into();
            self.vertex_input.screen_size = self.camera.screen_size.into();
            self.needs_render = true;
        }
        match event {
            UserEvent::KeyDown { key_code, .. } => {
                // "P" Key Code
                if key_code == 35 {
                    // Toggle between orthographic and perspective
                    self.vertex_input.use_perspective = !self.vertex_input.use_perspective;
                    self.needs_render = true;
                }
            }
            _ => {}
        }
    }

    fn needs_render(&self) -> bool {
        self.needs_render
    }

    fn device(&self) -> &Device {
        &self.device
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 2 - Transformations");
}
