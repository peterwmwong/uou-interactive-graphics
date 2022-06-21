mod geometry;
mod materials;

use crate::metal::*;
use geometry::{DrawInfo, Geometry, GeometryBuffers};
pub use geometry::{GeometryArgumentEncoder, MaxBounds};
pub use materials::MaterialArgumentEncoder;
use materials::{MaterialResults, Materials};
use std::path::{Path, PathBuf};
use tobj::LoadOptions;

pub struct Model {
    heap: Heap,
    draws: Vec<DrawInfo>,
    pub geometry_max_bounds: MaxBounds,
    // Needs to be owned and not dropped (causing deallocation from heap).
    #[allow(dead_code)]
    geometry_buffers: GeometryBuffers,
    // Needs to be owned and not dropped (causing deallocation from heap).
    #[allow(dead_code)]
    materials: MaterialResults,
}

impl Model {
    // TODO: START HERE
    // TODO: START HERE
    // TODO: START HERE
    // Change api to take in a GeometryArgumentEncoder, MaterialArgumentEncode (new traits), that
    // allow using common.h generated struct AND metal 3 bindless.
    pub fn from_file<
        T: AsRef<Path>,
        TG: Sized,
        G: GeometryArgumentEncoder<TG>,
        TM: Sized,
        M: MaterialArgumentEncoder<TM>,
    >(
        obj_file: T,
        device: &Device,
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
        let mut materials = Materials::<TM, M>::new(device, &material_file_dir, &materials);
        let mut geometry = Geometry::<TG, G>::new(&models, device);

        // Allocate Heap for Geometry and Materials
        let desc = HeapDescriptor::new();
        desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
        desc.set_storage_mode(MTLStorageMode::Shared);
        desc.set_size((materials.heap_size() + geometry.heap_size()) as _);
        let heap = device.new_heap(&desc);
        heap.set_label("Geometry and Materials Heap");

        // IMPORTANT: Load material textures *BEFORE* geometry. Heap size calculations
        // (specifically alignment padding) assume this.
        let materials = materials.allocate_and_encode(&heap);
        let geometry_buffers = geometry.allocate_and_encode(&heap);

        Self {
            heap,
            draws: geometry.draws,
            geometry_buffers,
            geometry_max_bounds: geometry.max_bounds,
            materials,
        }
    }

    #[inline]
    pub fn encode_use_resources(&self, encoder: &RenderCommandEncoderRef) {
        encoder.use_heap_at(
            &self.heap,
            MTLRenderStages::Vertex | MTLRenderStages::Fragment,
        )
    }

    // TODO: START HERE 4
    // TODO: START HERE 4
    // TODO: START HERE 4
    // Change vertex_geometry_arg_buffer_id and fragment_material_arg_buffer_id as generic constants
    #[inline]
    pub fn encode_draws(
        &self,
        encoder: &RenderCommandEncoderRef,
        vertex_geometry_arg_buffer_id: usize,
        fragment_material_arg_buffer_id: usize,
    ) {
        let mut geometry_arg_buffer_offset = 0;
        for d in &self.draws {
            encoder.push_debug_group(&d.debug_group_name);

            let material_arg_buffer_offset = d.material_id * self.materials.argument_byte_size;

            // For the first object, encode the vertex/fragment buffer.
            if geometry_arg_buffer_offset == 0 {
                encoder.set_vertex_buffer(
                    vertex_geometry_arg_buffer_id as _,
                    Some(self.geometry_buffers.arguments.as_ref()),
                    0,
                );
                encoder.set_fragment_buffer(
                    fragment_material_arg_buffer_id as _,
                    Some(self.materials.arguments.as_ref()),
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
            geometry_arg_buffer_offset += self.geometry_buffers.argument_byte_size;

            encoder.draw_primitives(MTLPrimitiveType::Triangle, 0, d.num_indices as _);

            encoder.pop_debug_group();
        }
    }
}
