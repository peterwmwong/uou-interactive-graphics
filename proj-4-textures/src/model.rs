use crate::shader_bindings::MaterialID;
use metal_app::{align_size, allocate_new_buffer_with_heap, metal::*};
use std::{
    collections::HashMap,
    hash::Hash,
    path::{Path, PathBuf},
    simd::f32x4,
};
use tobj::LoadOptions;

const DEFAULT_RESOURCE_OPTIONS: MTLResourceOptions = MTLResourceOptions::from_bits_truncate(
    MTLResourceOptions::StorageModeShared.bits()
        | MTLResourceOptions::CPUCacheModeWriteCombined.bits(),
);

trait HeapResident<T: Sized> {
    fn heap_size(&self) -> usize;
    fn allocate_and_encode(
        self,
        heap: &Heap,
        device: &Device,
        materials_arg_encoder: &ArgumentEncoder,
    ) -> (Buffer, u32, Vec<T>);
}

pub struct MaxBounds {
    pub center: f32x4,
    pub size: f32x4,
    // TODO: Calculate the xyz center of the model
}

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// Can we remove this object?
struct ModelObject {
    name: String,
    num_indices: u32,
    material_id: u32,
}

type RGB32 = [f32; 3];

#[derive(Hash, Copy, Clone, Eq, PartialEq)]
struct RGB8Unorm(u8, u8, u8);

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum MaterialSourceKey<'a> {
    PNG(&'a str),
    Color(RGB8Unorm),
}

impl<'a> MaterialSourceKey<'a> {
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

    fn size_and_padding(&self, device: &Device) -> (usize, usize) {
        let size_align = device.heap_texture_size_and_align(&self.texture_descriptor);
        let aligned_size = align_size(size_align);
        let unaligned_size = size_align.size as usize;
        debug_assert!(aligned_size >= unaligned_size);
        (unaligned_size, aligned_size - unaligned_size)
    }

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

struct Materials<'a> {
    sources: HashMap<MaterialSourceKey<'a>, MaterialSource<'a>>,
    materials: Vec<Material<'a>>,
    heap_size: usize,
}

impl<'a> Materials<'a> {
    fn new<'b>(
        device: &Device,
        material_file_dir: &'b PathBuf,
        obj_mats: &'a [tobj::Material],
    ) -> Self {
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

impl<'a> HeapResident<Texture> for Materials<'a> {
    fn heap_size(&self) -> usize {
        self.heap_size
    }

    fn allocate_and_encode(
        mut self,
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

        let mut texture_map: HashMap<MaterialSourceKey<'a>, Texture> =
            HashMap::with_capacity(self.sources.len());
        for (i, mat) in self.materials.into_iter().enumerate() {
            arg_encoder.set_argument_buffer_to_element(i as _, &buffer, 0);
            for (id, source_key) in [
                (MaterialID::ambient_texture, mat.ambient),
                (MaterialID::diffuse_texture, mat.diffuse),
                (MaterialID::specular_texture, mat.specular),
            ] {
                let texture = texture_map.entry(source_key).or_insert_with(|| {
                    self.sources
                        .get_mut(&source_key)
                        .expect("Couldn't find source key")
                        .allocate_texture(heap)
                });
                arg_encoder.set_texture(id as _, &texture);
            }
            unsafe {
                *(arg_encoder.constant_data(MaterialID::specular_shineness as _) as *mut f32) =
                    mat.specular_shineness
            };
        }
        let textures = texture_map.into_iter().map(|(_, tx)| tx).collect();
        (buffer, arg_encoded_length, textures)
    }
}

pub struct Model {
    pub max_bounds: MaxBounds,
    heap: Heap,
    objects: Vec<ModelObject>,
    geometry_arg_buffer: Buffer,
    geometry_arg_encoded_length: u32,
    // TODO: Create a type (GeometryBuffers { indices, positions, normals, tx_coords })
    #[allow(dead_code)]
    geometry_buffers: [Buffer; 4],
    materials_arg_buffer: Buffer,
    materials_arg_encoded_length: u32,
    #[allow(dead_code)]
    material_textures: Vec<Texture>,
}

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// Mimic Materials
struct GeometryResults {
    heap_size: usize,
    indices_buf_length: usize,
    positions_buf_length: usize,
    normals_buf_length: usize,
    tx_coords_buf_length: usize,
    objects: Vec<ModelObject>,
}

fn get_total_geometry_buffers_size(objs: &[tobj::Model], device: &Device) -> GeometryResults {
    let mut heap_size = 0;

    // Create a shared buffer (shared between all objects) for each ObjectGeometry member.
    // Calculate the size of each buffer...
    let mut indices_buf_length = 0;
    let mut positions_buf_length = 0;
    let mut normals_buf_length = 0;
    let mut tx_coords_buf_length = 0;
    let mut objects = Vec::<ModelObject>::with_capacity(objs.len());
    for tobj::Model { mesh, name, .. } in objs {
        assert!(
            (mesh.indices.len() % 3) == 0 &&
            (mesh.positions.len() % 3) == 0 &&
            (mesh.normals.len() % 3) == 0 &&
            (mesh.texcoords.len() % 2) == 0,
            "Unexpected number of positions, normals, or texcoords. Expected each to be triples, triples, and pairs (respectively)"
        );
        let num_positions = mesh.positions.len() / 3;
        assert!(
            (mesh.normals.len() / 3) == num_positions &&
            (mesh.texcoords.len() / 2) == num_positions,
            "Unexpected number of positions, normals, or texcoords. Expected each to be the number of indices"
        );
        indices_buf_length += byte_len(&mesh.indices);
        positions_buf_length += byte_len(&mesh.positions);
        normals_buf_length += byte_len(&mesh.normals);
        tx_coords_buf_length += byte_len(&mesh.texcoords);
        objects.push(ModelObject {
            name: name.to_owned(),
            num_indices: mesh.indices.len() as _,
            material_id: mesh.material_id.expect("No material found for object.") as _,
        });
    }
    for buf_length in [
        indices_buf_length,
        positions_buf_length,
        normals_buf_length,
        tx_coords_buf_length,
    ] {
        /*
        This may seem like a mistake to use the aligned size (size + padding) for the last buffer (No
        subsequent buffer needs padding to be aligned), but this padding actually represents the padding
        needed for the **first** buffer (right after the last texture).
        */
        heap_size += align_size(
            device.heap_buffer_size_and_align(buf_length as _, DEFAULT_RESOURCE_OPTIONS),
        );
    }
    GeometryResults {
        heap_size,
        indices_buf_length,
        positions_buf_length,
        normals_buf_length,
        tx_coords_buf_length,
        objects,
    }
}

#[inline]
fn copy_into_buffer<T: Sized>(src: &[T], dst: *mut T, offset: usize) -> usize {
    unsafe {
        let count = src.len();
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst.byte_add(offset), count);
        offset + std::mem::size_of::<T>() * count
    }
}

#[inline(always)]
const fn byte_len<T: Sized>(slice: &[T]) -> usize {
    slice.len() * std::mem::size_of::<T>()
}

// TODO: Create a return type
fn load_geometry(
    objs: &[tobj::Model],
    device: &Device,
    heap: &Heap,
    geometry_arg_encoder: &ArgumentEncoder,
    indices_buf_length: usize,
    positions_buf_length: usize,
    normals_buf_length: usize,
    tx_coords_buf_length: usize,
) -> (Buffer, u32, MaxBounds, [Buffer; 4]) {
    let arg_encoded_length = geometry_arg_encoder.encoded_length() as u32;
    let length = arg_encoded_length * objs.len() as u32;
    let arg_buffer = device.new_buffer(
        length as _,
        MTLResourceOptions::CPUCacheModeWriteCombined | MTLResourceOptions::StorageModeShared,
    );
    arg_buffer.set_label("Geometry Argument Buffer");

    // Allocate buffers...
    let mut indices_offset = 0;
    let (indices_ptr, indices_buf) =
        allocate_new_buffer_with_heap::<u32>(heap, "indices", indices_buf_length as _);
    let mut positions_offset = 0;
    let (positions_ptr, positions_buf) =
        allocate_new_buffer_with_heap::<f32>(heap, "positions", positions_buf_length as _);
    let mut normals_offset = 0;
    let (normals_ptr, normals_buf) =
        allocate_new_buffer_with_heap::<f32>(heap, "normals", normals_buf_length as _);
    let mut tx_coords_offset = 0;
    let (tx_coords_ptr, tx_coords_buf) =
        allocate_new_buffer_with_heap::<f32>(heap, "tx_coords", tx_coords_buf_length as _);

    let mut mins = f32x4::splat(f32::MAX);
    let mut maxs = f32x4::splat(f32::MIN);
    let buffers = [indices_buf, positions_buf, normals_buf, tx_coords_buf];
    let buffer_refs: [&BufferRef; 4] = [
        buffers[0].as_ref(),
        buffers[1].as_ref(),
        buffers[2].as_ref(),
        buffers[3].as_ref(),
    ];
    for (i, tobj::Model { mesh, .. }) in objs.iter().enumerate() {
        geometry_arg_encoder.set_argument_buffer_to_element(i as _, &arg_buffer, 0);
        // TODO: Figure out a way of asserting the index -> buffer mapping.
        // - Currently the Shader Binding ObjectGeometryID is completely unused/un-enforced!
        // - Maybe set_buffers() isn't worth it and we should just call set_buffer(ObjectGeometryID::_, buffer, offset)
        geometry_arg_encoder.set_buffers(
            0,
            &buffer_refs,
            &[
                indices_offset as _,
                positions_offset as _,
                normals_offset as _,
                tx_coords_offset as _,
            ],
        );

        indices_offset = copy_into_buffer(&mesh.indices, indices_ptr, indices_offset);
        normals_offset = copy_into_buffer(&mesh.normals, normals_ptr, normals_offset);
        tx_coords_offset = copy_into_buffer(&mesh.texcoords, tx_coords_ptr, tx_coords_offset);

        let positions = &mesh.positions;
        positions_offset = copy_into_buffer(&positions, positions_ptr, positions_offset);
        for &[x, y, z] in positions.as_chunks::<3>().0 {
            let input = f32x4::from_array([x, y, z, 0.0]);
            mins = mins.min(input);
            maxs = maxs.max(input);
        }
    }
    let size = maxs - mins;
    let center = mins + (size * f32x4::splat(0.5));
    (
        arg_buffer,
        arg_encoded_length,
        MaxBounds { center, size },
        buffers,
    )
}

impl Model {
    pub fn from_file<T: AsRef<Path>>(
        obj_file: T,
        device: &Device,
        geometry_arg_encoder: &ArgumentEncoder,
        materials_arg_encoder: &ArgumentEncoder,
    ) -> Self {
        let obj_file_ref = obj_file.as_ref();
        let (models, materials) = tobj::load_obj(
            obj_file_ref,
            &LoadOptions {
                single_index: true,
                triangulate: true,
                ignore_points: true,
                ignore_lines: true,
            },
        )
        .expect("Failed to load OBJ file");

        let materials = materials.expect("Failed to load materials data");
        let material_file_dir = PathBuf::from(
            obj_file_ref
                .parent()
                .expect("Failed to get obj file's parent directory"),
        );
        let all_materials = Materials::new(device, &material_file_dir, &materials);

        // Size Heap for Geometry and Materials
        let geometry = get_total_geometry_buffers_size(&models, device);

        // Allocate Heap for Geometry and Materials
        let desc = HeapDescriptor::new();
        desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
        desc.set_storage_mode(MTLStorageMode::Shared);
        desc.set_size((all_materials.heap_size() + geometry.heap_size) as _);
        let heap = device.new_heap(&desc);
        heap.set_label("Geometry and Materials Heap");

        // IMPORTANT: Load material textures *BEFORE* geometry. Heap size calculations
        // (specifically alignment padding) assume this.
        let (materials_arg_buffer, materials_arg_encoded_length, material_textures) =
            all_materials.allocate_and_encode(&heap, device, materials_arg_encoder);

        let (geometry_arg_buffer, geometry_arg_encoded_length, max_bounds, geometry_buffers) =
            load_geometry(
                &models,
                device,
                &heap,
                &geometry_arg_encoder,
                geometry.indices_buf_length,
                geometry.positions_buf_length,
                geometry.normals_buf_length,
                geometry.tx_coords_buf_length,
            );

        Self {
            heap,
            objects: geometry.objects,
            geometry_arg_buffer,
            geometry_arg_encoded_length,
            geometry_buffers,
            max_bounds,
            materials_arg_buffer,
            materials_arg_encoded_length,
            material_textures,
        }
    }

    #[inline]
    pub fn encode_use_resources(&self, encoder: &RenderCommandEncoderRef) {
        encoder.use_heap_at(
            &self.heap,
            MTLRenderStages::Vertex | MTLRenderStages::Fragment,
        )
    }

    #[inline]
    pub fn encode_draws(
        &self,
        encoder: &RenderCommandEncoderRef,
        vertex_geometry_arg_buffer_id: u8,
        fragment_material_arg_buffer_id: u8,
    ) {
        let mut geometry_arg_buffer_offset = 0;
        for o in &self.objects {
            encoder.push_debug_group(&o.name);

            let material_arg_buffer_offset = o.material_id * self.materials_arg_encoded_length;

            // For the first object, encode the vertex/fragment buffer.
            if geometry_arg_buffer_offset == 0 {
                encoder.set_vertex_buffer(
                    vertex_geometry_arg_buffer_id as _,
                    Some(self.geometry_arg_buffer.as_ref()),
                    0,
                );
                encoder.set_fragment_buffer(
                    fragment_material_arg_buffer_id as _,
                    Some(self.materials_arg_buffer.as_ref()),
                    material_arg_buffer_offset as _,
                );
            }
            // Subsequent objects, just move the vertex/fragment buffer offsets
            else {
                encoder.set_vertex_buffer_offset(
                    vertex_geometry_arg_buffer_id as _,
                    geometry_arg_buffer_offset as _,
                );

                encoder.set_fragment_buffer_offset(
                    fragment_material_arg_buffer_id as _,
                    material_arg_buffer_offset as _,
                );
            }
            geometry_arg_buffer_offset += self.geometry_arg_encoded_length;

            encoder.draw_primitives(MTLPrimitiveType::Triangle, 0, o.num_indices as _);

            encoder.pop_debug_group();
        }
    }
}
