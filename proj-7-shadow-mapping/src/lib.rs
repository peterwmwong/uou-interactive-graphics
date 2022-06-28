#![feature(array_zip)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{components::camera, metal::*, metal_types::*, *};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    ops::Neg,
    path::PathBuf,
    simd::{f32x2, u32x2},
};

const MAX_TEXTURE_SIZE: u16 = 16384;
const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const SHADOW_MAP_DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth32Float;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 32., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 4., -PI / 2.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

struct Delegate<'a> {
    camera: camera::Camera,
    command_queue: CommandQueue,
    depth_texture: Option<Texture>,
    device: Device,
    matrix_model_to_world: f32x4x4,
    needs_render: bool,
    light: camera::Camera,
    model_depth_state: DepthStencilState,
    model_pipeline_state: RenderPipelineState,
    model: Model<{ VertexBufferIndex::Geometry as _ }, { NO_MATERIALS_ID }>,
    plane_pipeline_state: RenderPipelineState,
    plane_y_unorm: u32,
    shadow_map_depth_state: DepthStencilState,
    shadow_map_pipeline: RenderPipelineState,
    shadow_map_texture: Option<Texture>,
    // TODO: Create a new metal-app TypedBuffer<T> abstraction for Argument Buffers with a Type.
    light_arg_buffer: Buffer,
    light_arg_ptr: &'a mut Space,
    camera_arg_buffer: Buffer,
    camera_arg_ptr: &'a mut Space,
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

        // TODO: Change create_pipline to take a Function objects and create helper for getting many
        // function in one shot.
        // - There's alot of reusing of vertex and fragment functions
        //    - `main_vertex` x 2
        //    - `main_fragment` x 2
        // - Alternatively (maybe even better), we extract the descriptor and allow callers to mutate
        //   and reuse the descriptor.
        //    1. Create descriptor
        //    2. Create Pipeline 1
        //    3. Change fragment function
        //    4. Create Pipeline 2
        //    5. etc.

        // Depth-Only Shadow Map Render Pipeline
        let shadow_map_pipeline = {
            let mut depth_only_desc = RenderPipelineDescriptor::new();
            depth_only_desc.set_depth_attachment_pixel_format(SHADOW_MAP_DEPTH_TEXTURE_FORMAT);
            let p = create_pipeline(
                &device,
                &library,
                &mut depth_only_desc,
                "Shadow Map",
                None,
                (&"main_vertex", VertexBufferIndex::LENGTH as _),
                None,
            );
            debug_assert_argument_buffer_size::<{ VertexBufferIndex::Space as _ }, Space>(
                &p,
                FunctionType::Vertex,
            );
            debug_assert_argument_buffer_size::<{ VertexBufferIndex::Geometry as _ }, Geometry>(
                &p,
                FunctionType::Vertex,
            );
            p
        };

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
                (&"main_vertex", VertexBufferIndex::LENGTH as _),
                Some((&"main_fragment", FragBufferIndex::LENGTH as _)),
            );
            debug_assert_argument_buffer_size::<{ VertexBufferIndex::Space as _ }, Space>(
                &p,
                FunctionType::Vertex,
            );
            debug_assert_argument_buffer_size::<{ VertexBufferIndex::Geometry as _ }, Geometry>(
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
            p
        };
        let plane_pipeline = {
            let p = create_pipeline(
                &device,
                &library,
                &mut render_pipeline_desc,
                "Plane",
                None,
                (&"plane_vertex", VertexBufferIndex::LENGTH as _),
                Some((&"main_fragment", FragBufferIndex::LENGTH as _)),
            );
            debug_assert_argument_buffer_size::<{ VertexBufferIndex::Space as _ }, Space>(
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
            p
        };
        let &MaxBounds { center, size } = &model.geometry_max_bounds;
        let &[cx, cy, cz, _] = center.neg().as_array();

        // IMPORTANT: Normalize the world coordinates to a reasonable range ~[0, 1].
        // 1. Camera distance is invariant of the model's coordinate range
        // 2. Dramatically reduces precision errors (compared to ranges >1000, like in Yoda model)
        //    - In the Vertex Shader, z-fighting in the depth buffer, even with Depth32Float.
        //    - In the Fragment Shader, diffuse and specular lighting is no longer smooth and
        //      exhibit a weird triangal-ish pattern.
        let scale = 1. / size.reduce_max();

        // Put the plane right below the model.
        // Store the floating point value as a u32 normalized ([0,1] -> [0,u32::MAX]).
        let plane_y = 0.5 * scale * size[2];
        assert!(plane_y >= 0.0 && plane_y <= 1.0, "Calculated Y-coordinate of the Plane is invalid. Calculation is based on the bounding box size of model.");
        let plane_y_unorm = (plane_y * (u32::MAX as f32)) as u32;

        // TODO: This generates an immense amount of code!
        // - It's the matrix multiplications we're unable to avoid with const evaluation (currently not supported in rust for floating point operations)
        // - We can create combo helpers, see f32x4x4::scale_translate()
        let matrix_model_to_world = (f32x4x4::scale(scale, scale, scale, 1.)
            * (f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.)))
            * f32x4x4::translate(cx, cy, cz);

        let camera_arg_buffer =
            device.new_buffer(std::mem::size_of::<Space>() as _, DEFAULT_RESOURCE_OPTIONS);
        camera_arg_buffer.set_label("Camera Space Argument Buffer");
        let camera_arg_ptr = unsafe { &mut *(camera_arg_buffer.contents() as *mut Space) };
        // IMPORTANT: Not a mistake, using Model-to-World Rotation 4x4 Matrix for
        // Normal-to-World 3x3 Matrix. Conceptually, we want a matrix that ONLY applies rotation
        // (no translation). Since normals are directions (not positions, relative to a
        // point on a surface), translations are meaningless.
        camera_arg_ptr.matrix_normal_to_world = matrix_model_to_world.into();

        let light_arg_buffer =
            device.new_buffer(std::mem::size_of::<Space>() as _, DEFAULT_RESOURCE_OPTIONS);
        light_arg_buffer.set_label("Light Space Argument Buffer");
        let light_arg_ptr = unsafe { &mut *(light_arg_buffer.contents() as *mut Space) };
        light_arg_ptr.matrix_normal_to_world = matrix_model_to_world.into();

        Self {
            camera: camera::Camera::new(INITIAL_CAMERA_ROTATION, ModifierKeys::empty(), false),
            command_queue: device.new_command_queue(),
            // TODO: If model_depth_state and shadow_map_depth_state are the same, just keep one.
            model_depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: None,
            matrix_model_to_world,
            light: camera::Camera::new(INITIAL_LIGHT_ROTATION, ModifierKeys::CONTROL, true),
            model,
            model_pipeline_state: model_pipeline.pipeline_state,
            plane_pipeline_state: plane_pipeline.pipeline_state,
            plane_y_unorm,
            shadow_map_depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            shadow_map_pipeline: shadow_map_pipeline.pipeline_state,
            shadow_map_texture: None,
            light_arg_buffer,
            light_arg_ptr,
            camera_arg_buffer,
            camera_arg_ptr,
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

        // Render Shadow Map
        {
            let desc = RenderPassDescriptor::new();
            let a = desc
                .depth_attachment()
                .expect("Failed to access depth/stencil attachment on render pass descriptor");
            a.set_clear_depth(1.);
            a.set_load_action(MTLLoadAction::Clear);
            a.set_store_action(MTLStoreAction::Store);
            a.set_texture(self.shadow_map_texture.as_deref());

            let encoder = command_buffer.new_render_command_encoder(desc);
            {
                encoder.push_debug_group("Shadow Map (Light 1)");
                self.model.encode_use_resources(encoder);
                encoder.set_render_pipeline_state(&self.shadow_map_pipeline);
                encoder.set_depth_stencil_state(&self.shadow_map_depth_state);
                encoder.set_vertex_buffer(
                    VertexBufferIndex::Space as _,
                    Some(&self.light_arg_buffer),
                    0,
                );
                self.model.encode_draws(encoder);
                encoder.pop_debug_group();
            }
            encoder.end_encoding();
        }

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
                VertexBufferIndex::Space as _,
                Some(&self.camera_arg_buffer),
                0,
            );
            encoder.set_fragment_buffer(
                FragBufferIndex::CameraSpace as _,
                Some(&self.camera_arg_buffer),
                0,
            );
            encoder.set_fragment_buffer(
                FragBufferIndex::LightSpace as _,
                Some(&self.light_arg_buffer),
                0,
            );
            encoder.set_fragment_texture(
                FragTextureIndex::ShadowMap as _,
                self.shadow_map_texture.as_deref(),
            );
            self.model.encode_draws(encoder);
            encoder.pop_debug_group();
        }
        {
            encoder.push_debug_group("Plane");
            encoder.set_render_pipeline_state(&self.plane_pipeline_state);
            encoder.draw_primitives_instanced_base_instance(
                MTLPrimitiveType::TriangleStrip,
                0,
                4,
                1,
                // Misuse the instance_id to store the Plane's Y-coordinate!
                self.plane_y_unorm as _,
            );
            encoder.pop_debug_group();
        }
        encoder.end_encoding();
        command_buffer
    }

    #[inline]
    fn on_event(&mut self, event: UserEvent) {
        if let Some(camera::CameraUpdate {
            matrix_world_to_projection,
            camera_position,
            matrix_screen_to_world,
        }) = self.camera.on_event(event)
        {
            self.camera_arg_ptr.matrix_world_to_projection = matrix_world_to_projection;
            self.camera_arg_ptr.matrix_model_to_projection =
                matrix_world_to_projection * self.matrix_model_to_world;
            self.camera_arg_ptr.position_world = camera_position.into();
            self.camera_arg_ptr.matrix_screen_to_world = matrix_screen_to_world;
            self.needs_render = true;
        }

        if let Some(camera::CameraUpdate {
            matrix_world_to_projection,
            camera_position,
            matrix_screen_to_world,
        }) = self.light.on_event(event)
        {
            self.light_arg_ptr.matrix_world_to_projection = matrix_world_to_projection;
            self.light_arg_ptr.matrix_model_to_projection =
                matrix_world_to_projection * self.matrix_model_to_world;
            self.light_arg_ptr.position_world = camera_position.into();
            self.light_arg_ptr.matrix_screen_to_world = matrix_screen_to_world;
            self.needs_render = true;
        }

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
        desc.set_storage_mode(MTLStorageMode::Memoryless);
        desc.set_usage(MTLTextureUsage::RenderTarget);
        let texture = self.device.new_texture(&desc);
        texture.set_label("Depth");
        self.depth_texture = Some(texture);

        // Make sure the shadow map texture is atleast 2x and no more than 4x, of the
        // screen size. Round up to the nearest power of 2 of each dimension.
        let xy = u32x2::from_array([size[0] as u32, size[1] as u32]);
        if let Some(tx) = &self.shadow_map_texture {
            #[inline(always)]
            fn is_shadow_map_correctly_sized(cur: NSUInteger, target: u32) -> bool {
                ((target << 1)..=(target << 2)).contains(&(cur as _))
            }
            if is_shadow_map_correctly_sized(tx.width(), xy[0])
                && is_shadow_map_correctly_sized(tx.height(), xy[1])
            {
                return;
            }
        }

        #[inline]
        fn round_up_pow_of_2(mut v: u32x2) -> u32x2 {
            v -= u32x2::splat(1);
            v |= v >> u32x2::splat(1);
            v |= v >> u32x2::splat(2);
            v |= v >> u32x2::splat(4);
            v |= v >> u32x2::splat(8);
            v |= v >> u32x2::splat(16);
            (v + u32x2::splat(1)).min(u32x2::splat(MAX_TEXTURE_SIZE as _))
        }
        let new_xy = round_up_pow_of_2(xy << u32x2::splat(1));
        println!("Allocating new Shadow Map {new_xy:?}");

        desc.set_width(new_xy[0] as _);
        desc.set_height(new_xy[1] as _);
        desc.set_storage_mode(MTLStorageMode::Private);
        desc.set_pixel_format(SHADOW_MAP_DEPTH_TEXTURE_FORMAT);
        desc.set_usage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
        let texture = self.device.new_texture(&desc);
        texture.set_label("Shadow Map Depth");
        self.shadow_map_texture = Some(texture);
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 7 - Shadow Mapping");
}
