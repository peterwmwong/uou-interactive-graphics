#![feature(array_zip)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{components::camera, metal::*, metal_types::*, *};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    ops::Neg,
    path::{Path, PathBuf},
    simd::{f32x2, u32x2},
};

const DEPTH_COMPARISON_BIAS: f32 = 4e-4;
const MAX_TEXTURE_SIZE: u16 = 16384;
const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const SHADOW_MAP_DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 32., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 4., -PI / 2.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

struct RenderableModelObject {
    name: &'static str,
    model: Model<{ VertexBufferIndex::Geometry as _ }, { FragBufferIndex::Material as _ }>,
    matrix_model_to_world: f32x4x4,
}

impl RenderableModelObject {
    #[inline]
    fn new<T: AsRef<Path>>(
        name: &'static str,
        device: &Device,
        model_file: T,
        init_matrix_model_to_world: impl FnOnce(&MaxBounds) -> f32x4x4,
    ) -> Self {
        let model = Model::from_file(
            model_file,
            device,
            // TODO: Does #[inline] do anything for these lambdas?
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
        Self {
            matrix_model_to_world: init_matrix_model_to_world(&model.geometry_max_bounds),
            model,
            name,
        }
    }

    #[inline]
    fn encode_use_resources(&self, encoder: &RenderCommandEncoderRef) {
        self.model.encode_use_resources(encoder);
    }

    #[inline]
    fn encode_render(
        &self,
        encoder: &RenderCommandEncoderRef,
        matrix_world_to_projection: f32x4x4,
    ) {
        encoder.push_debug_group(self.name);
        encode_vertex_bytes(
            encoder,
            VertexBufferIndex::ModelSpace as _,
            &ModelSpace {
                matrix_model_to_projection: (matrix_world_to_projection
                    * self.matrix_model_to_world),
                matrix_normal_to_world: self.matrix_model_to_world.into(),
            },
        );
        self.model.encode_draws(encoder);
        encoder.pop_debug_group();
    }
}

// TODO: Kill this. Part of the greater theme of "Don't calculate all the matrices and render/encode time"
// - Migrate back to calculating all these matrices when they change (`on_event`).
impl From<camera::CameraUpdate> for Space {
    fn from(
        camera::CameraUpdate {
            camera_position,
            matrix_screen_to_world,
            matrix_world_to_projection,
        }: camera::CameraUpdate,
    ) -> Self {
        Self {
            matrix_world_to_projection,
            matrix_screen_to_world,
            position_world: camera_position.into(),
        }
    }
}

struct Delegate {
    camera: camera::Camera,
    command_queue: CommandQueue,
    depth_texture: Option<Texture>,
    device: Device,
    needs_render: bool,
    light: camera::Camera,
    model_depth_state: DepthStencilState,
    model_pipeline_state: RenderPipelineState,
    model: RenderableModelObject,
    model_plane: RenderableModelObject,
    shadow_map_depth_state: DepthStencilState,
    shadow_map_pipeline: RenderPipelineState,
    shadow_map_texture: Option<Texture>,
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let executable_name = std::env::args()
            .nth(0)
            .expect("Failed to access command line executable name");
        let model_file_path = std::env::args().nth(1).expect(&format!(
            "Usage: {executable_name} [Path to Wavefront OBJ file]"
        ));

        let mut plane_y = 0_f32;
        let model = RenderableModelObject::new(
            "Model",
            &device,
            PathBuf::from(model_file_path),
            |&MaxBounds { center, size }| {
                let &[cx, cy, cz, _] = center.neg().as_array();

                // IMPORTANT: Normalize the world coordinates to a reasonable range ~[0, 1].
                // 1. Camera distance is invariant of the model's coordinate range
                // 2. Dramatically reduces precision errors (compared to ranges >1000, like in Yoda model)
                //    - In the Vertex Shader, z-fighting in the depth buffer, even with Depth32Float.
                //    - In the Fragment Shader, diffuse and specular lighting is no longer smooth and
                //      exhibit a weird triangal-ish pattern.
                let scale = 1. / size.reduce_max();

                // Put the plane (subsequent RenderableModelObject) right below the model.
                // Store the floating point value as a u32 normalized ([0,1] -> [0,u32::MAX]).
                plane_y = 0.5 * scale * size[2];
                assert!(plane_y >= 0.0 && plane_y <= 1.0, "Calculated Y-coordinate of the Plane is invalid. Calculation is based on the bounding box size of model.");

                (f32x4x4::scale(scale, scale, scale, 1.)
                    * (f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.)))
                    * f32x4x4::translate(cx, cy, cz)
            },
        );
        let model_plane = RenderableModelObject::new(
            "Model",
            &device,
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("plane")
                .join("plane.obj"),
            |_| f32x4x4::translate(0., -plane_y, 0.),
        );

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

        let library = device
            .new_library_with_data(LIBRARY_BYTES)
            .expect("Failed to import shader metal lib.");

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
            light: camera::Camera::new(INITIAL_LIGHT_ROTATION, ModifierKeys::CONTROL, true),
            model,
            model_plane,
            model_pipeline_state: {
                let p = create_pipeline(
                    &device,
                    &library,
                    &mut new_basic_render_pipeline_descriptor(
                        DEFAULT_PIXEL_FORMAT,
                        Some(DEPTH_TEXTURE_FORMAT),
                        false,
                    ),
                    "Model",
                    None,
                    (&"main_vertex", VertexBufferIndex::LENGTH as _),
                    Some((&"main_fragment", FragBufferIndex::LENGTH as _)),
                );
                debug_assert_argument_buffer_size::<
                    { VertexBufferIndex::ModelSpace as _ },
                    ModelSpace,
                >(&p, FunctionType::Vertex);
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
                p.pipeline_state
            },
            shadow_map_depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            // TODO: How much instruction reduction do we get if we reorder/group like things together.
            // - Pipeline Creation
            // - Depth State Creation
            // - Camera and Light Space Creation
            shadow_map_pipeline: {
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
                debug_assert_argument_buffer_size::<
                    { VertexBufferIndex::ModelSpace as _ },
                    ModelSpace,
                >(&p, FunctionType::Vertex);
                debug_assert_argument_buffer_size::<{ VertexBufferIndex::Geometry as _ }, Geometry>(
                    &p,
                    FunctionType::Vertex,
                );
                p.pipeline_state
            },
            shadow_map_texture: None,
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
            encoder.push_debug_group("Shadow Map (Light 1)");
            self.model.encode_use_resources(encoder);
            encoder.set_render_pipeline_state(&self.shadow_map_pipeline);
            encoder.set_depth_stencil_state(&self.shadow_map_depth_state);
            self.model.encode_render(
                encoder,
                // TODO: Avoid calling `create_update`, it's super heavy
                self.light.create_update().matrix_world_to_projection,
            );
            encoder.pop_debug_group();
            encoder.end_encoding();
        }

        // Render Models
        let encoder = command_buffer.new_render_command_encoder(new_basic_render_pass_descriptor(
            render_target,
            self.depth_texture.as_ref(),
        ));
        {
            self.model.encode_use_resources(encoder);
            self.model_plane.encode_use_resources(encoder);
            encoder.set_render_pipeline_state(&self.model_pipeline_state);
            encoder.set_depth_stencil_state(&self.model_depth_state);
            encode_fragment_bytes::<Space>(
                encoder,
                FragBufferIndex::CameraSpace as _,
                // TODO: Avoid calling `create_update`, it's super heavy
                &self.camera.create_update().into(),
            );
            {
                // TODO: Avoid calling `create_update`, it's super heavy
                let light_camera_space = self.light.create_update();
                let light_space = Space {
                    // IMPORTANT: "to_projection" has a different meaning here!
                    // The target coordinate space is **NOT** actually the normal Metal Normalized Device
                    // Coordinate space (NDC).
                    // This is used to sample Shadow Map Depth Texture, it differs in the following ways:
                    // - XY dimension range: [0,1] (not [-1,1])
                    // - Y dimension is inverted: +Y -> -Y
                    // - Z includes a bias for better depth comparison
                    matrix_world_to_projection: {
                        // Performance: Bake all the transform as a constant.
                        // - Currently Rust does not allow floating-point operations in constant
                        //   expressions.
                        // - Reduces the amount of code generated/executed, >150 bytes of instructions saved.
                        const PROJECTION_TO_TEXTURE_COORDINATE_SPACE: f32x4x4 = f32x4x4::new(
                            [0.5, 0.0, 0.0, 0.5],
                            [0.0, -0.5, 0.0, 0.5],
                            [0.0, 0.0, 1.0, -DEPTH_COMPARISON_BIAS],
                            [0.0, 0.0, 0.0, 1.0],
                        );
                        #[cfg(debug_assertions)]
                        {
                            // Invert Y
                            let projection_to_texture_coordinate_space_derived =
                                f32x4x4::scale_translate(1., -1., 1., 0., 1., 0.)
                                    * f32x4x4::scale_translate(
                                        // Convert from [-1, 1] -> [0, 1] for XY dimensions
                                        0.5,
                                        0.5,
                                        1.0,
                                        0.5,
                                        0.5,
                                        // Add Depth Comparison Bias
                                        -DEPTH_COMPARISON_BIAS,
                                    );
                            assert_eq!(
                                projection_to_texture_coordinate_space_derived.columns,
                                PROJECTION_TO_TEXTURE_COORDINATE_SPACE.columns
                            );
                        }
                        PROJECTION_TO_TEXTURE_COORDINATE_SPACE
                    } * light_camera_space.matrix_world_to_projection,
                    matrix_screen_to_world: light_camera_space.matrix_screen_to_world,
                    position_world: light_camera_space.camera_position.into(),
                };
                encode_fragment_bytes::<Space>(
                    encoder,
                    FragBufferIndex::LightSpace as _,
                    &light_space,
                );
            }
            encoder.set_fragment_texture(
                FragTextureIndex::ShadowMap as _,
                self.shadow_map_texture.as_deref(),
            );
            // TODO: Avoid calling `create_update`, it's super heavy
            let matrix_world_to_projection = self.camera.create_update().matrix_world_to_projection;
            self.model
                .encode_render(encoder, matrix_world_to_projection);
            self.model_plane
                .encode_render(encoder, matrix_world_to_projection);
        }
        encoder.end_encoding();
        command_buffer
    }

    #[inline]
    fn on_event(&mut self, event: UserEvent) {
        self.needs_render =
            self.camera.on_event(event).is_some() || self.light.on_event(event).is_some();
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

impl Delegate {
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

        #[cfg(debug_assertions)]
        println!("Allocating new Shadow Map {new_xy:?}");

        desc.set_width(new_xy[0] as _);
        desc.set_height(new_xy[1] as _);
        desc.set_pixel_format(SHADOW_MAP_DEPTH_TEXTURE_FORMAT);
        desc.set_storage_mode(MTLStorageMode::Private);
        desc.set_usage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
        let texture = self.device.new_texture(&desc);
        texture.set_label("Shadow Map Depth");
        self.shadow_map_texture = Some(texture);
    }
}

pub fn run() {
    launch_application::<Delegate>("Project 7 - Shadow Mapping");
}
