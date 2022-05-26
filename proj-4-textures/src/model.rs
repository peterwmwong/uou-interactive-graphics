use metal_app::{f32x4x4, metal::*};
use std::{
    path::{Path, PathBuf},
    simd::f32x4,
};
use tobj::{LoadOptions, Mesh};

pub enum AmbientDiffuseMaterial {
    Color(f32x4),
    Texture(Texture),
}

pub enum SpecularMaterial {
    Color { shineness: f32, color: f32x4 },
    Texture { shineness: f32, texture: Texture },
}

pub struct ModelObject {
    ambient_diffuse: AmbientDiffuseMaterial,
    specular: SpecularMaterial,
}

pub struct Model {
    objects: Vec<ModelObject>,
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

impl Model {
    pub fn from_file<T: AsRef<Path>>(obj_file: T, device: &Device) -> Self {
        let obj_file_ref = obj_file.as_ref();
        let obj_parent_dir = PathBuf::from(obj_file_ref)
            .parent()
            .expect("Could not get parent directory of object file");
        let (mut models, materials) = tobj::load_obj(
            obj_file.as_ref(),
            &LoadOptions {
                single_index: true,
                triangulate: true,
                ignore_points: true,
                ignore_lines: true,
            },
        )
        .expect("Failed to load OBJ file");

        let material = materials
            .expect("Failed to load materials data")
            .pop()
            .expect("Failed to load material, expected atleast one material");
        let specular_shineness = material.shininess;
        let texture_ambient_diffuse = load_texture_from_png(
            "Ambient/Diffuse",
            obj_parent_dir.join(material.ambient_texture),
            &device,
        );
        let texture_specular = load_texture_from_png(
            "Specular",
            obj_parent_dir.join(material.specular_texture),
            &device,
        );

        let model = models
            .pop()
            .expect("Failed to parse model, expecting atleast one model (teapot)");
        let Mesh {
            positions,
            indices,
            normals,
            texcoords,
            ..
        } = model.mesh;

        debug_assert_eq!(
            indices.len() % 3,
            0,
            "`mesh.indices` should contain triples (triangle vertices). Model should have been loaded with `triangulate`, guaranteeing all faces have 3 vertices."
        );
        debug_assert_eq!(
            positions.len() % 3,
            0,
            "`mesh.positions` should contain triples (3D position)"
        );
        debug_assert_eq!(
            normals.len(),
            positions.len(),
            "`mesh.normals` should contain triples (3D vector)"
        );
        debug_assert_eq!(
            texcoords.len() % 2,
            0,
            "`mesh.texcoords` should contain pairs (UV coordinates)"
        );
        debug_assert_eq!(
            texcoords.len() / 2,
            positions.len() / 3,
            "`mesh.texcoords` shoud contain UV coordinate for each position"
        );

        let (positions3, ..) = positions.as_chunks::<3>();
        let mut mins = f32x4::splat(f32::MAX);
        let mut maxs = f32x4::splat(f32::MIN);
        for &[x, y, z] in positions3 {
            let input = f32x4::from_array([x, y, z, 0.0]);
            mins = mins.min(input);
            maxs = maxs.max(input);
        }
        let max_bound = mins.reduce_min().abs().max(maxs.reduce_max());

        // TODO: START HERE
        // TODO: START HERE
        // TODO: START HERE
        // Use Argument Buffers
        // - metal-rs example: https://github.com/gfx-rs/metal-rs/blob/master/examples/argument-buffer/main.rs
        // - Apple Metal guide: https://developer.apple.com/documentation/metal/buffers/managing_groups_of_resources_with_argument_buffers
        // - Vertex Argument Buffer
        //   - What will the shader's structs look like?
        //   - Add parameters to from_file to allow caller to set the [[id(???)]] for...
        //      - positions
        //      - texture_coords
        //      - normals
        //      - material_id
        //   - Create/use an argument encoder
        //   - Create an argument buffer
        //   - Add getter fn geometry_argument_buffer()
        // - Fragment Argument Buffer
        //   - What will the shader's structs look like?
        //   - Add parameters to from_file to allow caller to set the [[id(???)]] for...
        //      - texture<2d> diffuse_texture
        //      - float4 diffuse_color
        //      - texture<2d> specular_texture
        //      - float4 specular_color
        //      - float specular_shineness
        //   - Create an argument encoder
        //   - Create an argument buffer
        //   - Add getter fn materials_argument_buffer()

        Self { objects: todo!() }
    }
}
