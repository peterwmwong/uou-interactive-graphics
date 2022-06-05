use crate::shader_bindings::{float4, MaterialID};
use metal_app::{align_size, allocate_new_buffer_with_heap, metal::*};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    simd::f32x4,
};
use tobj::LoadOptions;

pub struct MaxBounds {
    pub center: f32x4,
    pub size: f32x4,
    // TODO: Calculate the xyz center of the model
}

struct ModelObject {
    name: String,
    num_indices: u32,
    material_id: u32,
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

fn get_png_reader<T: AsRef<Path> + std::fmt::Debug>(
    path_to_png: T,
) -> (png::Reader<std::fs::File>, TextureDescriptor) {
    use std::fs::File;
    let decoder = png::Decoder::new(File::open(path_to_png).unwrap());

    let reader = decoder.read_info().unwrap();
    let desc = {
        let desc = TextureDescriptor::new();
        let &png::Info {
            color_type,
            width,
            height,
            ..
        } = reader.info();
        assert_eq!(
            color_type,
            png::ColorType::Rgba,
            "Unexpected PNG color format, expected RGBA"
        );
        desc.set_width(width as _);
        desc.set_height(height as _);
        desc.set_pixel_format(MTLPixelFormat::RGBA8Unorm);
        desc.set_storage_mode(MTLStorageMode::Shared);
        desc.set_texture_type(MTLTextureType::D2);
        desc.set_usage(MTLTextureUsage::ShaderRead);
        desc.set_resource_options(
            MTLResourceOptions::StorageModeShared | MTLResourceOptions::CPUCacheModeWriteCombined,
        );
        desc
    };
    (reader, desc)
}

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
    for tobj::Model {
        mesh:
            tobj::Mesh {
                indices,
                positions,
                normals,
                texcoords,
                material_id,
                ..
            },
        name,
        ..
    } in objs
    {
        assert!(
            (indices.len() % 3) == 0 &&
            (positions.len() % 3) == 0 &&
            (normals.len() % 3) == 0 &&
            (texcoords.len() % 2) == 0,
            "Unexpected number of positions, normals, or texcoords. Expected each to be triples, triples, and pairs (respectively)"
        );
        let num_positions = positions.len() / 3;
        assert!(
            (normals.len() / 3) == num_positions &&
            (texcoords.len() / 2) == num_positions,
            "Unexpected number of positions, normals, or texcoords. Expected each to be the number of indices"
        );

        indices_buf_length += byte_len(&indices);
        positions_buf_length += byte_len(&positions);
        normals_buf_length += byte_len(&normals);
        tx_coords_buf_length += byte_len(&texcoords);
        objects.push(ModelObject {
            name: name.to_owned(),
            num_indices: indices.len() as _,
            material_id: material_id.expect("No material found for object.") as _,
        });
    }

    heap_size += align_size(device.heap_buffer_size_and_align(
        indices_buf_length as _,
        MTLResourceOptions::StorageModeShared | MTLResourceOptions::CPUCacheModeWriteCombined,
    ));
    heap_size += align_size(device.heap_buffer_size_and_align(
        positions_buf_length as _,
        MTLResourceOptions::StorageModeShared | MTLResourceOptions::CPUCacheModeWriteCombined,
    ));
    heap_size += align_size(device.heap_buffer_size_and_align(
        positions_buf_length as _,
        MTLResourceOptions::StorageModeShared | MTLResourceOptions::CPUCacheModeWriteCombined,
    ));
    heap_size += align_size(device.heap_buffer_size_and_align(
        normals_buf_length as _,
        MTLResourceOptions::StorageModeShared | MTLResourceOptions::CPUCacheModeWriteCombined,
    ));

    /*
    This may seem like a mistake to use the aligned size (size + padding) for the last buffer (No
    subsequent buffer needs padding to be aligned), but this padding actually represents the padding
    needed for the **first** buffer (right after the last texture).
    */
    heap_size += align_size(device.heap_buffer_size_and_align(
        tx_coords_buf_length as _,
        MTLResourceOptions::StorageModeShared | MTLResourceOptions::CPUCacheModeWriteCombined,
    ));

    GeometryResults {
        heap_size,
        indices_buf_length,
        positions_buf_length,
        normals_buf_length,
        tx_coords_buf_length,
        objects,
    }
}

fn get_total_material_texture_size_and_readers<'a, 'b, 'c>(
    materials: &'a [tobj::Material],
    material_file_dir: &'b PathBuf,
    device: &'c Device,
) -> (
    usize,
    HashMap<&'a str, (png::Reader<std::fs::File>, TextureDescriptor)>,
) {
    let mut texture_to_reader_map =
        HashMap::<&str, (png::Reader<std::fs::File>, TextureDescriptor)>::with_capacity(
            materials.len(),
        );
    let mut size = 0;

    // Add padding between textures to make sure every texture is properly aligned according to
    // `Device#heap_texture_size_and_align()`. See https://developer.apple.com/documentation/metal/mtldevice/1649927-heaptexturesizeandalignwithdescr?language=objc.
    // IMPORTANT: Assumes the first texture (allocated from the heap) is aligned and does not need
    // any padding in the beginning to be aligned.
    let mut last_alignment_padding = 0;
    for mat in materials {
        for texture_file in [&mat.diffuse_texture, &mat.specular_texture] {
            if !texture_file.is_empty() {
                texture_to_reader_map
                    .entry(texture_file)
                    .or_insert_with(|| {
                        let (reader, desc) = get_png_reader(&material_file_dir.join(texture_file));
                        let size_align = device.heap_texture_size_and_align(&desc);

                        // Add alignment-padding to make sure texture is properly aligned.
                        size += last_alignment_padding + (size_align.size as usize);

                        debug_assert!(align_size(size_align) >= size_align.size as _);
                        last_alignment_padding =
                            align_size(size_align) - (size_align.size as usize);

                        (reader, desc)
                    });
            }
        }
    }
    (size, texture_to_reader_map)
}

// TODO: Move into metal-app
fn load_texture_from_png(
    label: &str,
    reader: &mut png::Reader<std::fs::File>,
    desc: &TextureDescriptor,
    heap: &Heap,
) -> Texture {
    // TODO: Allocate this buf once
    // - Get the maximum output_buffer_size() of all the textures
    let buf_size = reader.output_buffer_size();
    let mut buf = Vec::with_capacity(buf_size);
    unsafe { buf.set_len(buf_size) };
    reader.next_frame(&mut buf).expect("Failed to load texture");

    let texture = heap
        .new_texture(&desc)
        .expect(&format!("Failed to allocate texture for {label}"));
    texture.set_label(label);
    const BYTES_PER_RGBA_PIXEL: NSUInteger = 4;
    texture.replace_region(
        MTLRegion {
            origin: MTLOrigin { x: 0, y: 0, z: 0 },
            size: MTLSize {
                width: desc.width(),
                height: desc.height(),
                depth: 1,
            },
        },
        0,
        buf.as_ptr() as _,
        desc.width() * BYTES_PER_RGBA_PIXEL,
    );
    texture
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

// TODO: Create a return type
fn load_materials(
    mats: &[tobj::Material],
    mut texture_to_reader_map: HashMap<&str, (png::Reader<std::fs::File>, TextureDescriptor)>,
    device: &Device,
    heap: &Heap,
    materials_arg_encoder: &ArgumentEncoder,
) -> (Buffer, Vec<Texture>, u32) {
    let arg_encoded_length = materials_arg_encoder.encoded_length() as u32;
    let length = (arg_encoded_length as u64) * mats.len() as u64;
    let arg_buffer = device.new_buffer(
        length,
        MTLResourceOptions::CPUCacheModeWriteCombined | MTLResourceOptions::StorageModeShared,
    );
    arg_buffer.set_label("Materials Argument Buffer");

    // Assume all materials have 2 textures: diffuse and specular
    let mut texture_map: HashMap<&str, Texture> = HashMap::with_capacity(mats.len() * 2);
    for (i, mat) in mats.iter().enumerate() {
        materials_arg_encoder.set_argument_buffer_to_element(i as _, &arg_buffer, 0);
        // TODO: Actually load diffuse and specular color
        unsafe {
            *(materials_arg_encoder.constant_data(MaterialID::diffuse_color as _) as *mut float4) =
                float4::new(0., 0., 0., 0.)
        };
        unsafe {
            *(materials_arg_encoder.constant_data(MaterialID::specular_color as _) as *mut float4) =
                float4::new(0., 0., 0., 0.)
        };
        for (id, texture_file) in [
            (MaterialID::diffuse_texture, &mat.diffuse_texture),
            (MaterialID::specular_texture, &mat.specular_texture),
        ] {
            if !texture_file.is_empty() {
                let tx = texture_map.entry(texture_file).or_insert_with(|| {
                    let (reader, desc) = texture_to_reader_map
                        .get_mut(texture_file as &str)
                        .expect(
                            "Failed to get reader and texture descriptor (see get_total_material_texture_size)"
                        );
                    load_texture_from_png(&texture_file, reader, desc, heap)
                });
                materials_arg_encoder.set_texture(id as _, &tx);
            }
        }
        unsafe {
            *(materials_arg_encoder.constant_data(MaterialID::specular_shineness as _)
                as *mut f32) = mat.shininess
        };
    }
    let textures = texture_map.into_values().collect();

    (arg_buffer, textures, arg_encoded_length)
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

        // Size Heap for Geometry and Materials
        let (texture_heap_size, texture_to_reader_map) =
            get_total_material_texture_size_and_readers(&materials, &material_file_dir, device);
        let geometry = get_total_geometry_buffers_size(&models, device);

        // Allocate Heap for Geometry and Materials
        let desc = HeapDescriptor::new();
        desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
        desc.set_storage_mode(MTLStorageMode::Shared);
        desc.set_size((texture_heap_size + geometry.heap_size) as _);
        let heap = device.new_heap(&desc);
        heap.set_label("Geometry and Materials Heap");

        // IMPORTANT: Load material textures *BEFORE* geometry. Heap size calculations
        // (specifically alignment padding) assume this.
        let (materials_arg_buffer, material_textures, materials_arg_encoded_length) =
            load_materials(
                &materials,
                texture_to_reader_map,
                device,
                &heap,
                &materials_arg_encoder,
            );

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
