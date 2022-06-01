use crate::shader_bindings::{float4, MaterialID};
use metal_app::{allocate_new_buffer, metal::*};
use std::{
    path::{Path, PathBuf},
    simd::f32x4,
};
use tobj::LoadOptions;

pub struct MaxBounds {
    pub width: f32,
    pub height: f32,
    // TODO: Calculate the xyz center of the model
}

struct ModelObject {
    name: String,
    num_triangles: u32,
}

// TODO: START HERE 2
// TODO: START HERE 2
// TODO: START HERE 2
// Consider using a Metal Heap
// - Should reduce encode_use_resources() to a single call for the heap
// - See https://developer.apple.com/documentation/metal/buffers/using_argument_buffers_with_resource_heaps
pub struct Model {
    pub max_bounds: MaxBounds,
    objects: Vec<ModelObject>,
    object_geometries_arg_buffer: Buffer,
    object_geometries_arg_encoded_length: u32,
    // TODO: Create a type (ObjectGeometryBuffers { indices, positions, normals, tx_coords })
    object_geometry_buffers: [Buffer; 4],
    materials_arg_buffer: Buffer,
    materials_arg_encoded_length: u32,
    material_textures: Vec<Texture>,
}

fn load_texture_from_png<T: AsRef<Path> + std::fmt::Debug>(
    label: &str,
    path_to_png: T,
    device: &Device,
) -> Texture {
    use png::ColorType::*;
    use std::fs::File;
    let mut decoder = png::Decoder::new(File::open(path_to_png).unwrap());
    decoder.set_transformations(png::Transformations::normalize_to_color8());

    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let width = info.width as _;
    let height = info.height as _;
    assert_eq!(
        info.color_type, Rgba,
        "Unexpected PNG format, expected RGBA"
    );

    let desc = TextureDescriptor::new();

    desc.set_width(width);
    desc.set_height(height);
    desc.set_pixel_format(MTLPixelFormat::RGBA8Unorm);
    desc.set_storage_mode(MTLStorageMode::Shared);
    desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
    desc.set_usage(MTLTextureUsage::ShaderRead);

    let texture = device.new_texture(&desc);
    texture.set_label(label);
    texture.replace_region(
        MTLRegion {
            origin: MTLOrigin { x: 0, y: 0, z: 0 },
            size: MTLSize {
                width,
                height,
                depth: 1,
            },
        },
        0,
        buf.as_ptr() as _,
        width * 4,
    );
    texture
}

#[inline]
fn copy_into_buffer<T: Sized>(src: &[T], dst: *mut T) -> (isize, *mut T) {
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
        let new_contents = dst.add(src.len());
        let offset = new_contents.offset_from(dst);
        (offset, new_contents)
    }
}

#[inline(always)]
const fn byte_len<T: Sized>(slice: &[T]) -> usize {
    slice.len() * std::mem::size_of::<T>()
}

// TODO: Create a return type
fn load_object_geometries(
    objs: &[tobj::Model],
    device: &Device,
    arg_encoder: &ArgumentEncoder,
) -> (Vec<ModelObject>, Buffer, u32, MaxBounds, [Buffer; 4]) {
    let arg_encoded_length = arg_encoder.encoded_length() as u32;
    let length = arg_encoded_length * objs.len() as u32;
    let buf = device.new_buffer(
        length as _,
        MTLResourceOptions::CPUCacheModeWriteCombined | MTLResourceOptions::StorageModeShared,
    );

    // Create a shared buffer (shared between all objects) for each ObjectGeometry member.
    // Calculate the size of each buffer...
    let mut indices_buf_length = 0;
    let mut positions_buf_length = 0;
    let mut normals_buf_length = 0;
    let mut texcoords_buf_length = 0;
    let mut objects: Vec<ModelObject> = vec![];
    let mut mins = f32x4::splat(f32::MAX);
    let mut maxs = f32x4::splat(f32::MIN);
    for tobj::Model {
        mesh:
            tobj::Mesh {
                indices,
                positions,
                normals,
                texcoords,
                ..
            },
        name,
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
        for &[x, y, z] in positions.as_chunks::<3>().0 {
            let input = f32x4::from_array([x, y, z, 0.0]);
            mins = mins.min(input);
            maxs = maxs.max(input);
        }

        indices_buf_length += byte_len(&indices);
        positions_buf_length += byte_len(&positions);
        normals_buf_length += byte_len(&normals);
        texcoords_buf_length += byte_len(&texcoords);
        objects.push(ModelObject {
            name: name.to_owned(),
            num_triangles: (indices.len() / 3) as _,
        });
    }
    let max_bounds = mins.abs().max(maxs);
    // TODO: Find the actual center, width, and height of the model.
    // - Currently ASSUMES a symmetrical model or, at the very least, a model where (0,0,0) is the
    // center.
    let width = max_bounds[0] * 2.;
    let height = max_bounds[1] * 2.;

    // Allocate buffers...
    let mut indices_offset = 0;
    let (mut indices_contents, indices_buf) =
        allocate_new_buffer::<u32>(device, "indices", indices_buf_length as _);
    let mut positions_offset = 0;
    let (mut positions_contents, positions_buf) =
        allocate_new_buffer::<f32>(device, "positions", positions_buf_length as _);
    let mut normals_offset = 0;
    let (mut normals_contents, normals_buf) =
        allocate_new_buffer::<f32>(device, "normals", normals_buf_length as _);
    let mut texcoords_offset = 0;
    let (mut texcoords_contents, texcoords_buf) =
        allocate_new_buffer::<f32>(device, "texcoords", texcoords_buf_length as _);

    let buffers = [indices_buf, positions_buf, normals_buf, texcoords_buf];
    let buf_refs: [&BufferRef; 4] = [
        buffers[0].as_ref(),
        buffers[1].as_ref(),
        buffers[2].as_ref(),
        buffers[3].as_ref(),
    ];
    for (i, tobj::Model { mesh, .. }) in objs.iter().enumerate() {
        arg_encoder.set_argument_buffer_to_element(i as _, &buf, 0);
        // TODO: Figure out a way of asserting the index -> buffer mapping.
        // - Currently the Shader Binding ObjectGeometryID is completely unused/un-enforced!
        // - Maybe set_buffers() isn't worth it and we should just call set_buffer(ObjectGeometryID::_, buffer, offset)
        arg_encoder.set_buffers(
            0,
            &buf_refs,
            &[
                indices_offset as _,
                positions_offset as _,
                normals_offset as _,
                texcoords_offset as _,
            ],
        );
        (indices_offset, indices_contents) = copy_into_buffer(&mesh.indices, indices_contents);
        (positions_offset, positions_contents) =
            copy_into_buffer(&mesh.positions, positions_contents);
        (normals_offset, normals_contents) = copy_into_buffer(&mesh.normals, normals_contents);
        (texcoords_offset, texcoords_contents) =
            copy_into_buffer(&mesh.texcoords, texcoords_contents);
    }

    (
        objects,
        buf,
        arg_encoded_length,
        MaxBounds { width, height },
        buffers,
    )
}

// TODO: Create a return type
fn load_materials(
    mats: &[tobj::Material],
    material_file_dir: PathBuf,
    device: &Device,
    mat_encoder: &ArgumentEncoder,
) -> (Buffer, Vec<Texture>, u32) {
    let mat_arg_encoded_length = mat_encoder.encoded_length() as u32;
    let length = (mat_arg_encoded_length as u64) * mats.len() as u64;
    let buf = device.new_buffer(
        length,
        MTLResourceOptions::CPUCacheModeWriteCombined | MTLResourceOptions::StorageModeShared,
    );

    // Assume all materials have a diffuse and specular texture
    let mut all_textures = Vec::with_capacity(mats.len() * 2);
    for (i, mat) in mats.iter().enumerate() {
        mat_encoder.set_argument_buffer_to_element(i as _, &buf, 0);
        unsafe {
            *(mat_encoder.constant_data(MaterialID::diffuse_color as _) as *mut float4) =
                float4::new(0., 0., 0., 0.)
        };
        unsafe {
            *(mat_encoder.constant_data(MaterialID::specular_color as _) as *mut float4) =
                float4::new(0., 0., 0., 0.)
        };
        assert_eq!(
            (MaterialID::diffuse_texture as u32) + 1,
            MaterialID::specular_texture as u32,
            "Following call to set_textures() expects IDs diffuse_texture + 1 == specular_texture"
        );
        for (id, texture_file) in [
            (MaterialID::diffuse_texture, &mat.diffuse_texture),
            (MaterialID::specular_texture, &mat.specular_texture),
        ] {
            if !texture_file.is_empty() {
                let tx = load_texture_from_png(
                    &texture_file,
                    &material_file_dir.join(texture_file),
                    device,
                );
                mat_encoder.set_texture(id as _, &tx);
                all_textures.push(tx);
            }
        }
        unsafe {
            *(mat_encoder.constant_data(MaterialID::specular_shineness as _) as *mut f32) =
                mat.shininess
        };
    }

    (buf, all_textures, mat_arg_encoded_length)
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
            Some(self.model.object_geometries_arg_buffer.as_ref()),
            (self.i as u64) * (self.model.object_geometries_arg_encoded_length as u64),
        );
    }

    #[inline(always)]
    pub fn encode_fragment_buffer_for_material_argument_buffer(
        &self,
        encoder: &RenderCommandEncoderRef,
        buffer_index: u64,
    ) {
        encoder.set_fragment_buffer(
            buffer_index,
            Some(self.model.materials_arg_buffer.as_ref()),
            (self.i as u64) * (self.model.materials_arg_encoded_length as u64),
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
        object_geometry_arg_encoder: &ArgumentEncoder,
        material_arg_encoder: &ArgumentEncoder,
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

        let (
            objects,
            object_geometries_buffer,
            object_geometry_arg_encoded_length,
            max_bounds,
            object_geometry_buffers,
        ) = load_object_geometries(&models, device, &object_geometry_arg_encoder);

        let materials = materials.expect("Failed to load materials data");
        let (materials_arg_buffer, material_textures, materials_arg_encoded_length) =
            load_materials(
                &materials,
                PathBuf::from(
                    obj_file_ref
                        .parent()
                        .expect("Failed to get obj file's parent directory"),
                ),
                device,
                &material_arg_encoder,
            );

        Self {
            objects,
            object_geometries_arg_buffer: object_geometries_buffer,
            object_geometries_arg_encoded_length: object_geometry_arg_encoded_length,
            object_geometry_buffers,
            max_bounds,
            materials_arg_buffer,
            materials_arg_encoded_length,
            material_textures,
        }
    }

    pub fn encode_use_resources(&self, encoder: &RenderCommandEncoderRef) {
        let bufs = &self.object_geometry_buffers;
        encoder.use_resources(
            &[&bufs[0], &bufs[1], &bufs[2], &bufs[3]],
            MTLResourceUsage::Read,
            MTLRenderStages::Vertex,
        );
        for texture in &self.material_textures {
            encoder.use_resource_at(texture, MTLResourceUsage::Sample, MTLRenderStages::Fragment);
        }
    }

    pub fn object_iter(&self) -> impl Iterator<Item = ModelObjectIterResult> {
        ModelObjectIter {
            model: self,
            iter: self.objects.iter().enumerate(),
        }
    }
}

#[cfg(test)]
mod tests {

    use std::ptr::null;

    const fn yolo() {
        #[repr(C)]
        struct Yolo {
            p1: [u8; 4],
            p2: u8,
        }
        let a: *const Yolo = null();
        unsafe {
            null::<Yolo>().offset_from(a);
        };
        let b = unsafe {
            #[allow(deref_nullptr)]
            (&(*(null::<Yolo>())).p1[1] as *const u8).offset_from(null())
        };
    }

    #[test]
    fn test() {}
}
