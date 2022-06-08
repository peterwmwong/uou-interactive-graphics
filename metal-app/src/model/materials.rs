use super::heap_resident::HeapResident;
use crate::{align_size, metal::*, DEFAULT_RESOURCE_OPTIONS};
use std::{collections::HashMap, path::PathBuf};

type RGB32 = [f32; 3];

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
}

impl<'a> MaterialSource<'a> {
    fn new<'b>(png_file_dir: &'b PathBuf, key: MaterialSourceKey<'a>) -> Self {
        let (width, height, png_reader) = match key {
            MaterialSourceKey::PNG(png_file) => {
                let decoder =
                    png::Decoder::new(std::fs::File::open(png_file_dir.join(png_file)).unwrap());
                let reader = decoder.read_info().unwrap();
                let info = reader.info();
                assert_eq!(
                    info.color_type,
                    png::ColorType::Rgba,
                    "Unexpected PNG color format, expected RGBA"
                );
                (info.width as _, info.height as _, Some(reader))
            }
            MaterialSourceKey::Color(_) => (1, 1, None),
        };

        let desc = TextureDescriptor::new();
        desc.set_width(width as _);
        desc.set_height(height as _);
        desc.set_pixel_format(MTLPixelFormat::RGBA8Unorm);
        desc.set_storage_mode(MTLStorageMode::Shared);
        desc.set_texture_type(MTLTextureType::D2);
        desc.set_usage(MTLTextureUsage::ShaderRead);
        desc.set_resource_options(DEFAULT_RESOURCE_OPTIONS);
        Self {
            key,
            texture_descriptor: desc,
            png_reader,
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

    #[inline]
    fn allocate_texture(&mut self, heap: &Heap) -> Texture {
        // TODO: Allocate this buf once
        // - Get the maximum output_buffer_size() of all the textures
        let tmp_color_label;
        let (buf, label): (Vec<u8>, &str) = match self.key {
            MaterialSourceKey::PNG(png_file) => {
                let reader = self
                    .png_reader
                    .as_mut()
                    .expect("png_reader is unexpectedely None, eventhough material texture is PNG");
                let buf_size = reader.output_buffer_size();
                let mut buf = Vec::with_capacity(buf_size);
                unsafe { buf.set_len(buf_size) };
                reader.next_frame(&mut buf).expect("Failed to load texture");
                (buf, png_file)
            }
            MaterialSourceKey::Color(RGB8Unorm(r, g, b)) => {
                tmp_color_label = format!("Color {r},{g},{b}");
                (vec![r, g, b], &tmp_color_label)
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

pub(crate) struct Materials<
    'a,
    const MATERIAL_ID_AMBIENT_TEXTURE: u16,
    const MATERIAL_ID_DIFFUSE_TEXTURE: u16,
    const MATERIAL_ID_SPECULAR_TEXTURE: u16,
    const MATERIAL_ID_SPECULAR_SHINENESS: u16,
> {
    sources: HashMap<MaterialSourceKey<'a>, MaterialSource<'a>>,
    materials: Vec<Material<'a>>,
    heap_size: usize,
}

impl<
        'a,
        const MATERIAL_ID_AMBIENT_TEXTURE: u16,
        const MATERIAL_ID_DIFFUSE_TEXTURE: u16,
        const MATERIAL_ID_SPECULAR_TEXTURE: u16,
        const MATERIAL_ID_SPECULAR_SHINENESS: u16,
    >
    Materials<
        'a,
        MATERIAL_ID_AMBIENT_TEXTURE,
        MATERIAL_ID_DIFFUSE_TEXTURE,
        MATERIAL_ID_SPECULAR_TEXTURE,
        MATERIAL_ID_SPECULAR_SHINENESS,
    >
{
    pub(crate) fn new<'b>(
        device: &Device,
        material_file_dir: &'b PathBuf,
        obj_mats: &'a [tobj::Material],
    ) -> Self {
        assert!(
            MATERIAL_ID_AMBIENT_TEXTURE != MATERIAL_ID_DIFFUSE_TEXTURE
                && MATERIAL_ID_AMBIENT_TEXTURE != MATERIAL_ID_SPECULAR_TEXTURE
                && MATERIAL_ID_AMBIENT_TEXTURE != MATERIAL_ID_SPECULAR_SHINENESS
                && MATERIAL_ID_DIFFUSE_TEXTURE != MATERIAL_ID_SPECULAR_TEXTURE
                && MATERIAL_ID_DIFFUSE_TEXTURE != MATERIAL_ID_SPECULAR_SHINENESS
                && MATERIAL_ID_SPECULAR_TEXTURE != MATERIAL_ID_SPECULAR_SHINENESS,
            r#"Material ID constants (Metal Shader [[id(...)]] argument bindings) must all be unique.
Check the following generic constants passed to Model::from_file()...
- MATERIAL_ID_AMBIENT_TEXTURE
- MATERIAL_ID_DIFFUSE_TEXTURE
- MATERIAL_ID_SPECULAR_TEXTURE
- MATERIAL_ID_SPECULAR_SHINENESS
"#
        );
        let num_materials = obj_mats.len();
        let mut sources = HashMap::with_capacity(num_materials * 3);
        let mut heap_size = 0;
        let mut last_alignment_padding = 0;
        let materials = obj_mats
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
                        let mat_tx = MaterialSource::new(material_file_dir, *key);
                        let (size, padding) = mat_tx.size_and_padding(device);
                        heap_size += last_alignment_padding + size;
                        last_alignment_padding = padding;
                        mat_tx
                    });
                }
                m
            })
            .collect();
        Self {
            sources,
            materials,
            heap_size,
        }
    }
}

impl<
        'a,
        const MATERIAL_ID_AMBIENT_TEXTURE: u16,
        const MATERIAL_ID_DIFFUSE_TEXTURE: u16,
        const MATERIAL_ID_SPECULAR_TEXTURE: u16,
        const MATERIAL_ID_SPECULAR_SHINENESS: u16,
    > HeapResident<Vec<Texture>>
    for Materials<
        'a,
        MATERIAL_ID_AMBIENT_TEXTURE,
        MATERIAL_ID_DIFFUSE_TEXTURE,
        MATERIAL_ID_SPECULAR_TEXTURE,
        MATERIAL_ID_SPECULAR_SHINENESS,
    >
{
    #[inline]
    fn heap_size(&self) -> usize {
        self.heap_size
    }

    fn allocate_and_encode(
        &mut self,
        heap: &Heap,
        device: &Device,
        arg_encoder: &ArgumentEncoder,
    ) -> (Buffer, u32, Vec<Texture>) {
        let num_materials = self.materials.len();
        let arg_encoded_length = arg_encoder.encoded_length() as u32;
        let buffer = device.new_buffer(
            (arg_encoded_length as u64) * num_materials as u64,
            DEFAULT_RESOURCE_OPTIONS,
        );
        buffer.set_label("Materials Argument Buffer");

        let mut texture_cache: HashMap<MaterialSourceKey<'a>, Texture> =
            HashMap::with_capacity(self.sources.len());
        for (i, mat) in self.materials.iter().enumerate() {
            arg_encoder.set_argument_buffer_to_element(i as _, &buffer, 0);
            for (id, source_key) in [
                (MATERIAL_ID_AMBIENT_TEXTURE, mat.ambient),
                (MATERIAL_ID_DIFFUSE_TEXTURE, mat.diffuse),
                (MATERIAL_ID_SPECULAR_TEXTURE, mat.specular),
            ] {
                arg_encoder.set_texture(
                    id as _,
                    texture_cache.entry(source_key).or_insert_with(|| {
                        self.sources
                            .get_mut(&source_key)
                            .expect("Couldn't find source key")
                            .allocate_texture(heap)
                    }),
                );
            }
            unsafe {
                *(arg_encoder.constant_data(MATERIAL_ID_SPECULAR_SHINENESS as _) as *mut f32) =
                    mat.specular_shineness
            };
        }
        let textures = texture_cache.into_iter().map(|(_, tx)| tx).collect();
        (buffer, arg_encoded_length, textures)
    }
}
