mod geometry;
mod materials;

use crate::metal::*;
use crate::time::debug_time;
use crate::typed_buffer::TypedBuffer;
use geometry::{DrawInfo, Geometry, GeometryBuffers};
pub use geometry::{GeometryToEncode, MaxBounds};
pub use materials::MaterialToEncode;
use materials::{MaterialResults, Materials};
use std::any::TypeId;
use std::iter::Enumerate;
use std::path::{Path, PathBuf};
use std::slice::Iter;
use tobj::LoadOptions;

pub const NO_MATERIALS_ID: u64 = u64::MAX;

#[derive(Copy, Clone)]
pub struct NoMaterial {}
#[allow(non_snake_case)]
pub fn NO_MATERIALS_ENCODER(_: &mut NoMaterial, _: MaterialToEncode) {}

pub trait Iterator {
    type Item<'a>
    where
        Self: 'a;
    fn next(&mut self) -> Option<Self::Item<'_>>;
}

pub struct DrawIteratorItem<'a, G: Sized + Copy + Clone, M: Sized + Copy + Clone> {
    pub num_vertices: usize,
    pub geometry: (&'a TypedBuffer<G>, usize),
    pub material: Option<(&'a TypedBuffer<M>, usize)>,
}
pub struct DrawIterator<'a, G: Sized + Copy + Clone, M: Sized + Copy + Clone> {
    draw_i: Enumerate<Iter<'a, DrawInfo>>,
    geometry: &'a TypedBuffer<G>,
    material: Option<&'a TypedBuffer<M>>,
}

impl<'a, G: Sized + Copy + Clone, M: Sized + Copy + Clone> DrawIterator<'a, G, M> {
    fn new(
        draws: &'a [DrawInfo],
        geometry_buffers: &'a GeometryBuffers<G>,
        materials: Option<&'a MaterialResults<M>>,
    ) -> Self {
        Self {
            draw_i: draws.iter().enumerate(),
            geometry: &geometry_buffers.arguments,
            material: materials.as_ref().map(|m| &m.arguments_buffer),
        }
    }
}

impl<'a, G: Sized + Copy + Clone, M: Sized + Copy + Clone> Iterator for DrawIterator<'a, G, M> {
    type Item<'b> = DrawIteratorItem<'a, G , M>
    where
        Self: 'b;
    fn next(&mut self) -> Option<Self::Item<'_>> {
        self.draw_i.next().map(|(i, d)| DrawIteratorItem {
            geometry: (self.geometry, i),
            material: self.material.zip(d.material_id),
            num_vertices: d.num_indices,
        })
    }
}

pub struct Model<
    const VERTEX_GEOMETRY_ARG_BUFFER_ID: u64,
    const FRAGMENT_MATERIAL_ARG_BUFFER_ID: u64,
    G: Sized + Copy + Clone + 'static,
    M: Sized + Copy + Clone + 'static,
> {
    heap: Heap,
    draws: Vec<DrawInfo>,
    pub geometry_max_bounds: MaxBounds,
    geometry_buffers: GeometryBuffers<G>,
    materials: Option<MaterialResults<M>>,
}

impl<
        const VERTEX_GEOMETRY_ARG_BUFFER_ID: u64,
        const FRAGMENT_MATERIAL_ARG_BUFFER_ID: u64,
        G: Sized + Copy + Clone + 'static,
        M: Sized + Copy + Clone + 'static,
    > Model<VERTEX_GEOMETRY_ARG_BUFFER_ID, FRAGMENT_MATERIAL_ARG_BUFFER_ID, G, M>
{
    pub fn from_file<
        T: AsRef<Path>,
        EG: FnMut(&mut G, GeometryToEncode),
        EM: FnMut(&mut M, MaterialToEncode),
    >(
        obj_file: T,
        device: &Device,
        encode_geometry_arg: EG,
        encode_material_arg: EM,
    ) -> Self {
        let obj_file_ref = obj_file.as_ref();
        let (models, materials) = debug_time("Model - Load OBJ", || {
            tobj::load_obj(
                obj_file_ref,
                &LoadOptions {
                    single_index: true,
                    triangulate: true,
                    ignore_points: true,
                    ignore_lines: true,
                },
            )
            .expect("Failed to load OBJ file")
        });

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
                Some(debug_time("Model - Size Material", || {
                    Materials::new(device, &material_file_dir, &materials)
                }))
            };
        let mut geometry = debug_time("Model - Size Geometry", || Geometry::new(&models, device));

        // Allocate Heap for Geometry and Materials
        let desc = HeapDescriptor::new();
        desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
        desc.set_storage_mode(MTLStorageMode::Shared);
        let material_heap_size = materials.as_ref().map_or(0, |m| m.heap_size());
        desc.set_size((material_heap_size + geometry.heap_size()) as _);
        let heap = debug_time("Model - Allocate Model Heap", || device.new_heap(&desc));
        heap.set_label("Model Heap");

        // IMPORTANT: Load material textures *BEFORE* geometry. Heap size calculations
        // (specifically alignment padding) assume this.
        let materials = debug_time("Model - Load Material textures", || {
            materials
                .as_mut()
                .map(|m| m.allocate_and_encode(&heap, encode_material_arg))
        });
        let geometry_buffers = debug_time("Model - Load Geometry", || {
            geometry.allocate_and_encode(&heap, encode_geometry_arg)
        });

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
        self.encode_draws_with_primitive_type(encoder, MTLPrimitiveType::Triangle);
    }
    #[inline]
    pub fn get_draws(&self) -> DrawIterator<'_, G, M> {
        DrawIterator::new(&self.draws, &self.geometry_buffers, self.materials.as_ref())
    }

    #[inline]
    pub fn encode_draws_with_primitive_type(
        &self,
        encoder: &RenderCommandEncoderRef,
        primitive_type: MTLPrimitiveType,
    ) {
        let mut geometry_arg_buffer_offset = 0;
        let materials = self.materials.as_ref();
        for d in &self.draws {
            encoder.push_debug_group(&d.debug_group_name);

            let material_arg_buffer_offset = d.material_id.zip(materials).map(|(mid, m)| {
                (
                    mid * m.arguments_buffer.element_size(),
                    &m.arguments_buffer.buffer,
                )
            });

            // For the first object, encode the vertex/fragment buffer.
            if geometry_arg_buffer_offset == 0 {
                encoder.set_vertex_buffer(
                    VERTEX_GEOMETRY_ARG_BUFFER_ID,
                    Some(&self.geometry_buffers.arguments.buffer),
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
            geometry_arg_buffer_offset += self.geometry_buffers.arguments.element_size();

            encoder.draw_primitives(primitive_type, 0, d.num_indices as _);
            encoder.pop_debug_group();
        }
    }
}
