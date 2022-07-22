mod geometry;
mod materials;

use crate::metal::*;
use crate::time::debug_time;
use crate::typed_buffer::TypedBuffer;
use geometry::{DrawInfo, Geometry, GeometryBuffers};
pub use geometry::{GeometryToEncode, MaxBounds};
pub use materials::MaterialToEncode;
use materials::Materials;
use std::iter::Enumerate;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::slice::Iter;
use tobj::LoadOptions;

pub struct DrawIteratorItem<'a, G: Sized + Copy + Clone, MI> {
    pub num_vertices: usize,
    pub geometry: (&'a TypedBuffer<G>, usize),
    pub material: MI,
}
pub struct DrawIterator<'a, G: Sized + Copy + Clone, MK: MaterialKind> {
    draw_i: Enumerate<Iter<'a, DrawInfo>>,
    geometry: &'a TypedBuffer<G>,
    material_draw_locator: &'a MK::Locator,
}

impl<'a, G: Sized + Copy + Clone, MK: MaterialKind> DrawIterator<'a, G, MK> {
    fn new(
        draws: &'a [DrawInfo],
        geometry_buffers: &'a GeometryBuffers<G>,
        material_draw_locator: &'a MK::Locator,
    ) -> Self {
        Self {
            draw_i: draws.iter().enumerate(),
            geometry: &geometry_buffers.arguments,
            material_draw_locator,
        }
    }
}

impl<'a, G: Sized + Copy + Clone, MK: MaterialKind> Iterator for DrawIterator<'a, G, MK> {
    type Item = DrawIteratorItem<'a, G, <<MK as MaterialKind>::Locator as MaterialLocator>::T<'a>>;
    fn next(&mut self) -> Option<Self::Item> {
        self.draw_i.next().map(|(i, d)| DrawIteratorItem {
            geometry: (self.geometry, i),
            material: self.material_draw_locator.get_material(d),
            num_vertices: d.num_indices,
        })
    }
}

pub trait MaterialLocator {
    type T<'a>
    where
        Self: 'a;
    fn get_material<'a>(&'a self, draw: &DrawInfo) -> Self::T<'a>;
}

pub struct HasMaterialLocator<M: Sized + Copy + Clone + 'static> {
    materials: self::materials::MaterialResults<M>,
}

impl<M: Sized + Copy + Clone + 'static> MaterialLocator for HasMaterialLocator<M> {
    type T<'a> = (&'a TypedBuffer<M>, usize)
    where
        Self: 'a;

    fn get_material<'a>(&'a self, draw: &DrawInfo) -> Self::T<'a> {
        draw.material_id
            .map(|mid| (&self.materials.arguments_buffer, mid))
            // TODO: START HERE
            // TODO: START HERE
            // TODO: START HERE
            // We should be able to remove the Option/expect().
            // DrawInfo should be generic to know whether material_id is a thing or not.
            .expect("Failed to get material for draw")
    }
}

pub trait MaterialsInitialized<Locator> {
    fn allocate_materials(self, heap: &Heap) -> Locator;
    fn heap_size(&self) -> usize;
}

pub struct HasMaterialInitialized<'a, M: Sized + Copy + Clone> {
    encoder: fn(&mut M, MaterialToEncode),
    materials: Materials<'a, M>,
    heap_size: usize,
}

impl<'a, M: Sized + Copy + Clone> MaterialsInitialized<HasMaterialLocator<M>>
    for HasMaterialInitialized<'a, M>
{
    fn allocate_materials(mut self, heap: &Heap) -> HasMaterialLocator<M> {
        HasMaterialLocator {
            materials: self.materials.allocate_and_encode(heap, self.encoder),
        }
    }

    fn heap_size(&self) -> usize {
        self.heap_size
    }
}

pub trait MaterialKind {
    type Argument: Sized + Copy + 'static;
    type Initialized<'a>: MaterialsInitialized<Self::Locator>;
    type Locator: MaterialLocator;

    fn initialize_materials<'a, P: AsRef<Path>>(
        self,
        device: &Device,
        material_file_dir: P,
        tobj_materials: &'a [tobj::Material],
    ) -> Self::Initialized<'a>;
}

pub struct HasMaterial<M: Sized + Copy + Clone + 'static> {
    encoder: fn(&mut M, MaterialToEncode),
    _m: PhantomData<M>,
}
impl<M: Sized + Copy + Clone + 'static> HasMaterial<M> {
    pub fn new(encoder: fn(&mut M, MaterialToEncode)) -> Self {
        Self {
            encoder,
            _m: PhantomData,
        }
    }
}

impl<M: Sized + Copy + Clone + 'static> MaterialKind for HasMaterial<M> {
    type Argument = M;
    type Initialized<'a> = HasMaterialInitialized<'a, Self::Argument>;
    type Locator = HasMaterialLocator<M>;

    fn initialize_materials<'a, P: AsRef<Path>>(
        self,
        device: &Device,
        material_file_dir: P,
        tobj_materials: &'a [tobj::Material],
    ) -> Self::Initialized<'a> {
        let materials = Materials::new(device, material_file_dir, tobj_materials);
        let heap_size = materials.heap_size();
        HasMaterialInitialized {
            encoder: self.encoder,
            materials,
            heap_size,
        }
    }
}

pub struct NoMaterial;
impl MaterialKind for NoMaterial {
    type Argument = ();
    type Initialized<'a> = NoMaterial;
    type Locator = NoMaterial;

    fn initialize_materials<'a, P: AsRef<Path>>(
        self,
        _device: &Device,
        _material_file_dir: P,
        _tobj_materials: &'a [tobj::Material],
    ) -> Self::Initialized<'a> {
        NoMaterial
    }
}
impl MaterialsInitialized<NoMaterial> for NoMaterial {
    fn allocate_materials(self, _heap: &Heap) -> NoMaterial {
        NoMaterial
    }

    fn heap_size(&self) -> usize {
        0
    }
}
impl MaterialLocator for NoMaterial {
    type T<'a> = ()
    where
        Self: 'a;
    fn get_material(&self, _draw: &DrawInfo) -> () {}
}
pub struct Model<G: Sized + Copy + Clone + 'static, MK: MaterialKind> {
    heap: Heap,
    draws: Vec<DrawInfo>,
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
        let initialized_materials = debug_time("Model - Size Material", || {
            material_kind.initialize_materials(device, material_file_dir, &materials)
        });
        let mut geometry = debug_time("Model - Size Geometry", || Geometry::new(&models, device));

        // Allocate Heap for Geometry and Materials
        let desc = HeapDescriptor::new();
        desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
        desc.set_storage_mode(MTLStorageMode::Shared);
        desc.set_size((initialized_materials.heap_size() + geometry.heap_size()) as _);
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
    pub fn get_draws(&self) -> DrawIterator<'_, G, MK> {
        DrawIterator::new(&self.draws, &self.geometry_buffers, &self.material_locator)
    }
}
