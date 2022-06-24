mod geometry;
mod materials;

use crate::metal::*;
pub use geometry::GeometryToEncode;
pub use geometry::MaxBounds;
use geometry::{DrawInfo, Geometry, GeometryBuffers};
pub use materials::MaterialToEncode;
use materials::{MaterialResults, Materials};
use std::any::TypeId;
use std::path::{Path, PathBuf};
use tobj::LoadOptions;

pub const NO_MATERIALS_ID: u64 = u64::MAX;

pub struct NoMaterial {}
#[allow(non_snake_case)]
pub fn NO_MATERIALS_ENCODER(_: &mut NoMaterial, _: MaterialToEncode) {}

pub struct Model<
    const VERTEX_GEOMETRY_ARG_BUFFER_ID: u64,
    const FRAGMENT_MATERIAL_ARG_BUFFER_ID: u64,
> {
    heap: Heap,
    draws: Vec<DrawInfo>,
    pub geometry_max_bounds: MaxBounds,
    geometry_buffers: GeometryBuffers,
    materials: Option<MaterialResults>,
}

impl<const VERTEX_GEOMETRY_ARG_BUFFER_ID: u64, const FRAGMENT_MATERIAL_ARG_BUFFER_ID: u64>
    Model<VERTEX_GEOMETRY_ARG_BUFFER_ID, FRAGMENT_MATERIAL_ARG_BUFFER_ID>
{
    pub fn from_file<
        T: AsRef<Path>,
        G: Sized + 'static,
        M: Sized + 'static,
        EG: FnMut(&mut G, GeometryToEncode),
        EM: FnMut(&mut M, MaterialToEncode),
    >(
        obj_file: T,
        device: &Device,
        encode_geometry_arg: EG,
        encode_material_arg: EM,
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

        debug_assert_eq!(
            FRAGMENT_MATERIAL_ARG_BUFFER_ID == NO_MATERIALS_ID,
            TypeId::of::<M>() == TypeId::of::<NoMaterial>(),
            r#"
Only one of these must be true:
1. FRAGMENT_MATERIAL_ARG_BUFFER_ID != NO_MATERIALS AND Material type is not ()
2. FRAGMENT_MATERIAL_ARG_BUFFER_ID == NO_MATERIALS AND Material type is ()
"#
        );

        // Size Heap for Geometry and Materials
        let mut materials =
            if FRAGMENT_MATERIAL_ARG_BUFFER_ID == NO_MATERIALS_ID || materials.is_empty() {
                None
            } else {
                Some(Materials::new(device, &material_file_dir, &materials))
            };
        let mut geometry = Geometry::new(&models, device);

        // Allocate Heap for Geometry and Materials
        let desc = HeapDescriptor::new();
        desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
        desc.set_storage_mode(MTLStorageMode::Shared);
        let material_heap_size = materials.as_ref().map_or(0, |m| m.heap_size());
        desc.set_size((material_heap_size + geometry.heap_size()) as _);
        let heap = device.new_heap(&desc);
        heap.set_label("Model Heap");

        // IMPORTANT: Load material textures *BEFORE* geometry. Heap size calculations
        // (specifically alignment padding) assume this.
        let materials = materials
            .as_mut()
            .map(|m| m.allocate_and_encode(&heap, encode_material_arg));
        let geometry_buffers = geometry.allocate_and_encode(&heap, encode_geometry_arg);

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

    #[inline]
    pub fn encode_draws(&self, encoder: &RenderCommandEncoderRef) {
        let mut geometry_arg_buffer_offset = 0;
        let materials = self.materials.as_ref();
        for d in &self.draws {
            encoder.push_debug_group(&d.debug_group_name);

            let material_arg_buffer_offset = d
                .material_id
                .zip(materials)
                .map(|(mid, m)| (mid * m.argument_byte_size, &m.arguments));

            // For the first object, encode the vertex/fragment buffer.
            if geometry_arg_buffer_offset == 0 {
                encoder.set_vertex_buffer(
                    VERTEX_GEOMETRY_ARG_BUFFER_ID,
                    Some(self.geometry_buffers.arguments.as_ref()),
                    0,
                );
                // TODO: Change condition to FRAGMENT_MATERIAL_ARG_BUFFER_ID != NO_MATERIALS
                // - Should generate better code
                if let Some((material_arg_buffer_offset, materials_arguments)) =
                    material_arg_buffer_offset
                {
                    encoder.set_fragment_buffer(
                        FRAGMENT_MATERIAL_ARG_BUFFER_ID,
                        Some(&materials_arguments),
                        material_arg_buffer_offset as _,
                    );
                }
            }
            // Subsequent objects, just move the vertex/fragment buffer offsets
            else {
                encoder.set_vertex_buffer_offset(
                    VERTEX_GEOMETRY_ARG_BUFFER_ID,
                    geometry_arg_buffer_offset as _,
                );
                if let Some((material_arg_buffer_offset, ..)) = material_arg_buffer_offset {
                    encoder.set_fragment_buffer_offset(
                        FRAGMENT_MATERIAL_ARG_BUFFER_ID,
                        material_arg_buffer_offset as _,
                    );
                }
            }
            geometry_arg_buffer_offset += self.geometry_buffers.argument_byte_size;

            encoder.draw_primitives(MTLPrimitiveType::Triangle, 0, d.num_indices as _);

            encoder.pop_debug_group();
        }
    }
}
