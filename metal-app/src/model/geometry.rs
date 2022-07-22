use crate::{
    get_gpu_addresses,
    metal::*,
    rolling_copy,
    typed_buffer::{TypedBuffer, TypedBufferSizer},
    MetalGPUAddress, DEFAULT_RESOURCE_OPTIONS,
};
use std::{marker::PhantomData, ops::Deref, simd::f32x4};

pub struct MaxBounds {
    pub center: f32x4,
    pub size: f32x4,
}

pub trait DrawInfo {
    fn new(debug_group_name: String, num_indices: usize, material_id: Option<usize>) -> Self;
    fn debug_group_name(&self) -> &str;
    fn num_indices(&self) -> usize;
}

pub struct DrawInfoWithMaterial {
    debug_group_name: String,
    num_indices: usize,
    material_id: usize,
}
impl DrawInfo for DrawInfoWithMaterial {
    fn new(debug_group_name: String, num_indices: usize, material_id: Option<usize>) -> Self {
        Self {
            debug_group_name,
            num_indices,
            material_id: material_id.expect("Expected Material ID from Geometry mesh."),
        }
    }
    fn debug_group_name(&self) -> &str {
        &self.debug_group_name
    }
    fn num_indices(&self) -> usize {
        self.num_indices
    }
}

impl DrawInfoWithMaterial {
    pub fn material_id(&self) -> usize {
        self.material_id
    }
}

pub struct DrawInfoNoMaterial {
    debug_group_name: String,
    num_indices: usize,
}
impl DrawInfo for DrawInfoNoMaterial {
    fn new(debug_group_name: String, num_indices: usize, _material_id: Option<usize>) -> Self {
        Self {
            debug_group_name,
            num_indices,
        }
    }
    fn debug_group_name(&self) -> &str {
        &self.debug_group_name
    }
    fn num_indices(&self) -> usize {
        self.num_indices
    }
}

pub struct GeometryToEncode {
    pub indices_buffer: MetalGPUAddress,
    pub positions_buffer: MetalGPUAddress,
    pub normals_buffer: MetalGPUAddress,
    pub tx_coords_buffer: MetalGPUAddress,
}

pub(crate) struct GeometryBuffers<T: Sized + Copy + Clone> {
    pub(crate) arguments: TypedBuffer<T>,
    // Each buffer needs to be owned and not dropped (causing deallocation from the owning MTLHeap).
    _indices: TypedBuffer<u32>,
    _positions: TypedBuffer<f32>,
    _normals: TypedBuffer<f32>,
    _tx_coords: TypedBuffer<f32>,
}

pub(crate) struct Geometry<'a, T: Sized + Copy + Clone, D: DrawInfo> {
    objects: &'a [tobj::Model],
    arguments_sizer: TypedBufferSizer<T>,
    indices_sizer: TypedBufferSizer<u32>,
    positions_sizer: TypedBufferSizer<f32>,
    normals_sizer: TypedBufferSizer<f32>,
    tx_coords_sizer: TypedBufferSizer<f32>,
    heap_size: usize,
    pub(crate) max_bounds: MaxBounds,
    pub(crate) draws: Vec<D>,
    _p: PhantomData<T>,
}

impl<'a, T: Sized + Copy + Clone, D: DrawInfo> Geometry<'a, T, D> {
    pub(crate) fn new(objects: &'a [tobj::Model], device: &Device) -> Self {
        let mut heap_size = 0;

        // Create a shared buffer (shared between all objects).
        // Calculate the size of each buffer...
        let mut indices_sizer = TypedBufferSizer::new(0, DEFAULT_RESOURCE_OPTIONS);
        let mut positions_sizer = TypedBufferSizer::new(0, DEFAULT_RESOURCE_OPTIONS);
        let mut normals_sizer = TypedBufferSizer::new(0, DEFAULT_RESOURCE_OPTIONS);
        let mut tx_coords_sizer = TypedBufferSizer::new(0, DEFAULT_RESOURCE_OPTIONS);
        let mut mins = f32x4::splat(f32::MAX);
        let mut maxs = f32x4::splat(f32::MIN);
        let mut draws = Vec::<D>::with_capacity(objects.len());
        for tobj::Model { mesh, name, .. } in objects {
            assert!(
                (mesh.indices.len() % 3) == 0 &&
                (mesh.positions.len() % 3) == 0 &&
                (mesh.normals.len() % 3) == 0 &&
                (mesh.texcoords.len() % 2) == 0,
                "Unexpected number of positions, normals, or texcoords. Expected each to be triples, triples, and pairs (respectively)"
            );
            let num_positions = mesh.positions.len() / 3;
            assert!(
                (mesh.normals.len() / 3) == num_positions &&
                (mesh.texcoords.len() / 2) == num_positions,
                "Unexpected number of positions, normals, or texcoords. Expected each to be the number of indices"
            );
            indices_sizer.num_of_elements += mesh.indices.len();
            positions_sizer.num_of_elements += mesh.positions.len();
            normals_sizer.num_of_elements += mesh.normals.len();
            tx_coords_sizer.num_of_elements += mesh.texcoords.len();
            draws.push(D::new(
                name.to_owned(),
                mesh.indices.len(),
                mesh.material_id,
            ));
            for &[x, y, z] in mesh.positions.as_chunks::<3>().0 {
                let input = f32x4::from_array([x, y, z, 0.0]);
                mins = mins.min(input);
                maxs = maxs.max(input);
            }
        }
        heap_size += indices_sizer.heap_aligned_byte_size(device);
        heap_size += positions_sizer.heap_aligned_byte_size(device);
        heap_size += normals_sizer.heap_aligned_byte_size(device);
        heap_size += tx_coords_sizer.heap_aligned_byte_size(device);
        let arguments_sizer = TypedBufferSizer::<T>::new(objects.len(), DEFAULT_RESOURCE_OPTIONS);
        heap_size += arguments_sizer.heap_aligned_byte_size(device);
        let size = maxs - mins;
        let center = mins + (size * f32x4::splat(0.5));
        Self {
            arguments_sizer,
            heap_size,
            indices_sizer,
            positions_sizer,
            normals_sizer,
            tx_coords_sizer,
            objects,
            draws,
            max_bounds: MaxBounds { center, size },
            _p: PhantomData,
        }
    }

    #[inline]
    pub fn heap_size(&self) -> usize {
        self.heap_size
    }

    pub fn allocate_and_encode(
        &mut self,
        heap: &Heap,
        mut encode_arg: impl FnMut(&mut T, GeometryToEncode),
    ) -> GeometryBuffers<T> {
        let arguments_buffer = self.arguments_sizer.allocate("Geometry", heap.deref());
        let arguments = arguments_buffer.get_mut();
        let indices_buf = self.indices_sizer.allocate("indices", heap.deref());
        let mut indices = indices_buf.get_mut();
        let positions_buf = self.positions_sizer.allocate("positions", heap.deref());
        let mut positions = positions_buf.get_mut();
        let normals_buf = self.normals_sizer.allocate("normals", heap.deref());
        let mut normals = normals_buf.get_mut();
        let tx_coords_buf = self.tx_coords_sizer.allocate("tx_coords", heap.deref());
        let mut tx_coords = tx_coords_buf.get_mut();
        let [mut indices_gpu_address, mut positions_gpu_address, mut normals_gpu_address, mut tx_coords_gpu_address] =
            get_gpu_addresses([
                &indices_buf.buffer,
                &positions_buf.buffer,
                &normals_buf.buffer,
                &tx_coords_buf.buffer,
            ]);
        for (i, tobj::Model { mesh, .. }) in self.objects.into_iter().enumerate() {
            encode_arg(
                &mut arguments[i],
                GeometryToEncode {
                    indices_buffer: indices_gpu_address,
                    positions_buffer: positions_gpu_address,
                    normals_buffer: normals_gpu_address,
                    tx_coords_buffer: tx_coords_gpu_address,
                },
            );
            indices = rolling_copy(&mesh.indices, indices);
            indices_gpu_address += std::mem::size_of_val(&mesh.indices[..]) as MetalGPUAddress;
            normals = rolling_copy(&mesh.normals, normals);
            normals_gpu_address += std::mem::size_of_val(&mesh.normals[..]) as MetalGPUAddress;
            tx_coords = rolling_copy(&mesh.texcoords, tx_coords);
            tx_coords_gpu_address += std::mem::size_of_val(&mesh.texcoords[..]) as MetalGPUAddress;
            positions = rolling_copy(&mesh.positions, positions);
            positions_gpu_address += std::mem::size_of_val(&mesh.positions[..]) as MetalGPUAddress;
        }
        GeometryBuffers {
            arguments: arguments_buffer,
            _indices: indices_buf,
            _positions: positions_buf,
            _normals: normals_buf,
            _tx_coords: tx_coords_buf,
        }
    }
}
