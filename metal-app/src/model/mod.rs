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

pub struct DrawIteratorItem<'a, G: Sized + Copy + Clone, M: Sized + Copy + Clone> {
    pub num_vertices: usize,
    pub geometry: (&'a TypedBuffer<G>, usize),
    pub material: Option<(&'a TypedBuffer<M>, usize)>,
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
    type Item = DrawIteratorItem<'a, G, MK::Argument>;
    fn next(&mut self) -> Option<Self::Item> {
        self.draw_i.next().map(|(i, d)| DrawIteratorItem {
            geometry: (self.geometry, i),
            material: self.material_draw_locator.get_material(d),
            num_vertices: d.num_indices,
        })
    }
}

pub trait MaterialLocatorFromDraw<M: Sized + Copy + Clone> {
    fn get_material(&self, draw: &DrawInfo) -> Option<(&TypedBuffer<M>, usize)>;
    fn legacy_encode_fragment<const FRAGMENT_MATERIAL_ARG_BUFFER_ID: u64>(
        &self,
        _encoder: &RenderCommandEncoderRef,
        _draw: &DrawInfo,
    ) {
    }
}

pub struct HasMaterial_MaterialLocatorFromDraw<M: Sized + Copy + Clone + 'static> {
    materials_arg_buffers: TypedBuffer<M>,
}

impl<M: Sized + Copy + Clone + 'static> MaterialLocatorFromDraw<M>
    for HasMaterial_MaterialLocatorFromDraw<M>
{
    fn get_material(&self, draw: &DrawInfo) -> Option<(&TypedBuffer<M>, usize)> {
        draw.material_id
            .map(|mid| (&self.materials_arg_buffers, mid))
    }

    fn legacy_encode_fragment<const FRAGMENT_MATERIAL_ARG_BUFFER_ID: u64>(
        &self,
        encoder: &RenderCommandEncoderRef,
        draw: &DrawInfo,
    ) {
        if let Some(material_id) = draw.material_id {
            encoder.set_fragment_buffer(
                FRAGMENT_MATERIAL_ARG_BUFFER_ID,
                Some(&self.materials_arg_buffers.buffer),
                (material_id * self.materials_arg_buffers.element_size()) as _,
            );
        }
    }
}

pub trait InitializedMaterials<Locator> {
    fn allocate_materials(self, heap: &Heap) -> Locator;
    fn heap_size(&self) -> usize;
}

pub struct HasMaterial_InitializedMaterials<
    'a,
    M: Sized + Copy + Clone,
    F: FnMut(&mut M, MaterialToEncode),
> {
    encoder: F,
    materials: Materials<'a, M>,
    heap_size: usize,
}

impl<'a, M: Sized + Copy + Clone, F: FnMut(&mut M, MaterialToEncode)>
    InitializedMaterials<HasMaterial_MaterialLocatorFromDraw<M>>
    for HasMaterial_InitializedMaterials<'a, M, F>
{
    fn allocate_materials(mut self, heap: &Heap) -> HasMaterial_MaterialLocatorFromDraw<M> {
        let results = self.materials.allocate_and_encode(heap, self.encoder);
        HasMaterial_MaterialLocatorFromDraw {
            materials_arg_buffers: results.arguments_buffer,
        }
    }

    fn heap_size(&self) -> usize {
        self.heap_size
    }
}

pub trait MaterialKind {
    type Argument: Sized + Copy + 'static;
    type Initialized<'a>: InitializedMaterials<Self::Locator>;
    type Locator: MaterialLocatorFromDraw<Self::Argument>;

    fn initialize_materials<'a, P: AsRef<Path>>(
        self,
        device: &Device,
        material_file_dir: P,
        tobj_materials: &'a [tobj::Material],
    ) -> Self::Initialized<'a>;
}

pub struct HasMaterial<M: Sized + Copy + Clone + 'static, F: FnMut(&mut M, MaterialToEncode)> {
    encoder: F,
    _m: PhantomData<M>,
}
impl<M: Sized + Copy + Clone + 'static, F: FnMut(&mut M, MaterialToEncode)> HasMaterial<M, F> {
    pub fn new(encoder: F) -> Self {
        Self {
            encoder,
            _m: PhantomData,
        }
    }
}

impl<M: Sized + Copy + Clone + 'static, F: FnMut(&mut M, MaterialToEncode)> MaterialKind
    for HasMaterial<M, F>
{
    type Argument = M;
    type Initialized<'a> = HasMaterial_InitializedMaterials<'a, Self::Argument, F>;
    type Locator = HasMaterial_MaterialLocatorFromDraw<M>;

    fn initialize_materials<'a, P: AsRef<Path>>(
        self,
        device: &Device,
        material_file_dir: P,
        tobj_materials: &'a [tobj::Material],
    ) -> Self::Initialized<'a> {
        let materials = Materials::new(device, material_file_dir, tobj_materials);
        let heap_size = materials.heap_size();
        HasMaterial_InitializedMaterials {
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
impl InitializedMaterials<NoMaterial> for NoMaterial {
    fn allocate_materials(self, _heap: &Heap) -> NoMaterial {
        NoMaterial
    }

    fn heap_size(&self) -> usize {
        0
    }
}

impl MaterialLocatorFromDraw<()> for NoMaterial {
    fn get_material(&self, _draw: &DrawInfo) -> Option<(&TypedBuffer<()>, usize)> {
        None
    }
}
pub struct Model<
    const VERTEX_GEOMETRY_ARG_BUFFER_ID: u64,
    G: Sized + Copy + Clone + 'static,
    MK: MaterialKind,
> {
    heap: Heap,
    draws: Vec<DrawInfo>,
    pub geometry_max_bounds: MaxBounds,
    geometry_buffers: GeometryBuffers<G>,
    material_locator: MK::Locator,
}

impl<
        const VERTEX_GEOMETRY_ARG_BUFFER_ID: u64,
        G: Sized + Copy + Clone + 'static,
        MK: MaterialKind,
    > Model<VERTEX_GEOMETRY_ARG_BUFFER_ID, G, MK>
{
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
    pub fn encode_draws<const FRAGMENT_MATERIAL_ARG_BUFFER_ID: u64>(
        &self,
        encoder: &RenderCommandEncoderRef,
    ) {
        self.encode_draws_with_primitive_type::<FRAGMENT_MATERIAL_ARG_BUFFER_ID>(
            encoder,
            MTLPrimitiveType::Triangle,
        );
    }
    #[inline]
    pub fn get_draws(&self) -> DrawIterator<'_, G, MK> {
        DrawIterator::new(&self.draws, &self.geometry_buffers, &self.material_locator)
    }

    #[inline]
    pub fn encode_draws_with_primitive_type<const FRAGMENT_MATERIAL_ARG_BUFFER_ID: u64>(
        &self,
        encoder: &RenderCommandEncoderRef,
        primitive_type: MTLPrimitiveType,
    ) {
        let mut geometry_arg_buffer_offset = 0;

        for d in &self.draws {
            encoder.push_debug_group(&d.debug_group_name);

            // For the first object, encode the vertex/fragment buffer.
            if geometry_arg_buffer_offset == 0 {
                encoder.set_vertex_buffer(
                    VERTEX_GEOMETRY_ARG_BUFFER_ID,
                    Some(&self.geometry_buffers.arguments.buffer),
                    0,
                );
                // TODO: Change condition to FRAGMENT_MATERIAL_ARG_BUFFER_ID != NO_MATERIALS
                // - Should generate better code
                self.material_locator
                    .legacy_encode_fragment::<FRAGMENT_MATERIAL_ARG_BUFFER_ID>(encoder, d);
            }
            // Subsequent objects, just move the vertex/fragment buffer offsets
            else {
                encoder.set_vertex_buffer_offset(
                    VERTEX_GEOMETRY_ARG_BUFFER_ID,
                    geometry_arg_buffer_offset as _,
                );
                self.material_locator
                    .legacy_encode_fragment::<FRAGMENT_MATERIAL_ARG_BUFFER_ID>(encoder, d);
            }
            geometry_arg_buffer_offset += self.geometry_buffers.arguments.element_size();

            encoder.draw_primitives(primitive_type, 0, d.num_indices as _);
            encoder.pop_debug_group();
        }
    }
}
