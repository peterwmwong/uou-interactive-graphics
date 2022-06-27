#![feature(array_zip)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{components::camera, metal::*, metal_types::*, *};
use shader_bindings::*;
use std::{f32::consts::PI, ops::Neg, path::PathBuf, simd::f32x2};

const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 32., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

struct Delegate<'a> {
    camera: camera::Camera,
    command_queue: CommandQueue,
    depth_texture: Option<Texture>,
    device: Device,
    matrix_model_to_world: f32x4x4,
    needs_render: bool,
    model_depth_state: DepthStencilState,
    model_pipeline_state: RenderPipelineState,
    model: Model<{ VertexBufferIndex::Geometry as _ }, { NO_MATERIALS_ID }>,
    plane_pipeline_state: RenderPipelineState,
    world_arg_buffer: Buffer,
    world_arg_ptr: &'a mut World,
}

impl<'a> RendererDelgate for Delegate<'a> {
    fn new(device: Device) -> Self {
        let executable_name = std::env::args()
            .nth(0)
            .expect("Failed to access command line executable name");
        let model_file_path = std::env::args().nth(1).expect(&format!(
            "Usage: {executable_name} [Path to Wavefront OBJ file]"
        ));

        let model_file = PathBuf::from(model_file_path);
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
            NO_MATERIALS_ENCODER,
        );
        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");

        let mut render_pipeline_desc = new_basic_render_pipeline_descriptor(
            DEFAULT_PIXEL_FORMAT,
            Some(DEPTH_TEXTURE_FORMAT),
            false,
        );
        let model_pipeline = {
            let p = create_pipeline(
                &device,
                &library,
                &mut render_pipeline_desc,
                "Model",
                None,
                &"main_vertex",
                VertexBufferIndex::LENGTH as _,
                &"main_fragment",
                FragBufferIndex::LENGTH as _,
            );
            debug_assert_argument_buffer_size::<{ VertexBufferIndex::World as _ }, World>(
                &p,
                FunctionType::Vertex,
            );
            debug_assert_argument_buffer_size::<{ VertexBufferIndex::Geometry as _ }, Geometry>(
                &p,
                FunctionType::Vertex,
            );
            debug_assert_argument_buffer_size::<{ FragBufferIndex::World as _ }, World>(
                &p,
                FunctionType::Fragment,
            );
            p
        };
        let plane_pipeline = {
            let p = create_pipeline(
                &device,
                &library,
                &mut render_pipeline_desc,
                "Plane",
                None,
                &"plane_vertex",
                VertexBufferIndex::LENGTH as _,
                &"plane_fragment",
                FragBufferIndex::LENGTH as _,
            );
            debug_assert_argument_buffer_size::<{ VertexBufferIndex::World as _ }, World>(
                &p,
                FunctionType::Vertex,
            );
            debug_assert_argument_buffer_size::<{ FragBufferIndex::World as _ }, World>(
                &p,
                FunctionType::Fragment,
            );
            p
        };
        let world_arg_buffer = device.new_buffer(
            std::mem::size_of::<World>() as _,
            MTLResourceOptions::CPUCacheModeWriteCombined | MTLResourceOptions::StorageModeShared,
        );
        world_arg_buffer.set_label("World Argument Buffer");
        let world_arg_ptr = unsafe { &mut *(world_arg_buffer.contents() as *mut World) };

        let &MaxBounds { center, size } = &model.geometry_max_bounds;
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
        let matrix_model_to_world = (f32x4x4::scale(scale, scale, scale, 1.)
            * (f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.)))
            * f32x4x4::translate(cx, cy, cz);

        // IMPORTANT: Not a mistake, using Model-to-World Rotation 4x4 Matrix for
        // Normal-to-World 3x3 Matrix. Conceptually, we want a matrix that ONLY applies rotation
        // (no translation). Since normals are directions (not positions, relative to a
        // point on a surface), translations are meaningless.
        world_arg_ptr.matrix_model_to_world = matrix_model_to_world.into();
        world_arg_ptr.matrix_normal_to_world = matrix_model_to_world.into();
        world_arg_ptr.plane_y = -0.5 * scale * size[2];

        Self {
            camera: camera::Camera::new(INITIAL_CAMERA_ROTATION, ModifierKeys::empty(), false),
            command_queue: device.new_command_queue(),
            model_depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: None,
            matrix_model_to_world,
            model,
            model_pipeline_state: model_pipeline.pipeline_state,
            plane_pipeline_state: plane_pipeline.pipeline_state,
            world_arg_buffer,
            world_arg_ptr,
            needs_render: false,
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
        let encoder = command_buffer.new_render_command_encoder(new_basic_render_pass_descriptor(
            render_target,
            self.depth_texture.as_ref(),
        ));
        {
            encoder.push_debug_group("Model");
            self.model.encode_use_resources(encoder);
            encoder.set_render_pipeline_state(&self.model_pipeline_state);
            encoder.set_depth_stencil_state(&self.model_depth_state);
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
            self.model.encode_draws(encoder);
            encoder.pop_debug_group();
        }
        {
            encoder.push_debug_group("Plane");
            encoder.set_render_pipeline_state(&self.plane_pipeline_state);
            encoder.draw_primitives(MTLPrimitiveType::TriangleStrip, 0, 4);
            encoder.pop_debug_group();
        }
        encoder.end_encoding();
        command_buffer
    }

    #[inline]
    fn on_event(&mut self, event: UserEvent) {
        self.camera.on_event(
            event,
            |camera::CameraUpdate {
                 matrix_world_to_projection,
                 camera_position,
                 matrix_screen_to_world,
             }| {
                self.world_arg_ptr.matrix_world_to_projection = matrix_world_to_projection;
                self.world_arg_ptr.matrix_model_to_projection =
                    matrix_world_to_projection * self.matrix_model_to_world;
                self.world_arg_ptr.camera_position = camera_position.into();
                self.world_arg_ptr.matrix_screen_to_world = matrix_screen_to_world;
                self.needs_render = true;
            },
        );

        match event {
            UserEvent::WindowFocusedOrResized { size } => {
                self.update_textures_size(size);
                self.needs_render = true;
            }
            _ => {}
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

impl<'a> Delegate<'a> {
    #[inline]
    fn update_textures_size(&mut self, size: f32x2) {
        let desc = TextureDescriptor::new();
        let &[x, y] = size.as_array();
        desc.set_width(x as _);
        desc.set_height(y as _);
        desc.set_pixel_format(DEPTH_TEXTURE_FORMAT);
        desc.set_storage_mode(MTLStorageMode::Private);
        desc.set_usage(MTLTextureUsage::RenderTarget);
        let texture = self.device.new_texture(&desc);
        texture.set_label("Depth");
        self.depth_texture = Some(texture);
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 7 - Shadow Mapping");
}
