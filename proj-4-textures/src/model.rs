use crate::shader_bindings::{float4, MaterialID};
use metal_app::{allocate_new_buffer, metal::*};
use std::path::{Path, PathBuf};
use tobj::LoadOptions;

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// - Rethink how this will actually be used and name/structure accordingly
// - In the renderer, really we need this for 2 things
//      1. Setup Model-to-World Matrix, translate the center of the model to (0,0,0)
//      2. Setup the Orthographic projection matrix, width of the near field
pub struct MaxBounds {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct ModelObject {
    pub name: String,
    pub num_triangles: usize,
}

pub struct Model {
    pub max_bounds: MaxBounds,
    pub objects: Vec<ModelObject>,
    pub object_geometries_buffer: Buffer,
    pub object_geometry_arg_encoded_length: u64,
    pub materials_buffer: Buffer,
    pub material_arg_encoded_length: u64,
}

fn load_texture_from_png<T: AsRef<Path>>(label: &str, path_to_png: T, device: &Device) -> Texture {
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

fn copy_into_buffer<T: Sized>(src: &[T], dst: *mut T) -> (isize, *mut T) {
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
        let new_contents = dst.add(src.len());
        let offset = new_contents.offset_from(dst);
        (offset, new_contents)
    }
}

const fn byte_len<T: Sized>(slice: &[T]) -> usize {
    slice.len() * std::mem::size_of::<T>()
}

fn load_object_geometries(
    objs: &[tobj::Model],
    device: &Device,
    obj_geo_encoder: &ArgumentEncoder,
) -> (Vec<ModelObject>, Buffer, u64, MaxBounds) {
    let obj_geo_arg_encoded_length = obj_geo_encoder.encoded_length();
    let length = obj_geo_arg_encoded_length * objs.len() as u64;
    let buf = device.new_buffer(
        length,
        MTLResourceOptions::CPUCacheModeWriteCombined | MTLResourceOptions::StorageModeShared,
    );

    // Create a shared buffer (shared between all objects) for each ObjectGeometry member.
    // Calculate the size of each buffer...
    let mut indices_buf_length = 0;
    let mut positions_buf_length = 0;
    let mut normals_buf_length = 0;
    let mut texcoords_buf_length = 0;
    let mut objects: Vec<ModelObject> = vec![];
    for tobj::Model { mesh, name } in objs {
        assert!(
            (mesh.positions.len() % 3) == 0 &&
            (mesh.normals.len() % 3) == 0 &&
            (mesh.texcoords.len() % 2) == 0,
            "Unexpected number of positions, normals, or texcoords. Expected each to be triples, triples, and pairs (respectively)"
        );
        let num_triangles = mesh.positions.len() / 3;
        assert!(
            num_triangles == mesh.indices.len() &&
            (mesh.normals.len() / 3) == mesh.indices.len() &&
            (mesh.texcoords.len() / 2) == mesh.indices.len(),
            "Unexpected number of positions, normals, or texcoords. Expected each to be the number of indices"
        );
        indices_buf_length += byte_len(&mesh.indices);
        positions_buf_length += byte_len(&mesh.positions);
        normals_buf_length += byte_len(&mesh.normals);
        texcoords_buf_length += byte_len(&mesh.texcoords);
        objects.push(ModelObject {
            name: name.to_owned(),
            num_triangles,
        });
    }

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

    let buffers: [&BufferRef; 4] = [&indices_buf, &positions_buf, &normals_buf, &texcoords_buf];
    for (i, tobj::Model { mesh, .. }) in objs.iter().enumerate() {
        obj_geo_encoder.set_argument_buffer_to_element(i as _, &buf, 0);
        // TODO: Figure out a way of asserting the index -> buffer mapping.
        obj_geo_encoder.set_buffers(
            0,
            &buffers,
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
        obj_geo_arg_encoded_length,
        MaxBounds {
            x: todo!(),
            y: todo!(),
            z: todo!(),
        },
    )
}

fn load_materials(
    mats: &[tobj::Material],
    material_file_dir: PathBuf,
    device: &Device,
    mat_encoder: &ArgumentEncoder,
) -> Buffer {
    let mat_arg_encoded_length = mat_encoder.encoded_length();
    let length = mat_arg_encoded_length * mats.len() as u64;
    let buf = device.new_buffer(
        length,
        MTLResourceOptions::CPUCacheModeWriteCombined | MTLResourceOptions::StorageModeShared,
    );

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
        let set_texture = |id: MaterialID, texture_file: &str| {
            if !texture_file.is_empty() {
                let tx = load_texture_from_png(
                    &texture_file,
                    &material_file_dir.join(texture_file),
                    device,
                );
                mat_encoder.set_texture(id as _, &tx);
            }
        };
        set_texture(MaterialID::diffuse_texture, &mat.diffuse_texture);
        set_texture(MaterialID::specular_texture, &mat.specular_texture);
        unsafe {
            *(mat_encoder.constant_data(MaterialID::specular_shineness as _) as *mut f32) =
                mat.shininess
        };
    }

    buf
}

impl Model {
    pub fn from_file<T: AsRef<Path>>(
        obj_file: T,
        device: &Device,
        object_geometry_arg_encoder: &ArgumentEncoder,
        material_arg_encoder: &ArgumentEncoder,
    ) -> Self {
        let obj_file_ref = obj_file.as_ref();
        let (mut models, materials) = tobj::load_obj(
            obj_file_ref,
            &LoadOptions {
                single_index: true,
                triangulate: true,
                ignore_points: true,
                ignore_lines: true,
            },
        )
        .expect("Failed to load OBJ file");

        let (objects, object_geometries_buffer, object_geometry_arg_encoded_length, max_bounds) =
            load_object_geometries(&models, device, &object_geometry_arg_encoder);

        let materials = materials.expect("Failed to load materials data");
        let materials_buffer = load_materials(
            &materials,
            PathBuf::from(obj_file_ref).join(".."),
            device,
            &material_arg_encoder,
        );

        Self {
            objects: todo!(),
            object_geometries_buffer,
            object_geometry_arg_encoded_length,
            max_bounds,
            materials_buffer,
            material_arg_encoded_length: material_arg_encoder.encoded_length(),
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
