#![feature(array_zip)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{components::camera, metal::*, metal_types::*, *};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    fs,
    ops::Neg,
    path::{Path, PathBuf},
    simd::f32x2,
};

const DEPTH_TEXTURE_FORMAT: MTLPixelFormat = MTLPixelFormat::Depth16Unorm;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([-PI / 32., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

struct Delegate<'a> {
    bg_pipeline_state: RenderPipelineState,
    bg_depth_state: DepthStencilState,
    camera: camera::Camera,
    command_queue: CommandQueue,
    cubemap_texture: Texture,
    depth_state: DepthStencilState,
    depth_texture: Option<Texture>,
    device: Device,
    matrix_model_to_world: f32x4x4,
    needs_render: bool,
    model_pipeline_state: RenderPipelineState,
    model: Model<{ VertexBufferIndex::Geometry as _ }, { NO_MATERIALS_ID }>,
    world_arg_buffer: Buffer,
    world_arg_ptr: &'a mut World,
}

const CUBEMAP_TEXTURE_BYTES_PER_PIXEL: u32 = 4; // Assumed to be 4-component (ex. RGBA)
const CUBEMAP_TEXTURE_WIDTH: u32 = 2048;
const CUBEMAP_TEXTURE_HEIGHT: u32 = CUBEMAP_TEXTURE_WIDTH;
const CUBEMAP_TEXTURE_BYTES_PER_ROW: u32 = CUBEMAP_TEXTURE_WIDTH * CUBEMAP_TEXTURE_BYTES_PER_PIXEL;
const CUBEMAP_TEXTURE_BYTES_PER_FACE: u32 = CUBEMAP_TEXTURE_HEIGHT * CUBEMAP_TEXTURE_BYTES_PER_ROW;

fn read_png_pixel_bytes_into<P: AsRef<Path>>(path_to_png: P, mut buffer: &mut Vec<u8>) -> usize {
    let mut decoder =
        png::Decoder::new(fs::File::open(&path_to_png).expect("Could not open input PNG file."));
    decoder.set_transformations(png::Transformations::normalize_to_color8());

    let mut reader = decoder.read_info().expect("Could not read input PNG file.");
    let info = reader.info();
    assert!(
        info.trns.is_none(),
        "input PNG file contains unsupported tRNS"
    );
    let &png::Info {
        width,
        height,
        color_type,
        ..
    } = info;

    assert_eq!(width, CUBEMAP_TEXTURE_WIDTH);
    assert_eq!(height, CUBEMAP_TEXTURE_HEIGHT);
    assert!(
        (color_type == png::ColorType::Rgba),
        "Unexpected input PNG file color format, expected RGB or RGBA"
    );

    let size = reader.output_buffer_size();
    buffer.resize(size, 0);
    reader
        .next_frame(&mut buffer)
        .expect("Could not read image data from input PNG file.");
    size
}

impl<'a> RendererDelgate for Delegate<'a> {
    fn new(device: Device) -> Self {
        let executable_name = std::env::args()
            .nth(0)
            .expect("Failed to access command line executable name");
        let model_file_path = std::env::args().nth(1).expect(&format!(
            "Usage: {executable_name} [Path to Wavefront OBJ file]"
        ));

        // Load Environment Map (Cube Map)
        let cubemap_texture = {
            let desc = TextureDescriptor::new();
            desc.set_pixel_format(MTLPixelFormat::RGBA8Unorm);
            desc.set_texture_type(MTLTextureType::Cube);
            desc.set_compression_type(MTLTextureCompressionType::Lossless);
            desc.set_resource_options(DEFAULT_RESOURCE_OPTIONS);
            desc.set_usage(MTLTextureUsage::ShaderRead);
            // TODO: Remove hardcoded values, use PNG dimensions
            desc.set_width(CUBEMAP_TEXTURE_WIDTH as _);
            desc.set_height(CUBEMAP_TEXTURE_HEIGHT as _);
            desc.set_depth(1);

            let texture = device.new_texture(&desc);
            let cubemap_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("cubemap");
            let mut buffer = vec![];
            for (face_index, filename) in [
                "cubemap_posx.png",
                "cubemap_negx.png",
                "cubemap_posy.png",
                "cubemap_negy.png",
                "cubemap_posz.png",
                "cubemap_negz.png",
            ]
            .iter()
            .enumerate()
            {
                let bytes = read_png_pixel_bytes_into(cubemap_path.join(filename), &mut buffer);
                assert_eq!(
                    bytes, CUBEMAP_TEXTURE_BYTES_PER_FACE as _,
                    "Unexpected number of bytes read for cube map texture"
                );
                texture.replace_region_in_slice(
                    MTLRegion {
                        origin: MTLOrigin { x: 0, y: 0, z: 0 },
                        size: MTLSize {
                            width: CUBEMAP_TEXTURE_WIDTH as _,
                            height: CUBEMAP_TEXTURE_WIDTH as _,
                            depth: 1,
                        },
                    },
                    0,
                    face_index as _,
                    buffer.as_ptr() as _,
                    CUBEMAP_TEXTURE_BYTES_PER_ROW as _,
                    CUBEMAP_TEXTURE_BYTES_PER_FACE as _,
                );
            }
            texture
        };

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
        let bg_pipeline = create_pipeline(
            &device,
            &library,
            &new_basic_render_pipeline_descriptor(
                DEFAULT_PIXEL_FORMAT,
                Some(DEPTH_TEXTURE_FORMAT),
                false,
            ),
            "BG",
            None,
            &"bg_vertex",
            0,
            &"bg_fragment",
            BGFragBufferIndex::LENGTH as _,
        );
        debug_assert_argument_buffer_size::<{ BGFragBufferIndex::World as _ }, World>(
            &bg_pipeline,
            FunctionType::Fragment,
        );
        let model_pipeline = create_pipeline(
            &device,
            &library,
            &new_basic_render_pipeline_descriptor(
                DEFAULT_PIXEL_FORMAT,
                Some(DEPTH_TEXTURE_FORMAT),
                false,
            ),
            "Model",
            None,
            &"main_vertex",
            VertexBufferIndex::LENGTH as _,
            &"main_fragment",
            FragBufferIndex::LENGTH as _,
        );
        debug_assert_argument_buffer_size::<{ VertexBufferIndex::Geometry as _ }, Geometry>(
            &model_pipeline,
            FunctionType::Vertex,
        );
        debug_assert_argument_buffer_size::<{ VertexBufferIndex::World as _ }, World>(
            &model_pipeline,
            FunctionType::Vertex,
        );
        debug_assert_argument_buffer_size::<{ FragBufferIndex::World as _ }, World>(
            &model_pipeline,
            FunctionType::Fragment,
        );
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
        let model_to_world_scale_rot = f32x4x4::scale(scale, scale, scale, 1.)
            * (f32x4x4::y_rotate(PI) * f32x4x4::x_rotate(PI / 2.));
        let matrix_model_to_world = model_to_world_scale_rot * f32x4x4::translate(cx, cy, cz);

        // IMPORTANT: Not a mistake, using Model-to-World Rotation 4x4 Matrix for
        // Normal-to-World 3x3 Matrix. Conceptually, we want a matrix that ONLY applies rotation
        // (no translation). Since normals are directions (not positions, relative to a
        // point on a surface), translations are meaningless.
        world_arg_ptr.matrix_normal_to_world = matrix_model_to_world.into();

        Self {
            camera: camera::Camera::new(
                (model_to_world_scale_rot * size).abs(),
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
            ),
            cubemap_texture,
            bg_depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::Always);
                desc.set_depth_write_enabled(false);
                device.new_depth_stencil_state(&desc)
            },
            depth_state: {
                let desc = DepthStencilDescriptor::new();
                desc.set_depth_compare_function(MTLCompareFunction::LessEqual);
                desc.set_depth_write_enabled(true);
                device.new_depth_stencil_state(&desc)
            },
            depth_texture: None,
            matrix_model_to_world,
            model,
            model_pipeline_state: model_pipeline.pipeline_state,
            bg_pipeline_state: bg_pipeline.pipeline_state,
            world_arg_buffer,
            world_arg_ptr,
            needs_render: false,
            command_queue: device.new_command_queue(),
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
        // Render Model
        {
            encoder.push_debug_group("Model");
            self.model.encode_use_resources(encoder);
            encoder.set_render_pipeline_state(&self.bg_pipeline_state);
            encoder.set_depth_stencil_state(&self.bg_depth_state);
            encoder.set_fragment_buffer(
                BGFragBufferIndex::World as _,
                Some(&self.world_arg_buffer),
                0,
            );
            encoder.set_fragment_texture(
                BGFragTextureIndex::CubeMapTexture as _,
                Some(&self.cubemap_texture),
            );
            encoder.draw_primitives(MTLPrimitiveType::TriangleStrip, 0, 4);
            // TODO: Temporarily disabled to focus on background
            // encoder.set_render_pipeline_state(&self.model_pipeline_state);
            // encoder.set_depth_stencil_state(&self.depth_state);
            // encoder.set_vertex_buffer(
            //     VertexBufferIndex::World as _,
            //     Some(&self.world_arg_buffer),
            //     0,
            // );
            // encoder.set_fragment_buffer(
            //     FragBufferIndex::World as _,
            //     Some(&self.world_arg_buffer),
            //     0,
            // );
            // self.model.encode_draws(encoder);
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
                self.world_arg_ptr.screen_size = size.into();
                self.update_depth_texture_size(size);
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
    launch_application::<Delegate>("Project 6 - Environment Mapping");
}
