use crate::shader_bindings::{float4, MaterialID};
use metal_app::{align_size, allocate_new_buffer_with_heap, metal::*};
use std::{
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
    num_triangles: u32,
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
        let info = reader.info();
        assert_eq!(
            info.color_type,
            png::ColorType::Rgba,
            "Unexpected PNG color format, expected RGBA"
        );
        desc.set_width(info.width as _);
        desc.set_height(info.height as _);
        desc.set_pixel_format(MTLPixelFormat::RGBA8Unorm);
        desc.set_storage_mode(MTLStorageMode::Shared);
        desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
        desc.set_usage(MTLTextureUsage::ShaderRead);
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
    let mut objects: Vec<ModelObject> = vec![];
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
            num_triangles: (indices.len() / 3) as _,
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
        normals_buf_length as _,
        MTLResourceOptions::StorageModeShared | MTLResourceOptions::CPUCacheModeWriteCombined,
    ));
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

fn get_total_material_texture_size(
    materials: &[tobj::Material],
    material_file_dir: &PathBuf,
    device: &Device,
) -> usize {
    let mut size = 0;
    for mat in materials {
        for texture_file in [&mat.diffuse_texture, &mat.specular_texture] {
            if !texture_file.is_empty() {
                // TODO: Try to reuse the reader later to load the texture, instead of calling get_png_reader again in load_texture_from_png.
                let (_, desc) = get_png_reader(&material_file_dir.join(texture_file));
                let size_align = device.heap_texture_size_and_align(&desc);
                let texture_size = align_size(size_align);
                size += texture_size;
            }
        }
    }
    size
}

// TODO: Move into metal-app
fn load_texture_from_png(
    label: &str,
    (mut reader, desc): (png::Reader<std::fs::File>, TextureDescriptor),
    heap: &Heap,
) -> Texture {
    // TODO: Allocate this buf once
    // - Get the maximum output_buffer_size() of all the textures
    let mut buf = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut buf).expect("Failed to load texture");

    let texture = heap
        .new_texture(&desc)
        .expect(&format!("Failed to allocate texture for {label}"));
    texture.set_label(label);
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
        // TODO: What is 4? Constantize and give it a name.
        desc.width() * 4,
    );
    texture
}

#[inline]
fn copy_into_buffer<T: Sized>(src: &[T], dst: *mut T, old_offset: isize) -> (isize, *mut T) {
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
        let new_contents = dst.add(src.len());
        let offset = new_contents.byte_offset_from(dst);
        (old_offset + offset, new_contents)
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
    let (mut indices_contents, indices_buf) =
        allocate_new_buffer_with_heap::<u32>(heap, "indices", indices_buf_length as _);
    let mut positions_offset = 0;
    let (mut positions_contents, positions_buf) =
        allocate_new_buffer_with_heap::<f32>(heap, "positions", positions_buf_length as _);
    let mut normals_offset = 0;
    let (mut normals_contents, normals_buf) =
        allocate_new_buffer_with_heap::<f32>(heap, "normals", normals_buf_length as _);
    let mut tx_coords_offset = 0;
    let (mut tx_coords_contents, tx_coords_buf) =
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

        // TODO: START HERE
        // TODO: START HERE
        // TODO: START HERE
        // Make copy_into_buffer just return offset and remove the need to update dst pointer
        // - copy_into_buffer will just do dst.byte_add(indices_offset)
        //      - IMPORTANT: byte_add()
        //      - IMPORTANT: byte_add()
        //      - IMPORTANT: byte_add()
        (indices_offset, indices_contents) =
            copy_into_buffer(&mesh.indices, indices_contents, indices_offset);
        (normals_offset, normals_contents) =
            copy_into_buffer(&mesh.normals, normals_contents, normals_offset);
        (tx_coords_offset, tx_coords_contents) =
            copy_into_buffer(&mesh.texcoords, tx_coords_contents, tx_coords_offset);

        let positions = &mesh.positions;
        (positions_offset, positions_contents) =
            copy_into_buffer(&positions, positions_contents, positions_offset);
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
    material_file_dir: PathBuf,
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

    // Assume all materials have a diffuse and specular texture
    let mut textures = Vec::with_capacity(mats.len() * 2);
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
        // TODO: START HERE
        // TODO: START HERE
        // TODO: START HERE
        // Handle multiple materials using the same texture image.
        // - The Yoda model reuses the body texture for Body and Body_1 materials
        for (id, texture_file) in [
            (MaterialID::diffuse_texture, &mat.diffuse_texture),
            (MaterialID::specular_texture, &mat.specular_texture),
        ] {
            if !texture_file.is_empty() {
                let png_reader_and_desc = get_png_reader(&material_file_dir.join(texture_file));
                let tx = load_texture_from_png(&texture_file, png_reader_and_desc, heap);
                materials_arg_encoder.set_texture(id as _, &tx);
                textures.push(tx);
            }
        }
        unsafe {
            *(materials_arg_encoder.constant_data(MaterialID::specular_shineness as _)
                as *mut f32) = mat.shininess
        };
    }

    (arg_buffer, textures, arg_encoded_length)
}

pub struct ModelObjectIterResult<'a> {
    i: usize,
    model: &'a Model,
    object: &'a ModelObject,
}

impl<'a> ModelObjectIterResult<'a> {
    #[inline(always)]
    pub fn num_triangles(&self) -> u32 {
        self.object.num_triangles
    }

    #[inline(always)]
    pub fn name(&self) -> &str {
        &self.object.name
    }

    #[inline(always)]
    pub fn encode_vertex_buffer_for_geometry_argument_buffer(
        &self,
        encoder: &RenderCommandEncoderRef,
        buffer_index: u64,
    ) {
        encoder.set_vertex_buffer(
            buffer_index,
            Some(self.model.geometry_arg_buffer.as_ref()),
            (self.i as u64) * (self.model.geometry_arg_encoded_length as u64),
        );
    }

    #[inline(always)]
    pub fn encode_fragment_buffer_for_material_argument_buffer(
        &self,
        encoder: &RenderCommandEncoderRef,
        buffer_index: u64,
    ) {
        let i = self.object.material_id as u64;
        encoder.set_fragment_buffer(
            buffer_index,
            Some(self.model.materials_arg_buffer.as_ref()),
            (i as u64) * (self.model.materials_arg_encoded_length as u64),
        );
    }
}

struct ModelObjectIter<'a, T: Iterator<Item = (usize, &'a ModelObject)>> {
    iter: T,
    model: &'a Model,
}

impl<'a, T: Iterator<Item = (usize, &'a ModelObject)>> Iterator for ModelObjectIter<'a, T> {
    type Item = ModelObjectIterResult<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(i, object)| ModelObjectIterResult {
            i,
            model: &self.model,
            object,
        })
    }
}
impl<'a, T: Iterator<Item = (usize, &'a ModelObject)>> ExactSizeIterator
    for ModelObjectIter<'a, T>
{
    #[inline(always)]
    fn len(&self) -> usize {
        self.model.objects.len()
    }
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
        let mut heap_size = 0;
        let geometry = get_total_geometry_buffers_size(&models, device);
        heap_size += geometry.heap_size;
        // TODO: Figure out why this is "* 2"!
        heap_size += get_total_material_texture_size(&materials, &material_file_dir, device) * 2;

        // Allocate Heap for Geometry and Materials
        let desc = HeapDescriptor::new();
        desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
        desc.set_storage_mode(MTLStorageMode::Shared);
        desc.set_size(heap_size as _);
        let heap = device.new_heap(&desc);
        heap.set_label("Geometry and Materials Heap");

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

        let (materials_arg_buffer, material_textures, materials_arg_encoded_length) =
            load_materials(
                &materials,
                material_file_dir,
                device,
                &heap,
                &materials_arg_encoder,
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

    pub fn encode_use_resources(&self, encoder: &RenderCommandEncoderRef) {
        encoder.use_heap_at(
            &self.heap,
            MTLRenderStages::Vertex | MTLRenderStages::Fragment,
        )
    }

    pub fn object_iter(&self) -> impl Iterator<Item = ModelObjectIterResult> {
        ModelObjectIter {
            model: self,
            iter: self.objects.iter().enumerate(),
        }
    }
}
