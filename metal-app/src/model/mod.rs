pub mod geometry;
mod materials;

use crate::metal::*;
use crate::time::debug_time;
use crate::typed_buffer::TypedBuffer;
use geometry::{Geometry, GeometryBuffers};
pub use geometry::{GeometryToEncode, MaxBounds};
pub use materials::MaterialToEncode;
use materials::{MaterialResults, Materials};
use std::path::{Path, PathBuf};
use tobj::LoadOptions;

pub trait MaterialKind {
    type Sizer<'a>;
    type Allocated;
    type Draw;
    type DrawItem<'a, G: Sized + Copy + Clone + 'a>;

    fn size<'a, P: AsRef<Path>>(
        &self,
        device: &Device,
        materials_dir: P,
        materials: &'a [tobj::Material],
    ) -> (usize, Self::Sizer<'a>);

    fn allocate(&self, heap: &Heap, sized: Self::Sizer<'_>) -> Self::Allocated;

    fn new_draw(name: String, vertex_count: usize, material_id: Option<usize>) -> Self::Draw;

    fn new_draw_item<'a, G: Sized + Copy + Clone>(
        materials: &'a Self::Allocated,
        geometries: &'a TypedBuffer<G>,
        draw: &'a Self::Draw,
        draw_index: usize,
    ) -> Self::DrawItem<'a, G>;
}

pub struct DrawItem<'a, G: Sized + Copy + Clone + 'a, M: Sized + Copy + Clone + 'a> {
    pub name: &'a str,
    pub vertex_count: usize,
    pub geometry: (&'a TypedBuffer<G>, usize),
    pub material: (&'a TypedBuffer<M>, usize),
}

pub struct Draw {
    name: String,
    vertex_count: usize,
    material_id: usize,
}

pub struct HasMaterial<M: Sized + Copy + Clone + 'static>(pub fn(&mut M, MaterialToEncode));
impl<M: Sized + Copy + Clone + 'static> MaterialKind for HasMaterial<M> {
    type Sizer<'a> = Materials<'a, M>;
    type Allocated = MaterialResults<M>;
    type Draw = Draw;
    type DrawItem<'a, G: Sized + Copy + Clone + 'a> = DrawItem<'a, G, M>;

    #[inline(always)]
    fn size<'a, P: AsRef<Path>>(
        &self,
        device: &Device,
        materials_dir: P,
        materials: &'a [tobj::Material],
    ) -> (usize, Self::Sizer<'a>) {
        let m = Materials::new(device, materials_dir, materials);
        (m.heap_size(), m)
    }

    #[inline(always)]
    fn allocate(&self, heap: &Heap, mut sized_materials: Self::Sizer<'_>) -> Self::Allocated {
        sized_materials.allocate_and_encode(heap, self.0)
    }

    #[inline(always)]
    fn new_draw(name: String, vertex_count: usize, material_id: Option<usize>) -> Self::Draw {
        Draw {
            name,
            vertex_count,
            material_id: material_id
                .expect("Expected geometry mesh to have an associated material"),
        }
    }

    #[inline(always)]
    fn new_draw_item<'a, G: Sized + Copy + Clone>(
        materials: &'a Self::Allocated,
        geometries: &'a TypedBuffer<G>,
        draw: &'a Self::Draw,
        draw_index: usize,
    ) -> Self::DrawItem<'a, G> {
        DrawItem {
            name: &draw.name,
            vertex_count: draw.vertex_count,
            geometry: (geometries, draw_index),
            material: (&materials.arguments_buffer, draw.material_id),
        }
    }
}

pub struct DrawNoMaterial {
    name: String,
    vertex_count: usize,
}

pub struct DrawItemNoMaterial<'a, G: Sized + Copy + Clone> {
    pub name: &'a str,
    pub vertex_count: usize,
    pub geometry: (&'a TypedBuffer<G>, usize),
}

pub struct NoMaterial;

impl MaterialKind for NoMaterial {
    type Sizer<'a> = ();
    type Allocated = ();
    type Draw = DrawNoMaterial;
    type DrawItem<'a, G: Sized + Copy + Clone + 'a> = DrawItemNoMaterial<'a, G>;

    #[inline(always)]
    fn size<'a, P: AsRef<Path>>(
        &self,
        _device: &Device,
        _materials_dir: P,
        _materials: &'a [tobj::Material],
    ) -> (usize, Self::Sizer<'a>) {
        (0, ())
    }

    #[inline(always)]
    fn allocate(&self, _heap: &Heap, mut _sized_materials: Self::Sizer<'_>) -> Self::Allocated {
        ()
    }

    #[inline(always)]
    fn new_draw(name: String, vertex_count: usize, _material_id: Option<usize>) -> Self::Draw {
        DrawNoMaterial { name, vertex_count }
    }

    #[inline(always)]
    fn new_draw_item<'a, G: Sized + Copy + Clone>(
        _materials: &'a Self::Allocated,
        geometries: &'a TypedBuffer<G>,
        draw: &'a Self::Draw,
        draw_index: usize,
    ) -> Self::DrawItem<'a, G> {
        DrawItemNoMaterial {
            name: &draw.name,
            vertex_count: draw.vertex_count,
            geometry: (geometries, draw_index),
        }
    }
}

pub struct Model<G: Sized + Copy + Clone, MK: MaterialKind> {
    pub heap: Heap,
    draws: Vec<MK::Draw>,
    pub geometry_max_bounds: MaxBounds,
    geometry_buffers: GeometryBuffers<G>,
    materials: MK::Allocated,
}

impl<G: Sized + Copy + Clone, MK: MaterialKind> Model<G, MK> {
    pub fn from_file<T: AsRef<Path>, EG: FnMut(&mut G, GeometryToEncode)>(
        obj_file: T,
        device: &Device,
        encode_geometry_arg: EG,
        material_kind: MK,
    ) -> Self {
        debug_time("Model::from_file", || {
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
            let (heap_size, sized_materials) =
                material_kind.size(device, material_file_dir, &materials);
            let mut geometry = Geometry::new(&models, device, MK::new_draw);

            // Allocate Heap for Geometry and Materials
            let desc = HeapDescriptor::new();
            desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
            desc.set_storage_mode(MTLStorageMode::Shared);
            desc.set_size((heap_size + geometry.heap_size()) as _);
            let heap = device.new_heap(&desc);
            heap.set_label(obj_file_ref.file_name().unwrap().to_str().unwrap());

            // IMPORTANT: Load material textures *BEFORE* geometry. Heap size calculations
            // (specifically alignment padding) assume this.
            let materials = material_kind.allocate(&heap, sized_materials);
            let geometry_buffers = geometry.allocate_and_encode(&heap, encode_geometry_arg);

            Self {
                heap,
                draws: geometry.draws,
                geometry_buffers,
                geometry_max_bounds: geometry.max_bounds,
                materials,
            }
        })
    }

    #[inline]
    pub fn draws(&self) -> impl Iterator<Item = MK::DrawItem<'_, G>> {
        self.draws.iter().enumerate().map(|(i, d)| {
            MK::new_draw_item(&self.materials, &self.geometry_buffers.arguments, d, i)
        })
    }
}
