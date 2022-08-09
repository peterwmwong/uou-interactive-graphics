use metal::*;
use std::{mem::MaybeUninit, path::Path};

const BYTES_PER_PIXELS: u64 = 4; // RGBA
const COMPRESSION_METHOD: MTLIOCompressionMethod = MTLIOCompressionMethod::lz4;
const CUBE_ASSET_DIR_FILENAMES: [&'static str; 6] =
    ["posx", "negx", "posy", "negy", "posz", "negz"];
const CUBE_ASSET_DIR_METADATA: &'static str = "metadata.info";
const CUBE_TEXTURE_DEPTH: u64 = 1;
const SUPPORTED_PIXEL_FORMAT: MTLPixelFormat = MTLPixelFormat::RGBA8Unorm;
const SUPPORTED_INPUT_PIXEL_FORMAT: png::ColorType = png::ColorType::Rgba;

fn load_image_bytes_from_png<P: AsRef<Path>>(image_path: P) -> (Vec<u8>, (usize, usize)) {
    let decoder = png::Decoder::new(std::fs::File::open(&image_path).expect(&format!(
        "Could not open input PNG file ({:?}).",
        image_path.as_ref()
    )));
    let mut reader = decoder
        .read_info()
        .expect("Failed to decode PNG information");
    let info = reader.info();
    assert_eq!(
        info.color_type, SUPPORTED_INPUT_PIXEL_FORMAT,
        "Invalid cube texture PNG color format. Must be RGBA."
    );
    let (width, height) = (info.width, info.height);
    let mut buf = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut buf).expect(&format!(
        "Failed to load image data ({:?}) into buffer.",
        image_path.as_ref()
    ));
    (buf, (width as _, height as _))
}

pub fn create_cube_texture_asset_dir<P: AsRef<Path> + Send, P2: AsRef<Path> + Send>(
    target_dir: P,
    cube_face_files: &[P2; 6],
) {
    let mut all_face_sizes: [std::mem::MaybeUninit<(usize, usize)>; 6] =
        std::mem::MaybeUninit::uninit_array();
    std::thread::scope(|s| {
        for ((src_file, dest_file), face_size) in cube_face_files
            .iter()
            .map(|s| s.as_ref())
            .zip(
                CUBE_ASSET_DIR_FILENAMES
                    .iter()
                    .map(|a| target_dir.as_ref().join(a)),
            )
            .zip(&mut all_face_sizes)
        {
            s.spawn(move || {
                let (img_buf, size) = load_image_bytes_from_png(src_file);
                face_size.write(size);

                let io = IOCompression::new(
                    &dest_file.to_string_lossy(),
                    COMPRESSION_METHOD,
                    IOCompression::default_chunk_size(),
                );
                io.append(img_buf.as_ptr() as _, img_buf.len() as _);
                let io_flush_result = io.flush();
                assert_eq!(
                    io_flush_result,
                    MTLIOCompressionStatus::complete,
                    "Failed to write compressed file"
                );
            });
        }
    });

    // Verify all faces have the same size
    let mut cur_width = 0;
    let mut cur_height = 0;
    for (width, height) in unsafe { MaybeUninit::array_assume_init(all_face_sizes) } {
        assert!(
            ((cur_width == 0 && width > 0) || (cur_width == width))
                && ((cur_height == 0 && height > 0) || (cur_height == height)),
            "Width and Height of each cube face texture must be the same"
        );
        cur_width = width;
        cur_height = height;
    }
    let metadata = Metadata {
        width: cur_width as _,
        height: cur_height as _,
        pixel_format: SUPPORTED_PIXEL_FORMAT,
    };
    let metadata_ptr = &metadata as *const Metadata;
    let metadata_raw_bytes = unsafe {
        std::slice::from_raw_parts(metadata_ptr as *const u8, std::mem::size_of::<Metadata>())
    };
    std::fs::write(
        target_dir.as_ref().join(CUBE_ASSET_DIR_METADATA),
        metadata_raw_bytes,
    )
    .expect("Failed to write cube asset");
}

fn encode_load_cube_face_texture<P: AsRef<Path>>(
    device: &Device,
    command_buffer: &IOCommandBufferRef,
    cube_texture: &TextureRef,
    face: usize,
    width: u32,
    height: u32,
    cube_asset_face_file: P,
) {
    let handle = device
        .new_io_handle(
            URL::new_with_string(&format!(
                "file:///{}",
                cube_asset_face_file.as_ref().to_string_lossy()
            )),
            COMPRESSION_METHOD,
        )
        .expect("Failed to get IO file handle");
    let (width, height) = (width as u64, height as u64);
    let source_bytes_per_row = width * BYTES_PER_PIXELS;
    command_buffer.load_texture(
        cube_texture,
        face as _,
        0,
        MTLSize {
            width,
            height,
            depth: CUBE_TEXTURE_DEPTH,
        },
        source_bytes_per_row,
        height * source_bytes_per_row,
        MTLOrigin { x: 0, y: 0, z: 0 },
        &handle,
        0,
    );
}

#[repr(C)]
struct Metadata {
    width: u32,
    height: u32,
    pixel_format: MTLPixelFormat,
}

pub fn load_cube_texture_asset_dir<P: AsRef<Path>>(device: &Device, cube_asset_dir: P) -> Texture {
    let metadata_file = cube_asset_dir.as_ref().join(CUBE_ASSET_DIR_METADATA);
    let metadata_raw =
        std::fs::read(&metadata_file).expect("Failed to find/read cube asset's metadata file");
    assert_eq!(
        metadata_raw.len(),
        std::mem::size_of::<Metadata>(),
        "Cube asset's metadata file is invalid (size)"
    );
    let &Metadata {
        width,
        height,
        pixel_format,
    } = unsafe { &*(metadata_raw.as_ptr() as *const Metadata) };
    assert_eq!(
        pixel_format, SUPPORTED_PIXEL_FORMAT,
        "Unsupported cube asset's pixel format"
    );

    let queue = device
        .new_io_command_queue(&IOCommandQueueDescriptor::new())
        .expect("Failed to create IO Command Queue");
    let cube_texture = {
        let desc = TextureDescriptor::new();
        desc.set_pixel_format(pixel_format);
        desc.set_texture_type(MTLTextureType::Cube);
        desc.set_resource_options(
            MTLResourceOptions::StorageModeShared | MTLResourceOptions::CPUCacheModeWriteCombined,
        );
        desc.set_usage(MTLTextureUsage::ShaderRead);
        desc.set_width(width as _);
        desc.set_height(height as _);
        device.new_texture(&desc)
    };

    let command_buffer = queue.new_command_buffer();
    for (face, face_file) in CUBE_ASSET_DIR_FILENAMES.iter().enumerate() {
        encode_load_cube_face_texture(
            &device,
            &command_buffer,
            &cube_texture,
            face,
            width,
            height,
            cube_asset_dir.as_ref().join(face_file),
        );
    }
    command_buffer.commit();
    command_buffer.wait_until_completed();
    assert_eq!(
        command_buffer.status(),
        MTLIOStatus::complete,
        "Failed to load texture for face."
    );
    cube_texture
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;
    use std::time::Instant;

    const MICROS_PER_MILLI: u128 = 1000;
    const TEST_CUBE_TEXTURE_WIDTH: usize = 2048;
    const TEST_CUBE_TEXTURE_HEIGHT: usize = TEST_CUBE_TEXTURE_WIDTH;
    const TEST_CUBE_TEXTURE_BYTES_PER_IMAGE: usize =
        TEST_CUBE_TEXTURE_WIDTH * TEST_CUBE_TEXTURE_HEIGHT * (BYTES_PER_PIXELS as usize);
    const TEST_CUBE_TEXTURES: [&'static str; 6] = [
        "cubemap_posx.png",
        "cubemap_negx.png",
        "cubemap_posy.png",
        "cubemap_negy.png",
        "cubemap_posz.png",
        "cubemap_negz.png",
    ];

    fn debug_time<T>(label: &'static str, f: impl FnOnce() -> T) -> T {
        #[cfg(debug_assertions)]
        {
            let now = Instant::now();
            let r = f();
            let elapsed = now.elapsed();
            let elapsed_micro = elapsed.as_micros();
            let (elapsed_display, unit) = if elapsed_micro > MICROS_PER_MILLI {
                (elapsed_micro / MICROS_PER_MILLI, "ms")
            } else {
                (elapsed_micro, "μ")
            };
            println!("[{label:<40}] {:>6} {}", elapsed_display, unit);
            return r;
        }
        #[cfg(not(debug_assertions))]
        {
            return f();
        }
    }

    #[test]
    fn test() {
        let asset_dir_name = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("Failed to get epoch time (for temp asset directory)")
            .as_millis()
            .to_string();
        let asset_dir = std::env::temp_dir().join(asset_dir_name);
        std::fs::create_dir(&asset_dir).expect("Failed to create temp asset directory");
        let cube_textures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test-assets")
            .join("cube-textures");
        let test_cube_texture_files = TEST_CUBE_TEXTURES.map(|f| cube_textures_dir.join(f));
        debug_time("Create Asset", || {
            create_cube_texture_asset_dir(&asset_dir, &test_cube_texture_files)
        });
        let device = Device::system_default().expect("Failed to access a Metal Device");
        let texture = debug_time("Load Asset", || {
            load_cube_texture_asset_dir(&device, &asset_dir)
        });

        assert_eq!(texture.width(), TEST_CUBE_TEXTURE_WIDTH as _);
        assert_eq!(texture.height(), TEST_CUBE_TEXTURE_HEIGHT as _);

        debug_time("Verified Cube Texture Faces", || {
            std::thread::scope(|s| {
                // As we are only reading from the Metal Cube Texture (slice for each thread), this
                // should be a thread-safe operation.
                #[derive(Clone, Copy)]
                struct SendTexture<'a>(&'a TextureRef);
                impl<'a> SendTexture<'a> {
                    #[inline]
                    pub fn get_bytes_in_slice(
                        &self,
                        bytes: *mut std::ffi::c_void,
                        stride: NSUInteger,
                        image_stride: NSUInteger,
                        region: MTLRegion,
                        mipmap_level: NSUInteger,
                        slice: NSUInteger,
                    ) {
                        self.0.get_bytes_in_slice(
                            bytes,
                            stride,
                            image_stride,
                            region,
                            mipmap_level,
                            slice,
                        )
                    }
                    #[inline]
                    pub fn size(&self) -> MTLSize {
                        MTLSize {
                            width: self.0.width(),
                            height: self.0.height(),
                            depth: self.0.depth(),
                        }
                    }
                }
                unsafe impl<'a> Send for SendTexture<'a> {}

                for (face, face_file) in test_cube_texture_files.iter().enumerate() {
                    let texture_ref = SendTexture(&texture);
                    s.spawn(move || {
                        let mut actual_texture_bytes = vec![0; TEST_CUBE_TEXTURE_BYTES_PER_IMAGE];
                        texture_ref.get_bytes_in_slice(
                            actual_texture_bytes.as_mut_ptr() as _,
                            TEST_CUBE_TEXTURE_WIDTH as u64 * BYTES_PER_PIXELS as u64,
                            TEST_CUBE_TEXTURE_BYTES_PER_IMAGE as u64 * BYTES_PER_PIXELS as u64,
                            MTLRegion {
                                origin: MTLOrigin { x: 0, y: 0, z: 0 },
                                size: texture_ref.size(),
                            },
                            0,
                            face as _,
                        );
                        let (expected_texture_bytes, ..) = load_image_bytes_from_png(face_file);

                        if &actual_texture_bytes != &expected_texture_bytes {
                            println!(
                                "Cube texture face #{} contents are incorrect: {:?} {:?}",
                                face,
                                &actual_texture_bytes[0..4],
                                &expected_texture_bytes[0..4],
                            );
                        }
                    });
                }
            });
        })
    }
}
