mod geometry;
mod materials;

use crate::metal::*;
use crate::time::debug_time;
use crate::typed_buffer::TypedBuffer;
pub use geometry::{DrawInfo, DrawInfoWithMaterial, GeometryToEncode, MaxBounds};
use geometry::{Geometry, GeometryBuffers};
pub use materials::MaterialToEncode;
use materials::Materials;
use std::path::{Path, PathBuf};
use tobj::LoadOptions;

pub struct DrawIteratorItem<'a, G: Sized + Copy + Clone, MI> {
    pub num_vertices: usize,
    pub geometry: (&'a TypedBuffer<G>, usize),
    pub material: MI,
}

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// Try to remove MaterialLocator
// - I suspect if we could pass Geometry a "DrawFactory", then we wouldn't need both a DrawIteratorItem and a DrawInfo
// - And we might be able to remove the `material` field all together when NoMaterial is selected.
pub trait MaterialLocator<D: DrawInfo> {
    type T<'a>
    where
        Self: 'a;
    fn get_material<'a>(&'a self, draw: &D) -> Self::T<'a>;
}

pub struct HasMaterialLocator<M: Sized + Copy + Clone + 'static> {
    materials: self::materials::MaterialResults<M>,
}

impl<M: Sized + Copy + Clone + 'static> MaterialLocator<DrawInfoWithMaterial>
    for HasMaterialLocator<M>
{
    type T<'a> = (&'a TypedBuffer<M>, usize)
    where
        Self: 'a;
    fn get_material<'a>(&'a self, draw: &DrawInfoWithMaterial) -> Self::T<'a> {
        (&self.materials.arguments_buffer, draw.material_id())
    }
}

pub trait MaterialsInitialized<Locator> {
    fn allocate_materials(self, heap: &Heap) -> Locator;
}

pub struct HasMaterialInitialized<'a, M: Sized + Copy + Clone> {
    encoder: fn(&mut M, MaterialToEncode),
    materials: Materials<'a, M>,
}

impl<'a, M: Sized + Copy + Clone> MaterialsInitialized<HasMaterialLocator<M>>
    for HasMaterialInitialized<'a, M>
{
    fn allocate_materials(mut self, heap: &Heap) -> HasMaterialLocator<M> {
        HasMaterialLocator {
            materials: self.materials.allocate_and_encode(heap, self.encoder),
        }
    }
}

pub trait MaterialKind {
    type Argument: Sized + Copy + 'static;
    type Initialized<'a>: MaterialsInitialized<Self::Locator>;
    type DrawInfo: DrawInfo;
    type Locator: MaterialLocator<Self::DrawInfo>;
    fn initialize_materials<'a, P: AsRef<Path>>(
        self,
        device: &Device,
        material_file_dir: P,
        tobj_materials: &'a [tobj::Material],
    ) -> (usize, Self::Initialized<'a>);
}

pub struct HasMaterial<M: Sized + Copy + Clone + 'static>(pub fn(&mut M, MaterialToEncode));

impl<M: Sized + Copy + Clone + 'static> MaterialKind for HasMaterial<M> {
    type Argument = M;
    type Initialized<'a> = HasMaterialInitialized<'a, Self::Argument>;
    type Locator = HasMaterialLocator<M>;
    type DrawInfo = DrawInfoWithMaterial;

    fn initialize_materials<'a, P: AsRef<Path>>(
        self,
        device: &Device,
        material_file_dir: P,
        tobj_materials: &'a [tobj::Material],
    ) -> (usize, Self::Initialized<'a>) {
        let materials = Materials::new(device, material_file_dir, tobj_materials);
        let heap_size = materials.heap_size();
        (
            heap_size,
            HasMaterialInitialized {
                encoder: self.0,
                materials,
            },
        )
    }
}

pub struct NoMaterial;
impl MaterialKind for NoMaterial {
    type Argument = ();
    type Initialized<'a> = NoMaterial;
    type Locator = NoMaterial;
    type DrawInfo = DrawInfoWithMaterial;

    fn initialize_materials<'a, P: AsRef<Path>>(
        self,
        _device: &Device,
        _material_file_dir: P,
        _tobj_materials: &'a [tobj::Material],
    ) -> (usize, Self::Initialized<'a>) {
        (0, NoMaterial)
    }
}
impl MaterialsInitialized<NoMaterial> for NoMaterial {
    fn allocate_materials(self, _heap: &Heap) -> NoMaterial {
        NoMaterial
    }
}
impl MaterialLocator<DrawInfoWithMaterial> for NoMaterial {
    type T<'a> = ()
    where
        Self: 'a;
    fn get_material(&self, _draw: &DrawInfoWithMaterial) -> () {}
}
pub struct Model<G: Sized + Copy + Clone + 'static, MK: MaterialKind> {
    heap: Heap,
    draws: Vec<MK::DrawInfo>,
    pub geometry_max_bounds: MaxBounds,
    geometry_buffers: GeometryBuffers<G>,
    material_locator: MK::Locator,
}

impl<G: Sized + Copy + Clone + 'static, MK: MaterialKind> Model<G, MK> {
    pub fn from_file<T: AsRef<Path>, EG: FnMut(&mut G, GeometryToEncode)>(
        obj_file: T,
        device: &Device,
        encode_geometry_arg: EG,
        material_kind: MK,
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
        // Size Heap for Geometry and Materials
        let (heap_size, initialized_materials) = debug_time("Model - Size Material", || {
            material_kind.initialize_materials(device, material_file_dir, &materials)
        });
        let mut geometry = debug_time("Model - Size Geometry", || Geometry::new(&models, device));

        // Allocate Heap for Geometry and Materials
        let desc = HeapDescriptor::new();
        desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
        desc.set_storage_mode(MTLStorageMode::Shared);
        desc.set_size((heap_size + geometry.heap_size()) as _);
        let heap = debug_time("Model - Allocate Model Heap", || device.new_heap(&desc));
        heap.set_label("Model Heap");

        // IMPORTANT: Load material textures *BEFORE* geometry. Heap size calculations
        // (specifically alignment padding) assume this.
        let material_locator = debug_time("Model - Load Material textures", || {
            initialized_materials.allocate_materials(&heap)
        });
        let geometry_buffers = debug_time("Model - Load Geometry", || {
            geometry.allocate_and_encode(&heap, encode_geometry_arg)
        });

        Self {
            heap,
            draws: geometry.draws,
            geometry_buffers,
            geometry_max_bounds: geometry.max_bounds,
            material_locator,
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
    pub fn get_draws(
        &self,
    ) -> impl Iterator<
        Item = DrawIteratorItem<'_, G, <MK::Locator as MaterialLocator<MK::DrawInfo>>::T<'_>>,
    > {
        self.draws
            .iter()
            .enumerate()
            .map(|(i, d)| DrawIteratorItem {
                num_vertices: d.num_indices(),
                geometry: (&self.geometry_buffers.arguments, i),
                material: self.material_locator.get_material(d),
            })
    }
}
