#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::{Camera, CameraUpdate, DepthTexture, ShadingModeSelector},
    metal::*,
    metal_types::*,
    render_pipeline::{BindOne, BlendMode, HasDepth, NoBinds, NoStencil, RenderPipeline},
    *,
};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    ops::Neg,
    path::PathBuf,
    simd::{f32x2, f32x4},
};

const DEFAULT_AMBIENT_AMOUNT: f32 = 0.15;
const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 6., 0.]);
const INITIAL_LIGHT_ROTATION: f32x2 = f32x2::from_array([-PI / 4., 0.]);
const INITIAL_MODE: ShadingModeSelector = ShadingModeSelector::DEFAULT;
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));
const LIGHT_DISTANCE: f32 = 0.5;

pub struct Delegate<const RENDER_LIGHT: bool> {
    camera: Camera,
    camera_space: ProjectedSpace,
    command_queue: CommandQueue,
    depth_state: DepthStencilState,
    depth_texture: DepthTexture,
    library: Library,
    light_pipeline: RenderPipeline<1, light_vertex, light_fragment, HasDepth, NoStencil>,
    light: Camera,
    light_position: float4,
    matrix_model_to_world: f32x4x4,
    model_pipeline: RenderPipeline<1, main_vertex, main_fragment, HasDepth, NoStencil>,
    model: Model<Geometry, HasMaterial<Material>>,
    needs_render: bool,
    device: Device,
    shading_mode: ShadingModeSelector,
}

fn create_model_pipeline(
    device: &Device,
    library: &Library,
    mode: ShadingModeSelector,
) -> RenderPipeline<1, main_vertex, main_fragment, HasDepth, NoStencil> {
    RenderPipeline::new(
        "Plane",
        &device,
        &library,
        [(DEFAULT_PIXEL_FORMAT, BlendMode::NoBlend)],
        main_vertex,
        main_fragment {
            HasAmbient: mode.contains(ShadingModeSelector::HAS_AMBIENT),
            HasDiffuse: mode.contains(ShadingModeSelector::HAS_DIFFUSE),
            OnlyNormals: mode.contains(ShadingModeSelector::ONLY_NORMALS),
            HasSpecular: mode.contains(ShadingModeSelector::HAS_SPECULAR),
        },
        HasDepth(DEPTH_TEXTURE_FORMAT),
        NoStencil,
    )
}

impl<const RENDER_LIGHT: bool> RendererDelgate for Delegate<RENDER_LIGHT> {
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
        let model_pipeline = create_model_pipeline(&device, &library, mode);
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
            HasMaterial::new(
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
            ),
        );
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

        Self {
            camera: Camera::new_with_default_distance(
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            camera_space: ProjectedSpace {
                matrix_world_to_projection: f32x4x4::identity().into(),
                matrix_screen_to_world: f32x4x4::identity().into(),
                position_world: f32x4::default().into(),
            },
            command_queue: device.new_command_queue(),
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: DepthTexture::new("Depth", DEPTH_TEXTURE_FORMAT),
            light_position: f32x4::default().into(),
            light: Camera::new(
                LIGHT_DISTANCE,
                INITIAL_LIGHT_ROTATION,
                ModifierKeys::CONTROL,
                true,
                0.,
            ),
            light_pipeline: RenderPipeline::new(
                "Light",
                &device,
                &library,
                [(DEFAULT_PIXEL_FORMAT, BlendMode::NoBlend)],
                light_vertex,
                light_fragment,
                HasDepth(DEPTH_TEXTURE_FORMAT),
                NoStencil,
            ),
            matrix_model_to_world,
            model,
            model_pipeline,
            needs_render: false,
            shading_mode: mode,
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
        let encoder = self.model_pipeline.new_render_command_encoder(
            "Render Encoder",
            command_buffer,
            [(
                render_target,
                (0., 0., 0., 0.),
                MTLLoadAction::Clear,
                MTLStoreAction::Store,
            )],
            (
                self.depth_texture.texture(),
                1.,
                MTLLoadAction::Clear,
                MTLStoreAction::DontCare,
            ),
            NoStencil,
        );
        self.model.encode_use_resources(&encoder);
        encoder.set_depth_stencil_state(&self.depth_state);
        // Render Model
        {
            encoder.push_debug_group("Model");
            self.model_pipeline.setup_binds(
                encoder,
                main_vertex_binds {
                    geometry: BindOne::Skip,
                    model: BindOne::Bytes(&ModelSpace {
                        matrix_model_to_projection: self.camera_space.matrix_world_to_projection
                            * self.matrix_model_to_world,
                        // IMPORTANT: Not a mistake, using Model-to-World Rotation 4x4 Matrix for
                        // Normal-to-World 3x3 Matrix. Conceptually, we want a matrix that ONLY applies rotation
                        // (no translation). Since normals are directions (not positions, relative to a
                        // point on a surface), translations are meaningless.
                        matrix_normal_to_world: self.matrix_model_to_world.into(),
                    }),
                },
                main_fragment_binds {
                    material: BindOne::Skip,
                    camera: BindOne::Bytes(&self.camera_space),
                    light_pos: BindOne::Bytes(&self.light_position),
                },
            );
            for (
                i,
                DrawIteratorItem {
                    num_vertices,
                    geometry,
                    material,
                },
            ) in self.model.get_draws().enumerate()
            {
                self.model_pipeline.setup_binds(
                    encoder,
                    main_vertex_binds {
                        geometry: BindOne::buffer_with_rolling_offset(geometry),
                        model: BindOne::Skip,
                    },
                    main_fragment_binds {
                        material: { BindOne::iterating_buffer_offset(i, material) },
                        camera: BindOne::Skip,
                        light_pos: BindOne::Skip,
                    },
                );
                encoder.draw_primitives(MTLPrimitiveType::Triangle, 0, num_vertices as _);
            }
            encoder.pop_debug_group();
        }
        // Render Light
        if RENDER_LIGHT {
            encoder.push_debug_group("Light");
            encoder.set_render_pipeline_state(&self.light_pipeline.pipeline);
            self.light_pipeline.setup_binds(
                encoder,
                light_vertex_binds {
                    camera: BindOne::Bytes(&self.camera_space),
                    light_pos: BindOne::Bytes(&self.light_position),
                },
                NoBinds,
            );
            encoder.draw_primitives(MTLPrimitiveType::Point, 0, 1);
            encoder.pop_debug_group();
        }
        encoder.end_encoding();
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        if let Some(update) = self.camera.on_event(event) {
            self.camera_space.position_world = update.position_world.into();
            self.camera_space.matrix_screen_to_world = update.matrix_screen_to_world;
            self.camera_space.matrix_world_to_projection = update.matrix_world_to_projection;
            self.needs_render = true;
        }
        if let Some(CameraUpdate { position_world, .. }) = self.light.on_event(event) {
            self.light_position = position_world.into();
            self.needs_render = true;
        };
        if self.shading_mode.on_event(event) {
            self.model_pipeline =
                create_model_pipeline(&self.device, &self.library, self.shading_mode);
            self.needs_render = true;
        }
        if self.depth_texture.on_event(event, &self.device) {
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

pub fn run() {
    launch_application::<Delegate<true>>("Project 4 - Textures");
}
