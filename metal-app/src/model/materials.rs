use crate::{
    align_size,
    metal::*,
    objc_sendmsg_with_cached_sel,
    typed_buffer::{TypedBuffer, TypedBufferSizer},
    MetalGPUAddress, DEFAULT_RESOURCE_OPTIONS,
};
use std::{collections::HashMap, ops::Deref, path::Path};

type RGB32 = [f32; 3];

pub struct MaterialToEncode {
    pub ambient_texture: MetalGPUAddress,
    pub diffuse_texture: MetalGPUAddress,
    pub specular_texture: MetalGPUAddress,
    pub specular_shineness: f32,
}

pub(crate) struct MaterialResults<T: Sized + Copy + Clone> {
    pub(crate) arguments_buffer: TypedBuffer<T>,
    // Needs to be owned and not dropped (causing deallocation from heap).
    _textures: Vec<Texture>,
}

#[derive(Hash, Copy, Clone, Eq, PartialEq)]
struct RGB8Unorm(u8, u8, u8);

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum MaterialSourceKey<'a> {
    PNG(&'a str),
    Color(RGB8Unorm),
}

impl<'a> MaterialSourceKey<'a> {
    #[inline(always)]
    fn new(png_file: &'a str, color: &RGB32) -> Self {
        if png_file.is_empty() {
            Self::Color(RGB8Unorm(
                (color[0] * 255.0).round() as u8,
                (color[1] * 255.0).round() as u8,
                (color[2] * 255.0).round() as u8,
            ))
        } else {
            Self::PNG(png_file)
        }
    }
}

struct MaterialSource<'a> {
    key: MaterialSourceKey<'a>,
    texture_descriptor: TextureDescriptor,
    png_reader: Option<png::Reader<std::fs::File>>,
    load_texture_buffer_size: usize,
}

impl<'a> MaterialSource<'a> {
    fn new<'b, P: AsRef<Path>>(png_file_dir: P, key: MaterialSourceKey<'a>) -> Self {
        let (width, height, load_texture_buffer_size, png_reader) = match key {
            MaterialSourceKey::PNG(png_file) => {
                let decoder = png::Decoder::new(
                    std::fs::File::open(png_file_dir.as_ref().join(png_file)).unwrap(),
                );
                let reader = decoder.read_info().unwrap();
                let info = reader.info();
                let load_texture_buffer_size = reader.output_buffer_size();
                assert_eq!(
                    info.color_type,
                    png::ColorType::Rgba,
                    r#"Unexpected PNG color format, expected RGBA.
Images with RGB color format (no alpha), can be preprocessed by running:
> cargo run --bin png-add-alpha [path to image]
"#
                );
                (
                    info.width as _,
                    info.height as _,
                    load_texture_buffer_size,
                    Some(reader),
                )
            }
            MaterialSourceKey::Color(_) => (1, 1, 0, None),
        };

        let desc = TextureDescriptor::new();
        desc.set_width(width);
        desc.set_height(height);
        desc.set_pixel_format(MTLPixelFormat::RGBA8Unorm);
        desc.set_storage_mode(MTLStorageMode::Shared);
        desc.set_texture_type(MTLTextureType::D2);
        desc.set_usage(MTLTextureUsage::ShaderRead);
        desc.set_resource_options(DEFAULT_RESOURCE_OPTIONS);
        Self {
            key,
            texture_descriptor: desc,
            png_reader,
            load_texture_buffer_size,
        }
    }

    #[inline]
    fn size_and_padding(&self, device: &Device) -> (usize, usize) {
        let size_align = device.heap_texture_size_and_align(&self.texture_descriptor);
        let aligned_size = align_size(size_align);
        let unaligned_size = size_align.size as usize;
        debug_assert!(aligned_size >= unaligned_size);
        (unaligned_size, aligned_size - unaligned_size)
    }

    // `read_png_buffer` allows single allocation/deallocation of the temp buffer for reading
    // all a model's material PNG textures.
    #[inline]
    fn allocate_texture(&mut self, heap: &Heap, read_png_buffer: &mut [u8]) -> Texture {
        let tmp_color_label;
        let tmp_color_buf;
        let (buf, label): (&[u8], &str) = match self.key {
            MaterialSourceKey::PNG(png_file) => {
                let reader = self
                    .png_reader
                    .as_mut()
                    .expect("png_reader is unexpectedely None, eventhough material texture is PNG");
                reader
                    .next_frame(read_png_buffer)
                    .expect("Failed to load texture");
                (read_png_buffer, png_file)
            }
            MaterialSourceKey::Color(RGB8Unorm(r, g, b)) => {
                tmp_color_label = format!("Color {r},{g},{b}");
                tmp_color_buf = [r, g, b];
                (&tmp_color_buf, &tmp_color_label)
            }
        };

        let texture = heap
            .new_texture(&self.texture_descriptor)
            .expect(&format!("Failed to allocate texture for {label}"));
        texture.set_label(label);

        const BYTES_PER_RGBA_PIXEL: NSUInteger = 4;
        texture.replace_region(
            MTLRegion {
                origin: MTLOrigin { x: 0, y: 0, z: 0 },
                size: MTLSize {
                    width: self.texture_descriptor.width(),
                    height: self.texture_descriptor.height(),
                    depth: 1,
                },
            },
            0,
            buf.as_ptr() as _,
            self.texture_descriptor.width() * BYTES_PER_RGBA_PIXEL,
        );
        texture
    }
}

struct Material<'a> {
    ambient: MaterialSourceKey<'a>,
    diffuse: MaterialSourceKey<'a>,
    specular: MaterialSourceKey<'a>,
    specular_shineness: f32,
}

pub(crate) struct Materials<'a, T: Sized + Copy + Clone> {
    arguments_sizer: TypedBufferSizer<T>,
    heap_size: usize,
    materials: Vec<Material<'a>>,
    max_load_texture_buffer_size: usize,
    sources: HashMap<MaterialSourceKey<'a>, MaterialSource<'a>>,
}

impl<'a, T: Sized + Copy + Clone> Materials<'a, T> {
    pub(crate) fn new<P: AsRef<Path>>(
        device: &Device,
        material_file_dir: P,
        obj_mats: &'a [tobj::Material],
    ) -> Self {
        let num_materials = obj_mats.len();
        let mut sources = HashMap::with_capacity(num_materials * 3);
        let mut heap_size = 0;
        let mut last_alignment_padding = 0;
        let mut max_load_texture_buffer_size = 0;
        let materials: Vec<Material<'a>> = obj_mats
            .iter()
            .map(|mat| {
                let m = Material {
                    ambient: MaterialSourceKey::new(&mat.ambient_texture, &mat.ambient),
                    diffuse: MaterialSourceKey::new(&mat.diffuse_texture, &mat.diffuse),
                    specular: MaterialSourceKey::new(&mat.specular_texture, &mat.specular),
                    specular_shineness: mat.shininess,
                };
                for key in [&m.ambient, &m.diffuse, &m.specular] {
                    sources.entry(*key).or_insert_with(|| {
                        let mat_tx = MaterialSource::new(&material_file_dir, *key);
                        max_load_texture_buffer_size =
                            max_load_texture_buffer_size.max(mat_tx.load_texture_buffer_size);
                        let (size, padding) = mat_tx.size_and_padding(device);
                        heap_size += last_alignment_padding + size;
                        last_alignment_padding = padding;
                        mat_tx
                    });
                }
                m
            })
            .collect();

        let arguments_sizer = TypedBufferSizer::new(materials.len(), DEFAULT_RESOURCE_OPTIONS);
        heap_size += arguments_sizer.heap_aligned_byte_size(device);
        Self {
            arguments_sizer,
            heap_size,
            materials,
            max_load_texture_buffer_size,
            sources,
        }
    }

    #[inline]
    pub fn heap_size(&self) -> usize {
        self.heap_size
    }

    pub fn allocate_and_encode(
        &mut self,
        heap: &Heap,
        mut encode_arg: impl FnMut(&mut T, MaterialToEncode),
    ) -> MaterialResults<T> {
        let arguments_buffer = self.arguments_sizer.allocate("Materials", heap.deref());
        let arguments = arguments_buffer.get_mut();

        let gpu_handle_sel = sel!(gpuHandle);
        let mut texture_cache: HashMap<MaterialSourceKey<'a>, Texture> =
            HashMap::with_capacity(self.sources.len());
        let mut read_png_buffer: Vec<u8> = Vec::with_capacity(self.max_load_texture_buffer_size);
        unsafe { read_png_buffer.set_len(self.max_load_texture_buffer_size) };
        for (i, mat) in self.materials.iter().enumerate() {
            let [ambient_texture, diffuse_texture, specular_texture] =
                [mat.ambient, mat.diffuse, mat.specular].map(|source_key| {
                    let texture: &TextureRef =
                        texture_cache.entry(source_key).or_insert_with(|| {
                            self.sources
                                .get_mut(&source_key)
                                .expect("Couldn't find source key")
                                .allocate_texture(heap, &mut read_png_buffer)
                        });
                    unsafe { objc_sendmsg_with_cached_sel(texture, gpu_handle_sel) }
                });
            encode_arg(
                &mut arguments[i],
                MaterialToEncode {
                    ambient_texture,
                    diffuse_texture,
                    specular_texture,
                    specular_shineness: mat.specular_shineness,
                },
            );
        }
        let textures = texture_cache.into_iter().map(|(_, tx)| tx).collect();
        MaterialResults {
            arguments_buffer,
            _textures: textures,
        }
    }
}
