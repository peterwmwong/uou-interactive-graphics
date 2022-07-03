#![feature(array_zip)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::{Camera, CameraUpdate, ShadingModeSelector},
    metal::*,
    metal_types::*,
    *,
};
use shader_bindings::{ShadingMode, *};
use std::{f32::consts::PI, ops::Neg, path::PathBuf, simd::f32x2};

const DEFAULT_AMBIENT_AMOUNT: f32 = 0.15;
const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 4., 0.]);
const INITIAL_MODE: ShadingModeSelector = ShadingModeSelector::DEFAULT;
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_DISTANCE: f32 = 0.5;

pub struct Delegate<'a, const RENDER_LIGHT: bool> {
    camera: Camera,
    command_queue: CommandQueue,
    depth_state: DepthStencilState,
    depth_texture: Option<Texture>,
    library: Library,
    light_pipeline: RenderPipelineState,
    light: Camera,
    matrix_model_to_world: f32x4x4,
    model_pipeline: RenderPipelineState,
    model: Model<{ VertexBufferIndex::Geometry as _ }, { FragBufferIndex::Material as _ }>,
    needs_render: bool,
    device: Device,
    shading_mode: ShadingModeSelector,
    world_arg_buffer: Buffer,
    world_arg_ptr: &'a mut World,
}

fn create_pipelines(
    device: &Device,
    library: &Library,
    mode: ShadingModeSelector,
) -> (RenderPipelineState, RenderPipelineState) {
    let function_constants = mode.encode(
        FunctionConstantValues::new(),
        ShadingMode::HasAmbient as _,
        ShadingMode::HasDiffuse as _,
        ShadingMode::HasSpecular as _,
        ShadingMode::OnlyNormals as _,
    );
    (
        {
            let p = create_render_pipeline(
                &device,
                &new_render_pipeline_descriptor(
                    "Plane",
                    &library,
                    Some((DEFAULT_PIXEL_FORMAT, false)),
                    Some(DEPTH_TEXTURE_FORMAT),
                    Some(&function_constants),
                    Some((&"main_vertex", VertexBufferIndex::LENGTH as _)),
                    Some((&"main_fragment", FragBufferIndex::LENGTH as _)),
                ),
            );
            debug_assert_argument_buffer_size::<{ VertexBufferIndex::Geometry as _ }, Geometry>(
                &p,
                FunctionType::Vertex,
            );
            debug_assert_argument_buffer_size::<{ FragBufferIndex::Material as _ }, Material>(
                &p,
                FunctionType::Fragment,
            );
            p.pipeline_state
        },
        {
            let p = create_render_pipeline(
                &device,
                &new_render_pipeline_descriptor(
                    "Light",
                    &library,
                    Some((DEFAULT_PIXEL_FORMAT, false)),
                    Some(DEPTH_TEXTURE_FORMAT),
                    Some(&function_constants),
                    Some((&"light_vertex", LightVertexBufferIndex::LENGTH as _)),
                    Some((&"light_fragment", 0)),
                ),
            );
            debug_assert_argument_buffer_size::<{ LightVertexBufferIndex::World as _ }, World>(
                &p,
                FunctionType::Vertex,
            );
            p.pipeline_state
        },
    )
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
        let (model_pipeline, light_pipeline) = create_pipelines(&device, &library, mode);
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
                arg.ambient_amount = DEFAULT_AMBIENT_AMOUNT;
            },
        );
        let world_arg_buffer =
            device.new_buffer(std::mem::size_of::<World>() as _, DEFAULT_RESOURCE_OPTIONS);
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

        // TODO: DO IT. This generates an immense amount of code!
        // - It's the matrix multiplications we're unable to avoid with const evaluation (currently not supported in rust for floating point operations)
        // - We can create combo helpers, see f32x4x4::scale_translate()
        let model_to_world_scale_rot = f32x4x4::scale(scale, scale, scale, 1.)
            * (f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.));
        let matrix_model_to_world = model_to_world_scale_rot * f32x4x4::translate(cx, cy, cz);

        // IMPORTANT: Not a mistake, using Model-to-World Rotation 4x4 Matrix for
        // Normal-to-World 3x3 Matrix. Conceptually, we want a matrix that ONLY applies rotation
        // (no translation). Since normals are directions (not positions, relative to a
        // point on a surface), translations are meaningless.
        world_arg_ptr.matrix_normal_to_world = matrix_model_to_world.into();

        Self {
            camera: Camera::new_with_default_distance(
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            command_queue: device.new_command_queue(),
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: None,
            library,
            light: Camera::new(
                LIGHT_DISTANCE,
                INITIAL_LIGHT_ROTATION,
                ModifierKeys::CONTROL,
                true,
                0.,
            ),
            light_pipeline,
            matrix_model_to_world,
            model,
            model_pipeline,
            needs_render: false,
            shading_mode: mode,
            world_arg_buffer,
            world_arg_ptr,
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
        let encoder = command_buffer.new_render_command_encoder(new_render_pass_descriptor(
            Some(render_target),
            self.depth_texture
                .as_ref()
                .map(|d| (d, MTLStoreAction::DontCare)),
        ));
        // Render Model
        {
            encoder.push_debug_group("Model");
            encoder.set_render_pipeline_state(&self.model_pipeline);
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
            encoder.set_render_pipeline_state(&self.light_pipeline);
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
        if let Some(update) = self.camera.on_event(event) {
            self.world_arg_ptr.camera_position = update.position_world.into();
            self.world_arg_ptr.matrix_screen_to_world = update.matrix_screen_to_world;
            self.world_arg_ptr.matrix_world_to_projection = update.matrix_world_to_projection;
            self.world_arg_ptr.matrix_model_to_projection =
                update.matrix_world_to_projection * self.matrix_model_to_world;
            self.needs_render = true;
        }

        if let Some(CameraUpdate { position_world, .. }) = self.light.on_event(event) {
            self.world_arg_ptr.light_position = position_world.into();
            self.needs_render = true;
        };

        if self.shading_mode.on_event(event) {
            (self.model_pipeline, self.light_pipeline) =
                create_pipelines(&self.device, &self.library, self.shading_mode);
            self.needs_render = true;
        }

        match event {
            UserEvent::WindowFocusedOrResized { size, .. } => {
                self.update_depth_texture_size(size);
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

impl<'a, const RENDER_LIGHT: bool> Delegate<'a, RENDER_LIGHT> {
    #[inline]
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
}

pub fn run() {
    launch_application::<Delegate<true>>("Project 4 - Textures");
}
