mod geometry;
mod heap_resident;
mod materials;

use self::heap_resident::HeapResident;
use crate::metal::*;
pub use geometry::MaxBounds;
use geometry::{DrawInfo, Geometry};
use materials::Materials;
use std::path::{Path, PathBuf};
use tobj::LoadOptions;

pub struct Model {
    heap: Heap,
    draws: Vec<DrawInfo>,
    geometry_arg_buffer: Buffer,
    geometry_arg_encoded_length: u32,
    pub geometry_max_bounds: MaxBounds,
    // TODO: Create a type (GeometryBuffers { indices, positions, normals, tx_coords })
    #[allow(dead_code)]
    geometry_buffers: [Buffer; 4],
    materials_arg_buffer: Buffer,
    materials_arg_encoded_length: u32,
    #[allow(dead_code)]
    material_textures: Vec<Texture>,
}

impl Model {
    pub fn from_file<
        T: AsRef<Path>,
        const GEOMETRY_ID_INDICES_BUFFER: u16,
        const GEOMETRY_ID_POSITIONS_BUFFER: u16,
        const GEOMETRY_ID_NORMALS_BUFFER: u16,
        const GEOMETRY_ID_TX_COORDS_BUFFER: u16,
        const MATERIAL_ID_AMBIENT_TEXTURE: u16,
        const MATERIAL_ID_DIFFUSE_TEXTURE: u16,
        const MATERIAL_ID_SPECULAR_TEXTURE: u16,
        const MATERIAL_ID_SPECULAR_SHINENESS: u16,
    >(
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
        let mut materials = Materials::<
            MATERIAL_ID_AMBIENT_TEXTURE,
            MATERIAL_ID_DIFFUSE_TEXTURE,
            MATERIAL_ID_SPECULAR_TEXTURE,
            MATERIAL_ID_SPECULAR_SHINENESS,
        >::new(device, &material_file_dir, &materials);
        let mut geometry = Geometry::<
            GEOMETRY_ID_INDICES_BUFFER,
            GEOMETRY_ID_POSITIONS_BUFFER,
            GEOMETRY_ID_NORMALS_BUFFER,
            GEOMETRY_ID_TX_COORDS_BUFFER,
        >::new(&models, device);

        // Allocate Heap for Geometry and Materials
        let desc = HeapDescriptor::new();
        desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
        desc.set_storage_mode(MTLStorageMode::Shared);
        desc.set_size((materials.heap_size() + geometry.heap_size()) as _);
        let heap = device.new_heap(&desc);
        heap.set_label("Geometry and Materials Heap");

        // IMPORTANT: Load material textures *BEFORE* geometry. Heap size calculations
        // (specifically alignment padding) assume this.
        let (materials_arg_buffer, materials_arg_encoded_length, material_textures) =
            materials.allocate_and_encode(&heap, device, materials_arg_encoder);

        let (geometry_arg_buffer, geometry_arg_encoded_length, geometry_buffers) =
            geometry.allocate_and_encode(&heap, device, geometry_arg_encoder);

        Self {
            heap,
            draws: geometry.draws,
            geometry_arg_buffer,
            geometry_arg_encoded_length,
            geometry_buffers,
            geometry_max_bounds: geometry.max_bounds,
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
        for d in &self.draws {
            encoder.push_debug_group(&d.debug_group_name);

            let material_arg_buffer_offset = d.material_id * self.materials_arg_encoded_length;

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

            encoder.draw_primitives(MTLPrimitiveType::Triangle, 0, d.num_indices as _);

            encoder.pop_debug_group();
        }
    }
}
