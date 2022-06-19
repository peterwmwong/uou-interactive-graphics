use super::heap_resident::HeapResident;
use crate::{
    align_size, allocate_new_buffer_with_heap, byte_size_of_slice, copy_into_buffer,
    get_gpu_addresses, metal::*, MetalGPUAddress, DEFAULT_RESOURCE_OPTIONS,
};
use std::simd::f32x4;

pub struct MaxBounds {
    pub center: f32x4,
    pub size: f32x4,
}

pub(crate) struct DrawInfo {
    pub(crate) debug_group_name: String,
    pub(crate) num_indices: u32,
    pub(crate) material_id: u32,
}

const NUM_GEOMETRY_BUFFERS: usize = 4;
const METAL_GPU_ADDRESS_BYTE_SIZE: usize = std::mem::size_of::<MetalGPUAddress>();
const MIN_GEOMETRY_ARGUMENT_BYTE_LENGTH: usize = NUM_GEOMETRY_BUFFERS * METAL_GPU_ADDRESS_BYTE_SIZE;

// Each buffer needs to be owned and not dropped (causing deallocation from the owning MTLHeap).
pub(crate) struct GeometryBuffers {
    #[allow(dead_code)]
    indices: Buffer,
    #[allow(dead_code)]
    positions: Buffer,
    #[allow(dead_code)]
    normals: Buffer,
    #[allow(dead_code)]
    tx_coords: Buffer,
}

pub(crate) struct Geometry<
    'a,
    const GEOMETRY_ID_INDICES_BUFFER: u16,
    const GEOMETRY_ID_POSITIONS_BUFFER: u16,
    const GEOMETRY_ID_NORMALS_BUFFER: u16,
    const GEOMETRY_ID_TX_COORDS_BUFFER: u16,
> {
    objects: &'a [tobj::Model],
    indices_buf_length: usize,
    positions_buf_length: usize,
    normals_buf_length: usize,
    tx_coords_buf_length: usize,
    heap_size: usize,
    pub(crate) max_bounds: MaxBounds,
    pub(crate) draws: Vec<DrawInfo>,
}

impl<
        'a,
        const GEOMETRY_ID_INDICES_BUFFER: u16,
        const GEOMETRY_ID_POSITIONS_BUFFER: u16,
        const GEOMETRY_ID_NORMALS_BUFFER: u16,
        const GEOMETRY_ID_TX_COORDS_BUFFER: u16,
    >
    Geometry<
        'a,
        GEOMETRY_ID_INDICES_BUFFER,
        GEOMETRY_ID_POSITIONS_BUFFER,
        GEOMETRY_ID_NORMALS_BUFFER,
        GEOMETRY_ID_TX_COORDS_BUFFER,
    >
{
    pub(crate) fn new(objects: &'a [tobj::Model], device: &Device) -> Self {
        assert!(
            GEOMETRY_ID_INDICES_BUFFER != GEOMETRY_ID_POSITIONS_BUFFER
                && GEOMETRY_ID_INDICES_BUFFER != GEOMETRY_ID_NORMALS_BUFFER
                && GEOMETRY_ID_INDICES_BUFFER != GEOMETRY_ID_TX_COORDS_BUFFER
                && GEOMETRY_ID_POSITIONS_BUFFER != GEOMETRY_ID_NORMALS_BUFFER
                && GEOMETRY_ID_POSITIONS_BUFFER != GEOMETRY_ID_TX_COORDS_BUFFER
                && GEOMETRY_ID_NORMALS_BUFFER != GEOMETRY_ID_TX_COORDS_BUFFER,
            r#"Geometry ID constants (Metal Shader [[id(...)]] argument bindings) must all be unique.
Check the following generic constants passed to Model::from_file()...
- GEOMETRY_ID_INDICES_BUFFER
- GEOMETRY_ID_POSITIONS_BUFFER
- GEOMETRY_ID_NORMALS_BUFFER
- GEOMETRY_ID_TX_COORDS_BUFFER
"#
        );
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
            // indices_buf_length += std::mem::size_of_val(&mesh.indices);
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
        let size = maxs - mins;
        let center = mins + (size * f32x4::splat(0.5));
        Self {
            heap_size,
            indices_buf_length,
            positions_buf_length,
            normals_buf_length,
            tx_coords_buf_length,
            objects,
            draws,
            max_bounds: MaxBounds { center, size },
        }
    }
}

impl<
        'a,
        const GEOMETRY_ID_INDICES_BUFFER: u16,
        const GEOMETRY_ID_POSITIONS_BUFFER: u16,
        const GEOMETRY_ID_NORMALS_BUFFER: u16,
        const GEOMETRY_ID_TX_COORDS_BUFFER: u16,
    > HeapResident<GeometryBuffers>
    for Geometry<
        'a,
        GEOMETRY_ID_INDICES_BUFFER,
        GEOMETRY_ID_POSITIONS_BUFFER,
        GEOMETRY_ID_NORMALS_BUFFER,
        GEOMETRY_ID_TX_COORDS_BUFFER,
    >
{
    #[inline]
    fn heap_size(&self) -> usize {
        self.heap_size
    }

    // TODO: START HERE 2
    // TODO: START HERE 2
    // TODO: START HERE 2
    // How do we get arg_encoded_length (size of Geometry struct in common.h) without ArgumentEncoder?
    // - Because ArgumentEncoder is deprecated, surely there's replacement for just getting the byte length
    // - Checkout MTLBinding and MTLBufferBinding documentation.
    fn allocate_and_encode(
        &mut self,
        heap: &Heap,
        device: &Device,
        geometry_arg_encoder: &ArgumentEncoder,
    ) -> (Buffer, u32, GeometryBuffers) {
        let arg_encoded_length = geometry_arg_encoder.encoded_length() as u32;
        debug_assert_eq!(MIN_GEOMETRY_ARGUMENT_BYTE_LENGTH, arg_encoded_length as _);

        let length = arg_encoded_length * self.objects.len() as u32;
        // TODO: Allocate from Heap
        let arg_buffer = device.new_buffer(
            length as _,
            MTLResourceOptions::CPUCacheModeWriteCombined | MTLResourceOptions::StorageModeShared,
        );
        arg_buffer.set_label("Geometry Argument Buffer");
        let mut args = arg_buffer.contents() as *mut MetalGPUAddress;

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
            if GEOMETRY_ID_INDICES_BUFFER + 1 == GEOMETRY_ID_POSITIONS_BUFFER
                && GEOMETRY_ID_POSITIONS_BUFFER + 1 == GEOMETRY_ID_NORMALS_BUFFER
                && GEOMETRY_ID_NORMALS_BUFFER + 1 == GEOMETRY_ID_TX_COORDS_BUFFER
            {
                unsafe {
                    *(args.add(0)) = indices_gpu_address + (indices_offset as MetalGPUAddress);
                    *(args.add(1)) = positions_gpu_address + (positions_offset as MetalGPUAddress);
                    *(args.add(2)) = normals_gpu_address + (normals_offset as MetalGPUAddress);
                    *(args.add(3)) = tx_coords_gpu_address + (tx_coords_offset as MetalGPUAddress);
                    args = args.add(NUM_GEOMETRY_BUFFERS);
                };
            } else {
                unsafe {
                    for (id, gpu_address, offset) in [
                        (
                            GEOMETRY_ID_INDICES_BUFFER,
                            indices_gpu_address,
                            indices_offset,
                        ),
                        (
                            GEOMETRY_ID_POSITIONS_BUFFER,
                            positions_gpu_address,
                            positions_offset,
                        ),
                        (
                            GEOMETRY_ID_NORMALS_BUFFER,
                            normals_gpu_address,
                            normals_offset,
                        ),
                        (
                            GEOMETRY_ID_TX_COORDS_BUFFER,
                            tx_coords_gpu_address,
                            tx_coords_offset,
                        ),
                    ] {
                        *(args.add(id as _)) = gpu_address + (offset as MetalGPUAddress);
                    }
                    args = args.byte_add(arg_encoded_length as _);
                };
            }

            indices_offset = copy_into_buffer(&mesh.indices, indices_ptr, indices_offset);
            normals_offset = copy_into_buffer(&mesh.normals, normals_ptr, normals_offset);
            tx_coords_offset = copy_into_buffer(&mesh.texcoords, tx_coords_ptr, tx_coords_offset);
            positions_offset = copy_into_buffer(&mesh.positions, positions_ptr, positions_offset);
        }
        (
            arg_buffer,
            arg_encoded_length,
            GeometryBuffers {
                indices: indices_buf,
                positions: positions_buf,
                normals: normals_buf,
                tx_coords: tx_coords_buf,
            },
        )
    }
}
