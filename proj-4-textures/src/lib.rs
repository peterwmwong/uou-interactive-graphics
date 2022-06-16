#![feature(portable_simd)]
mod shader_bindings;

use bitflags::bitflags;
use metal_app::{metal::*, *};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    ops::Neg,
    path::PathBuf,
    simd::{f32x2, f32x4},
};

bitflags! {
    struct Mode: u8 {
        const HAS_AMBIENT = 1 << FC::HasAmbient as u8;
        const HAS_DIFFUSE = 1 << FC::HasDiffuse as u8;
        const HAS_NORMAL = 1 << FC::HasNormal as u8;
        const HAS_SPECULAR = 1 << FC::HasSpecular as u8;
        const DEFAULT = Self::HAS_AMBIENT.bits | Self::HAS_DIFFUSE.bits | Self::HAS_SPECULAR.bits;
    }
}

const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_DISTANCE: f32 = 1.;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 4., 0.]);
const INITIAL_MODE: Mode = Mode::DEFAULT;
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_DISTANCE: f32 = INITIAL_CAMERA_DISTANCE / 2.;

const N: f32 = 0.1;
const F: f32 = 100000.0;
const NEAR_FIELD_MAJOR_AXIS: f32 = N / INITIAL_CAMERA_DISTANCE;
const PERSPECTIVE_MATRIX: f32x4x4 = f32x4x4::new(
    [N, 0., 0., 0.],
    [0., N, 0., 0.],
    [0., 0., N + F, -N * F],
    [0., 0., 1., 0.],
);

struct Delegate {
    camera_distance: f32,
    camera_rotation: f32x2,
    depth_state: DepthStencilState,
    depth_texture: Option<Texture>,
    device: Device,
    library: Library,
    light_xy_rotation: f32x2,
    matrix_model_to_world: f32x4x4,
    mode: Mode,
    model: Model,
    render_light_pipeline_state: RenderPipelineState,
    render_pipeline_state: RenderPipelineState,
    screen_size: f32x2,
    world_arg_buffer: Buffer,
    world_arg_encoder: ArgumentEncoder,
    needs_render: bool,
}

struct PipelineResults {
    model: CreateRenderPipelineResults,
    light: CreateRenderPipelineResults,
}

fn create_pipelines(device: &Device, library: &Library, mode: Mode) -> PipelineResults {
    let base_pipeline_desc = RenderPipelineDescriptor::new();
    base_pipeline_desc.set_depth_attachment_pixel_format(DEPTH_TEXTURE_FORMAT);
    {
        let desc = unwrap_option_dcheck(
            base_pipeline_desc.color_attachments().object_at(0 as u64),
            "Failed to access color attachment on pipeline descriptor",
        );
        desc.set_blending_enabled(false);
        desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
    }

    let function_constants = FunctionConstantValues::new();
    for index in [
        FC::HasAmbient as usize,
        FC::HasDiffuse as usize,
        FC::HasSpecular as usize,
        FC::HasNormal as usize,
    ] {
        function_constants.set_constant_value_at_index(
            (&mode.contains(Mode::from_bits_truncate(1 << index)) as *const _) as _,
            MTLDataType::Bool,
            index as _,
        );
    }
    PipelineResults {
        model: create_pipeline(
            &device,
            &library,
            &base_pipeline_desc,
            "Model",
            Some(&function_constants),
            &"main_vertex",
            VertexBufferIndex::LENGTH as _,
            &"main_fragment",
            FragBufferIndex::LENGTH as _,
        ),
        light: create_pipeline(
            &device,
            &library,
            &base_pipeline_desc,
            "Light",
            Some(&function_constants),
            &"light_vertex",
            LightVertexBufferIndex::LENGTH as _,
            &"light_fragment",
            0,
        ),
    }
}

impl RendererDelgate for Delegate {
    fn new(device: Device, _command_queue: &CommandQueue) -> Self {
        let model_file_path = std::env::args()
            .skip(1)
            .nth(0)
            .expect("Usage: proj-4-textures [Path to Wavefront OBJ file]");
        let model_file = PathBuf::from(model_file_path);
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");
        let mode = INITIAL_MODE;
        let pipelines = create_pipelines(&device, &library, mode);
        let model = Model::from_file::<
            PathBuf,
            { GeometryID::Indices as _ },
            { GeometryID::Positions as _ },
            { GeometryID::Normals as _ },
            { GeometryID::TXCoords as _ },
            { MaterialID::AmbientTexture as _ },
            { MaterialID::DiffuseTexture as _ },
            { MaterialID::SpecularTexture as _ },
            { MaterialID::SpecularShineness as _ },
        >(
            model_file,
            &device,
            &pipelines
                .model
                .vertex_function
                .new_argument_encoder(VertexBufferIndex::Geometry as _),
            &pipelines
                .model
                .fragment_function
                .new_argument_encoder(FragBufferIndex::Material as _),
        );

        let world_arg_encoder = pipelines
            .model
            .fragment_function
            .new_argument_encoder(FragBufferIndex::World as _);
        let world_arg_buffer = device.new_buffer(
            world_arg_encoder.encoded_length(),
            MTLResourceOptions::CPUCacheModeWriteCombined | MTLResourceOptions::StorageModeShared,
        );
        world_arg_buffer.set_label("World Argument Buffer");
        world_arg_encoder.set_argument_buffer(&world_arg_buffer, 0);

        let MaxBounds { center, size } = &model.geometry_max_bounds;
        let &[cx, cy, cz, _] = center.neg().as_array();

        // IMPORTANT: Normalize the world coordinates to a reasonable range ~[0, 1].
        // 1. INITIAL_CAMERA_DISTANCE is invariant of the model's coordinate range
        // 2. Dramatically reduces precision errors (compared to ranges >1000, like in Yoda model)
        //    - In the Vertex Shader, z-fighting in the depth buffer, even with Depth32Float.
        //    - In the Fragment Shader, diffuse and specular lighting is no longer smooth and
        //      exhibit a weird triangal-ish pattern.
        let scale = 1. / size.reduce_max();
        let matrix_model_to_world = f32x4x4::scale(scale, scale, scale, 1.)
            * (f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.))
            * f32x4x4::translate(cx, cy, cz);

        let mut delegate = Self {
            camera_distance: INITIAL_CAMERA_DISTANCE,
            camera_rotation: INITIAL_CAMERA_ROTATION,
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: None,
            library,
            light_xy_rotation: INITIAL_LIGHT_ROTATION,
            matrix_model_to_world,
            mode,
            model,
            render_pipeline_state: pipelines.model.pipeline_state,
            render_light_pipeline_state: pipelines.light.pipeline_state,
            screen_size: f32x2::default(),
            world_arg_buffer,
            world_arg_encoder,
            needs_render: false,
            device,
        };

        // IMPORTANT: Not a mistake, using Model-to-World Rotation 4x4 Matrix for
        // Normal-to-World 3x3 Matrix. Conceptually, we want a matrix that ONLY applies rotation
        // (no translation). Since normals are directions (not positions, relative to a
        // point on a surface), translations are meaningless.
        delegate.update_world(
            WorldID::MatrixNormalToWorld,
            matrix_model_to_world.metal_float3x3_upper_left(),
        );
        delegate.update_light(delegate.light_xy_rotation);
        delegate.update_camera(
            delegate.screen_size,
            delegate.camera_rotation,
            delegate.camera_distance,
        );
        delegate.reset_needs_render();
        delegate
    }

    #[inline]
    fn render<'a>(
        &mut self,
        command_queue: &'a CommandQueue,
        render_target: &TextureRef,
    ) -> &'a CommandBufferRef {
        self.reset_needs_render();
        let command_buffer = command_queue.new_command_buffer_with_unretained_references();
        command_buffer.set_label("Renderer Command Buffer");
        let encoder = command_buffer.new_render_command_encoder({
            let desc = RenderPassDescriptor::new();
            {
                let a = unwrap_option_dcheck(
                    desc.color_attachments().object_at(0),
                    "Failed to access color attachment on render pass descriptor",
                );
                a.set_texture(Some(render_target));
                a.set_load_action(MTLLoadAction::Clear);
                a.set_clear_color(MTLClearColor::new(0.0, 0.0, 0.0, 0.0));
                a.set_store_action(MTLStoreAction::Store);
            }
            {
                let a = desc.depth_attachment().unwrap();
                a.set_clear_depth(1.);
                a.set_load_action(MTLLoadAction::Clear);
                a.set_store_action(MTLStoreAction::DontCare);
                a.set_texture(self.depth_texture.as_deref());
            }
            desc
        });
        // Render Model
        {
            encoder.push_debug_group("Model");
            encoder.set_render_pipeline_state(&self.render_pipeline_state);
            encoder.set_depth_stencil_state(&self.depth_state);
            self.model.encode_use_resources(&encoder);
            encoder.set_vertex_buffer(
                VertexBufferIndex::World as _,
                Some(&self.world_arg_buffer),
                0,
            );
            encoder.set_fragment_buffer(
                FragBufferIndex::World as _,
                Some(&self.world_arg_buffer),
                0,
            );
            self.model.encode_draws(
                &encoder,
                VertexBufferIndex::Geometry as _,
                FragBufferIndex::Material as _,
            );
            encoder.pop_debug_group();
        }
        // Render Light
        {
            encoder.push_debug_group("Light");
            encoder.set_render_pipeline_state(&self.render_light_pipeline_state);
            // // TODO: Figure out a better way to unset this buffers from the previous draw call
            encoder.set_vertex_buffers(
                0,
                &[None; VertexBufferIndex::LENGTH as _],
                &[0; VertexBufferIndex::LENGTH as _],
            );
            encoder.set_fragment_buffers(
                0,
                &[None; FragBufferIndex::LENGTH as _],
                &[0; FragBufferIndex::LENGTH as _],
            );
            encoder.set_vertex_buffer(
                LightVertexBufferIndex::World as _,
                Some(&self.world_arg_buffer),
                0,
            );
            encoder.draw_primitives(MTLPrimitiveType::Point, 0, 1);
            encoder.pop_debug_group();
        }
        encoder.end_encoding();
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        use MouseButton::*;
        use UserEvent::*;
        match event {
            MouseDrag {
                button,
                modifier_keys,
                drag_amount,
                ..
            } => {
                if modifier_keys.is_empty() {
                    let mut camera_rotation = self.camera_rotation;
                    let mut camera_distance = self.camera_distance;
                    match button {
                        Left => {
                            camera_rotation += {
                                let adjacent = f32x2::splat(self.camera_distance);
                                let opposite = drag_amount / f32x2::splat(500.);
                                let &[x, y] = (opposite / adjacent).as_array();
                                f32x2::from_array([
                                    y.atan(), // Rotation on x-axis
                                    x.atan(), // Rotation on y-axis
                                ])
                            }
                        }
                        Right => camera_distance += -drag_amount[1] / 250.,
                    }
                    self.update_camera(self.screen_size, camera_rotation, camera_distance);
                } else if modifier_keys.contains(ModifierKeys::CONTROL) {
                    match button {
                        Left => {
                            let adjacent = f32x2::splat(self.camera_distance);
                            let opposite = -drag_amount / f32x2::splat(500.);
                            let &[x, y] = (opposite / adjacent).as_array();
                            self.update_light(
                                self.light_xy_rotation
                                    + f32x2::from_array([
                                        y.atan(), // Rotation on x-axis
                                        x.atan(), // Rotation on y-axis
                                    ]),
                            );
                        }
                        _ => {}
                    }
                }
            }
            KeyDown { key_code, .. } => {
                self.update_mode(match key_code {
                    29 /* 0 */ => Mode::DEFAULT,
                    18 /* 1 */ => Mode::HAS_NORMAL,
                    19 /* 2 */ => Mode::HAS_AMBIENT,
                    20 /* 3 */ => Mode::HAS_AMBIENT | Mode::HAS_DIFFUSE,
                    21 /* 4 */ => Mode::HAS_SPECULAR,
                    _ => self.mode
                });
            }
            WindowResize { size, .. } => {
                self.update_depth_texture_size(size);
                self.update_camera(size, self.camera_rotation, self.camera_distance);
            }
            _ => {}
        }
    }

    #[inline(always)]
    fn needs_render(&self) -> bool {
        self.needs_render
    }
}

impl Delegate {
    #[inline(always)]
    fn reset_needs_render(&mut self) {
        self.needs_render = false;
    }

    fn update_mode(&mut self, mode: Mode) {
        if mode != self.mode {
            self.mode = mode;
            let results = create_pipelines(&self.device, &self.library, mode);
            self.render_pipeline_state = results.model.pipeline_state;
            self.render_light_pipeline_state = results.light.pipeline_state;
            self.needs_render = true;
        }
    }

    fn update_depth_texture_size(&mut self, size: f32x2) {
        let desc = TextureDescriptor::new();
        let &[x, y] = size.as_array();
        desc.set_width(x as _);
        desc.set_height(y as _);
        desc.set_pixel_format(DEPTH_TEXTURE_FORMAT);
        desc.set_storage_mode(MTLStorageMode::Memoryless);
        desc.set_usage(MTLTextureUsage::RenderTarget);
        let texture = self.device.new_texture(&desc);
        texture.set_label("Depth");
        self.depth_texture = Some(texture);
    }

    #[inline]
    fn calc_matrix_camera_to_projection(&self, aspect_ratio: f32) -> f32x4x4 {
        let &[x, y, ..] = self.model.geometry_max_bounds.size.as_array();
        let (w, h) = if x > y {
            (NEAR_FIELD_MAJOR_AXIS, aspect_ratio * NEAR_FIELD_MAJOR_AXIS)
        } else {
            (NEAR_FIELD_MAJOR_AXIS / aspect_ratio, NEAR_FIELD_MAJOR_AXIS)
        };
        let orthographic_matrix = {
            f32x4x4::new(
                [2. / w, 0., 0., 0.],
                [0., 2. / h, 0., 0.],
                // IMPORTANT: Metal's NDC coordinate space has a z range of [0.,1], **NOT [-1,1]** (OpenGL).
                [0., 0., 1. / (F - N), -N / (F - N)],
                [0., 0., 0., 1.],
            )
        };
        orthographic_matrix * PERSPECTIVE_MATRIX
    }

    #[inline(always)]
    fn update_world<T: Sized>(&mut self, id: WorldID, value: T) {
        unsafe {
            *(self.world_arg_encoder.constant_data(id as _) as *mut T) = value;
        };
        self.needs_render = true;
    }

    fn update_light(&mut self, light_xy_rotation: f32x2) {
        self.light_xy_rotation = light_xy_rotation;
        let &[rotx, roty] = self.light_xy_rotation.as_array();
        let light_position =
            f32x4x4::rotate(rotx, roty, 0.) * f32x4::from_array([0., 0., -LIGHT_DISTANCE, 1.]);
        self.update_world(WorldID::LightPosition, light_position);
    }

    fn update_camera(&mut self, screen_size: f32x2, camera_rotation: f32x2, camera_distance: f32) {
        self.screen_size = screen_size;
        self.camera_rotation = camera_rotation;
        self.camera_distance = camera_distance;
        let &[rotx, roty] = self.camera_rotation.neg().as_array();
        let matrix_world_to_camera =
            f32x4x4::translate(0., 0., self.camera_distance) * f32x4x4::rotate(rotx, roty, 0.);
        self.update_world(
            WorldID::CameraPosition,
            matrix_world_to_camera.inverse() * f32x4::from_array([0., 0., 0., 1.]),
        );

        let &[sx, sy, ..] = screen_size.as_array();
        let aspect_ratio = sy / sx;
        let matrix_world_to_projection =
            self.calc_matrix_camera_to_projection(aspect_ratio) * matrix_world_to_camera;

        self.update_world(
            WorldID::MatrixWorldToProjection,
            *matrix_world_to_projection.metal_float4x4(),
        );
        self.update_world(
            WorldID::MatrixModelToProjection,
            *(matrix_world_to_projection * self.matrix_model_to_world).metal_float4x4(),
        );
        let matrix_screen_to_projection =
            f32x4x4::translate(-1., 1., 0.) * f32x4x4::scale(2. / sx, -2. / sy, 1., 1.);
        self.update_world(
            WorldID::MatrixScreenToWorld,
            matrix_world_to_projection.inverse() * matrix_screen_to_projection,
        );
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 4 - Textures");
}
