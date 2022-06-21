use crate::{
    align_size, allocate_new_buffer_with_heap, byte_size_of_slice, copy_into_buffer,
    get_gpu_addresses, metal::*, MetalGPUAddress, DEFAULT_RESOURCE_OPTIONS,
};
use std::{marker::PhantomData, simd::f32x4};

pub struct MaxBounds {
    pub center: f32x4,
    pub size: f32x4,
}

pub(crate) struct DrawInfo {
    pub(crate) debug_group_name: String,
    pub(crate) num_indices: usize,
    pub(crate) material_id: usize,
}

pub trait GeometryArgumentEncoder<T: Sized> {
    fn set(
        arg: &mut T,
        indices_buffer: MetalGPUAddress,
        positions_buffer: MetalGPUAddress,
        normals_buffer: MetalGPUAddress,
        tx_coords_buffer: MetalGPUAddress,
    );
}

// Each buffer needs to be owned and not dropped (causing deallocation from the owning MTLHeap).
pub(crate) struct GeometryBuffers {
    pub(crate) arguments: Buffer,
    pub(crate) argument_byte_size: usize,
    #[allow(dead_code)]
    indices: Buffer,
    #[allow(dead_code)]
    positions: Buffer,
    #[allow(dead_code)]
    normals: Buffer,
    #[allow(dead_code)]
    tx_coords: Buffer,
}

pub(crate) struct Geometry<'a, T: Sized, E: GeometryArgumentEncoder<T>> {
    arguments_byte_size: usize,
    objects: &'a [tobj::Model],
    indices_buf_length: usize,
    positions_buf_length: usize,
    normals_buf_length: usize,
    tx_coords_buf_length: usize,
    heap_size: usize,
    pub(crate) max_bounds: MaxBounds,
    pub(crate) draws: Vec<DrawInfo>,
    _p: PhantomData<(T, E)>,
}

impl<'a, T: Sized, E: GeometryArgumentEncoder<T>> Geometry<'a, T, E> {
    pub(crate) fn new(objects: &'a [tobj::Model], device: &Device) -> Self {
        let mut heap_size = 0;

        // Create a shared buffer (shared between all objects) for each ObjectGeometry member.
        // Calculate the size of each buffer...
        let mut indices_buf_length = 0;
        let mut positions_buf_length = 0;
        let mut normals_buf_length = 0;
        let mut tx_coords_buf_length = 0;
        let mut mins = f32x4::splat(f32::MAX);
        let mut maxs = f32x4::splat(f32::MIN);
        let mut draws = Vec::<DrawInfo>::with_capacity(objects.len());
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
            indices_buf_length += byte_size_of_slice(&mesh.indices);
            positions_buf_length += byte_size_of_slice(&mesh.positions);
            normals_buf_length += byte_size_of_slice(&mesh.normals);
            tx_coords_buf_length += byte_size_of_slice(&mesh.texcoords);
            draws.push(DrawInfo {
                debug_group_name: name.to_owned(),
                num_indices: mesh.indices.len() as _,
                material_id: mesh.material_id.expect("No material found for object.") as _,
            });
            for &[x, y, z] in mesh.positions.as_chunks::<3>().0 {
                let input = f32x4::from_array([x, y, z, 0.0]);
                mins = mins.min(input);
                maxs = maxs.max(input);
            }
        }
        for buf_length in [
            indices_buf_length,
            positions_buf_length,
            normals_buf_length,
            tx_coords_buf_length,
        ] {
            /*
            This may seem like a mistake to use the aligned size (size + padding) for the last buffer (No
            subsequent buffer needs padding to be aligned), but this padding actually represents the padding
            needed for the **first** buffer (right after the last texture).
            */
            heap_size += align_size(
                device.heap_buffer_size_and_align(buf_length as _, DEFAULT_RESOURCE_OPTIONS),
            );
        }
        let arguments_byte_size = std::mem::size_of::<T>() * objects.len();
        heap_size += align_size(
            device.heap_buffer_size_and_align(arguments_byte_size as _, DEFAULT_RESOURCE_OPTIONS),
        );
        let size = maxs - mins;
        let center = mins + (size * f32x4::splat(0.5));
        Self {
            arguments_byte_size,
            heap_size,
            indices_buf_length,
            positions_buf_length,
            normals_buf_length,
            tx_coords_buf_length,
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

    pub fn allocate_and_encode(&mut self, heap: &Heap) -> GeometryBuffers {
        let (mut arguments_ptr, arguments) = allocate_new_buffer_with_heap::<T>(
            heap,
            "Geometry Arguments",
            self.arguments_byte_size as _,
        );

        // Allocate buffers...
        let mut indices_offset: usize = 0;
        let (indices_ptr, indices_buf) =
            allocate_new_buffer_with_heap::<u32>(heap, "indices", self.indices_buf_length as _);
        let mut positions_offset: usize = 0;
        let (positions_ptr, positions_buf) =
            allocate_new_buffer_with_heap::<f32>(heap, "positions", self.positions_buf_length as _);
        let mut normals_offset: usize = 0;
        let (normals_ptr, normals_buf) =
            allocate_new_buffer_with_heap::<f32>(heap, "normals", self.normals_buf_length as _);
        let mut tx_coords_offset: usize = 0;
        let (tx_coords_ptr, tx_coords_buf) =
            allocate_new_buffer_with_heap::<f32>(heap, "tx_coords", self.tx_coords_buf_length as _);
        let [indices_gpu_address, positions_gpu_address, normals_gpu_address, tx_coords_gpu_address] =
            get_gpu_addresses([&indices_buf, &positions_buf, &normals_buf, &tx_coords_buf]);

        for tobj::Model { mesh, .. } in self.objects.into_iter() {
            E::set(
                unsafe { &mut *arguments_ptr },
                indices_gpu_address + (indices_offset as MetalGPUAddress),
                positions_gpu_address + (positions_offset as MetalGPUAddress),
                normals_gpu_address + (normals_offset as MetalGPUAddress),
                tx_coords_gpu_address + (tx_coords_offset as MetalGPUAddress),
            );
            indices_offset = copy_into_buffer(&mesh.indices, indices_ptr, indices_offset);
            normals_offset = copy_into_buffer(&mesh.normals, normals_ptr, normals_offset);
            tx_coords_offset = copy_into_buffer(&mesh.texcoords, tx_coords_ptr, tx_coords_offset);
            positions_offset = copy_into_buffer(&mesh.positions, positions_ptr, positions_offset);
            unsafe { arguments_ptr = arguments_ptr.add(1) };
        }
        GeometryBuffers {
            arguments,
            argument_byte_size: std::mem::size_of::<T>() as _,
            indices: indices_buf,
            positions: positions_buf,
            normals: normals_buf,
            tx_coords: tx_coords_buf,
        }
    }
}
