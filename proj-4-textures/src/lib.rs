#![feature(array_zip)]
#![feature(portable_simd)]
mod shader_bindings;

use bitflags::bitflags;
use metal_app::{metal::*, ui_ray, *};
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

pub struct Delegate<'a, const RENDER_LIGHT: bool> {
    camera_ray: ui_ray::UIRay,
    depth_state: DepthStencilState,
    depth_texture: Option<Texture>,
    command_queue: CommandQueue,
    pub device: Device,
    library: Library,
    light_ray: ui_ray::UIRay,
    matrix_model_to_world: f32x4x4,
    mode: Mode,
    model: Model,
    render_light_pipeline_state: RenderPipelineState,
    render_pipeline_state: RenderPipelineState,
    screen_size: f32x2,
    world_arg_buffer: Buffer,
    world_arg_ptr: &'a mut World,
    needs_render: bool,
}

struct PipelineResults {
    model_pipeline: CreateRenderPipelineResults,
    light_pipeline: CreateRenderPipelineResults,
}

fn create_pipelines(device: &Device, library: &Library, mode: Mode) -> PipelineResults {
    let base_pipeline_desc = new_basic_render_pipeline_descriptor(
        DEFAULT_PIXEL_FORMAT,
        Some(DEPTH_TEXTURE_FORMAT),
        false,
    );
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
        model_pipeline: create_pipeline(
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
        light_pipeline: create_pipeline(
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

// TODO: START HERE 3
// TODO: START HERE 3
// TODO: START HERE 3
// Can we change the API so we're just passing a FnMut() for Material and Geometry Encoding?
enum MaterialArgEncoder {}
impl MaterialArgumentEncoder<Material> for MaterialArgEncoder {
    fn set(
        arg: &mut Material,
        ambient_texture: MetalGPUAddress,
        diffuse_texture: MetalGPUAddress,
        specular_texture: MetalGPUAddress,
        specular_shineness: f32,
    ) {
        arg.ambient_texture = ambient_texture;
        arg.diffuse_texture = diffuse_texture;
        arg.specular_texture = specular_texture;
        arg.specular_shineness = specular_shineness;
    }
}

enum GeometryArgEncoder {}
impl GeometryArgumentEncoder<Geometry> for GeometryArgEncoder {
    fn set(
        arg: &mut Geometry,
        indices_buffer: MetalGPUAddress,
        positions_buffer: MetalGPUAddress,
        normals_buffer: MetalGPUAddress,
        tx_coords_buffer: MetalGPUAddress,
    ) {
        arg.indices = indices_buffer;
        arg.positions = positions_buffer;
        arg.normals = normals_buffer;
        arg.tx_coords = tx_coords_buffer;
    }
}

impl<'a, const RENDER_LIGHT: bool> RendererDelgate for Delegate<'a, RENDER_LIGHT> {
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
        let PipelineResults {
            model_pipeline,
            light_pipeline,
        } = create_pipelines(&device, &library, mode);
        #[cfg(debug_assertions)]
        {
            let model_pipeline_reflection = &model_pipeline.pipeline_state_reflection;
            let geometry_arg_size = model_pipeline_reflection
                .vertex_bindings()
                .object_at_as::<BufferBindingRef>(VertexBufferIndex::Geometry as _)
                .expect("Failed to access geometry vertex buffer argument information")
                .buffer_data_size() as u32;
            let material_arg_size = model_pipeline_reflection
                .fragment_bindings()
                .object_at_as::<BufferBindingRef>(FragBufferIndex::Material as _)
                .expect("Failed to access material fragment buffer argument information")
                .buffer_data_size() as u32;
            debug_assert_eq!(std::mem::size_of::<Geometry>(), geometry_arg_size as _, "Shader bindings generated a differently sized Geometry struct than what Metal expects");
            debug_assert_eq!(std::mem::size_of::<Material>(), material_arg_size as _, "Shader bindings generated a differently sized Material struct than what Metal expects");
        }
        let model = Model::from_file::<
            PathBuf,
            Geometry,
            GeometryArgEncoder,
            Material,
            MaterialArgEncoder,
        >(model_file, &device);

        #[cfg(debug_assertions)]
        {
            let model_pipeline_reflection = &model_pipeline.pipeline_state_reflection;

            let vertex_world_arg_size = model_pipeline_reflection
                .vertex_bindings()
                .object_at_as::<BufferBindingRef>(VertexBufferIndex::World as _)
                .expect("Failed to access world vertex buffer argument information")
                .buffer_data_size() as u32;
            debug_assert_eq!(std::mem::size_of::<World>(), vertex_world_arg_size as _, "Shader bindings generated a differently sized Vertex World struct than what Metal expects");

            let fragment_world_arg_size = model_pipeline_reflection
                .fragment_bindings()
                .object_at_as::<BufferBindingRef>(FragBufferIndex::World as _)
                .expect("Failed to access world fragment buffer argument information")
                .buffer_data_size() as u32;
            debug_assert_eq!(std::mem::size_of::<World>(), fragment_world_arg_size as _, "Shader bindings generated a differently sized Fragment World struct than what Metal expects");
        }
        let world_arg_buffer = device.new_buffer(
            std::mem::size_of::<World>() as _,
            MTLResourceOptions::CPUCacheModeWriteCombined | MTLResourceOptions::StorageModeShared,
        );
        world_arg_buffer.set_label("World Argument Buffer");
        let world_arg_ptr = unsafe { &mut *(world_arg_buffer.contents() as *mut World) };

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
        world_arg_ptr.matrix_normal_to_world = matrix_model_to_world.into();

        let mut delegate = Self {
            camera_ray: ui_ray::UIRay::new(
                ModifierKeys::empty(),
                INITIAL_CAMERA_DISTANCE,
                INITIAL_CAMERA_ROTATION,
                false,
            ),
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: None,
            library,
            light_ray: ui_ray::UIRay::new(
                ModifierKeys::CONTROL,
                LIGHT_DISTANCE,
                INITIAL_LIGHT_ROTATION,
                true,
            ),
            matrix_model_to_world,
            mode,
            model,
            render_pipeline_state: model_pipeline.pipeline_state,
            render_light_pipeline_state: light_pipeline.pipeline_state,
            screen_size: f32x2::default(),
            world_arg_buffer,
            world_arg_ptr,
            needs_render: false,
            command_queue: device.new_command_queue(),
            device,
        };

        // IMPORTANT: Not a mistake, using Model-to-World Rotation 4x4 Matrix for
        // Normal-to-World 3x3 Matrix. Conceptually, we want a matrix that ONLY applies rotation
        // (no translation). Since normals are directions (not positions, relative to a
        // point on a surface), translations are meaningless.
        delegate.update_light();
        delegate.update_camera(delegate.screen_size);
        delegate.reset_needs_render();
        delegate
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.reset_needs_render();
        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        command_buffer.set_label("Renderer Command Buffer");
        let encoder = command_buffer.new_render_command_encoder(new_basic_render_pass_descriptor(
            render_target,
            self.depth_texture.as_ref(),
        ));
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
        if RENDER_LIGHT {
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
        if self.camera_ray.on_event(event) {
            self.update_camera(self.screen_size);
        } else if self.light_ray.on_event(event) {
            self.update_light();
        } else {
            match event {
                UserEvent::KeyDown { key_code, .. } => {
                    self.update_mode(match key_code {
                    29 /* 0 */ => Mode::DEFAULT,
                    18 /* 1 */ => Mode::HAS_NORMAL,
                    19 /* 2 */ => Mode::HAS_AMBIENT,
                    20 /* 3 */ => Mode::HAS_AMBIENT | Mode::HAS_DIFFUSE,
                    21 /* 4 */ => Mode::HAS_SPECULAR,
                    _ => self.mode
                });
                }
                UserEvent::WindowResize { size, .. } => {
                    self.update_depth_texture_size(size);
                    self.update_camera(size);
                }
                _ => {}
            }
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

impl<'a, const RENDER_LIGHT: bool> Delegate<'a, RENDER_LIGHT> {
    #[inline(always)]
    fn reset_needs_render(&mut self) {
        self.needs_render = false;
    }
    #[inline(always)]
    fn set_needs_render(&mut self) {
        self.needs_render = true;
    }

    fn update_mode(&mut self, mode: Mode) {
        if mode != self.mode {
            self.mode = mode;
            let results = create_pipelines(&self.device, &self.library, mode);
            self.render_pipeline_state = results.model_pipeline.pipeline_state;
            self.render_light_pipeline_state = results.light_pipeline.pipeline_state;
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

    fn update_light(&mut self) {
        let &[rotx, roty] = self.light_ray.rotation_xy.as_array();
        let light_position = f32x4x4::rotate(rotx, roty, 0.)
            * f32x4::from_array([0., 0., -self.light_ray.distance_from_origin, 1.]);
        self.world_arg_ptr.light_position = light_position.into();
        self.set_needs_render();
    }

    fn update_camera(&mut self, screen_size: f32x2) {
        self.screen_size = screen_size;
        let &[rotx, roty] = self.camera_ray.rotation_xy.neg().as_array();
        let matrix_world_to_camera =
            f32x4x4::translate(0., 0., self.camera_ray.distance_from_origin)
                * f32x4x4::rotate(rotx, roty, 0.);
        self.world_arg_ptr.camera_position =
            (matrix_world_to_camera.inverse() * f32x4::from_array([0., 0., 0., 1.])).into();

        let &[sx, sy, ..] = (f32x2::splat(2.) / screen_size).as_array();
        let aspect_ratio = sx / sy;
        let matrix_world_to_projection =
            self.calc_matrix_camera_to_projection(aspect_ratio) * matrix_world_to_camera;

        self.world_arg_ptr.matrix_world_to_projection = matrix_world_to_projection;
        self.world_arg_ptr.matrix_model_to_projection =
            matrix_world_to_projection * self.matrix_model_to_world;
        let matrix_screen_to_projection = f32x4x4::scale_translate(sx, -sy, 1., -1., 1., 0.);
        self.world_arg_ptr.matrix_screen_to_world =
            matrix_world_to_projection.inverse() * matrix_screen_to_projection;
        self.set_needs_render();
    }
}

pub fn run() {
    launch_application::<Delegate<true>>("Project 4 - Textures");
}
