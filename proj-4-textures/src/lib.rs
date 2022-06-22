#![feature(array_zip)]
#![feature(portable_simd)]
mod shader_bindings;

use bitflags::bitflags;
use metal_app::{components::*, metal::*, metal_types::*, *};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    ops::Neg,
    path::PathBuf,
    simd::{f32x2, f32x4},
};

bitflags! {
    struct Mode: usize {
        const HAS_AMBIENT = 1 << FC::HasAmbient as usize;
        const HAS_DIFFUSE = 1 << FC::HasDiffuse as usize;
        const HAS_NORMAL = 1 << FC::HasNormal as usize;
        const HAS_SPECULAR = 1 << FC::HasSpecular as usize;
        const DEFAULT = Self::HAS_AMBIENT.bits | Self::HAS_DIFFUSE.bits | Self::HAS_SPECULAR.bits;
    }
}

const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 4., 0.]);
const INITIAL_MODE: Mode = Mode::DEFAULT;
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_DISTANCE: f32 = 0.5;

pub struct Delegate<'a, const RENDER_LIGHT: bool> {
    camera: camera::Camera,
    depth_state: DepthStencilState,
    depth_texture: Option<Texture>,
    command_queue: CommandQueue,
    pub device: Device,
    library: Library,
    light_ray: ui_ray::UIRay,
    matrix_model_to_world: f32x4x4,
    mode: Mode,
    model: Model<{ VertexBufferIndex::Geometry as _ }, { FragBufferIndex::Material as _ }>,
    render_light_pipeline_state: RenderPipelineState,
    render_pipeline_state: RenderPipelineState,
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
                .buffer_data_size();
            let material_arg_size = model_pipeline_reflection
                .fragment_bindings()
                .object_at_as::<BufferBindingRef>(FragBufferIndex::Material as _)
                .expect("Failed to access material fragment buffer argument information")
                .buffer_data_size();
            debug_assert_eq!(std::mem::size_of::<Geometry>(), geometry_arg_size as _, "Shader bindings generated a differently sized Geometry struct than what Metal expects");
            debug_assert_eq!(std::mem::size_of::<Material>(), material_arg_size as _, "Shader bindings generated a differently sized Material struct than what Metal expects");
        }
        let model = Model::from_file(
            model_file,
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
            |arg: &mut Material,
             MaterialToEncode {
                 ambient_texture,
                 diffuse_texture,
                 specular_texture,
                 specular_shineness,
             }| {
                arg.ambient_texture = ambient_texture;
                arg.diffuse_texture = diffuse_texture;
                arg.specular_texture = specular_texture;
                arg.specular_shineness = specular_shineness;
            },
        );

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
        // 1. Camera distance is invariant of the model's coordinate range
        // 2. Dramatically reduces precision errors (compared to ranges >1000, like in Yoda model)
        //    - In the Vertex Shader, z-fighting in the depth buffer, even with Depth32Float.
        //    - In the Fragment Shader, diffuse and specular lighting is no longer smooth and
        //      exhibit a weird triangal-ish pattern.
        let scale = 1. / size.reduce_max();

        // TODO: This generates an immense amount of code!
        // - It's the matrix multiplications we're unable to avoid with const evaluation (currently not supported in rust for floating point operations)
        // - We can create combo helpers, see f32x4x4::scale_translate()
        let matrix_model_to_world = f32x4x4::scale(scale, scale, scale, 1.)
            * (f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.))
            * f32x4x4::translate(cx, cy, cz);
        world_arg_ptr.matrix_normal_to_world = matrix_model_to_world.into();

        let mut delegate = Self {
            camera: camera::Camera::new(INITIAL_CAMERA_ROTATION, ModifierKeys::empty()),
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
        delegate.needs_render = false;
        delegate
    }

    #[inline]
    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.needs_render = false;
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
            self.model.encode_draws(&encoder);
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
        self.camera.on_event(
            event,
            |camera::CameraUpdate {
                 camera_position,
                 matrix_screen_to_world,
                 matrix_world_to_projection,
             }| {
                self.world_arg_ptr.camera_position = camera_position.into();
                self.world_arg_ptr.matrix_screen_to_world = matrix_screen_to_world;
                self.world_arg_ptr.matrix_world_to_projection = matrix_world_to_projection;
                self.world_arg_ptr.matrix_model_to_projection =
                    matrix_world_to_projection * self.matrix_model_to_world;
                self.needs_render = true;
            },
        );

        if self.light_ray.on_event(event) {
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

    fn update_light(&mut self) {
        let &[rotx, roty] = self.light_ray.rotation_xy.as_array();
        let light_position = f32x4x4::rotate(rotx, roty, 0.)
            * f32x4::from_array([0., 0., -self.light_ray.distance_from_origin, 1.]);
        self.world_arg_ptr.light_position = light_position.into();
        self.needs_render = true;
    }
}

pub fn run() {
    launch_application::<Delegate<true>>("Project 4 - Textures");
}
