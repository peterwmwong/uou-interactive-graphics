use super::heap_resident::HeapResident;
use crate::{
    align_size, allocate_new_buffer_with_heap, byte_size_of_slice, copy_into_buffer, metal::*,
    DEFAULT_RESOURCE_OPTIONS,
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

    fn allocate_and_encode(
        &mut self,
        heap: &Heap,
        device: &Device,
        geometry_arg_encoder: &ArgumentEncoder,
    ) -> (Buffer, u32, GeometryBuffers) {
        let arg_encoded_length = geometry_arg_encoder.encoded_length() as u32;
        let length = arg_encoded_length * self.objects.len() as u32;
        let arg_buffer = device.new_buffer(
            length as _,
            MTLResourceOptions::CPUCacheModeWriteCombined | MTLResourceOptions::StorageModeShared,
        );
        arg_buffer.set_label("Geometry Argument Buffer");

        // Allocate buffers...
        let mut indices_offset = 0;
        let (indices_ptr, indices_buf) =
            allocate_new_buffer_with_heap::<u32>(heap, "indices", self.indices_buf_length as _);
        let mut positions_offset = 0;
        let (positions_ptr, positions_buf) =
            allocate_new_buffer_with_heap::<f32>(heap, "positions", self.positions_buf_length as _);
        let mut normals_offset = 0;
        let (normals_ptr, normals_buf) =
            allocate_new_buffer_with_heap::<f32>(heap, "normals", self.normals_buf_length as _);
        let mut tx_coords_offset = 0;
        let (tx_coords_ptr, tx_coords_buf) =
            allocate_new_buffer_with_heap::<f32>(heap, "tx_coords", self.tx_coords_buf_length as _);

        for (i, tobj::Model { mesh, .. }) in self.objects.into_iter().enumerate() {
            geometry_arg_encoder.set_argument_buffer_to_element(i as _, &arg_buffer, 0);
            if GEOMETRY_ID_INDICES_BUFFER + 1 == GEOMETRY_ID_POSITIONS_BUFFER
                && GEOMETRY_ID_POSITIONS_BUFFER + 1 == GEOMETRY_ID_NORMALS_BUFFER
                && GEOMETRY_ID_NORMALS_BUFFER + 1 == GEOMETRY_ID_TX_COORDS_BUFFER
            {
                geometry_arg_encoder.set_buffers(
                    0,
                    &[&indices_buf, &positions_buf, &normals_buf, &tx_coords_buf],
                    &[
                        indices_offset as _,
                        positions_offset as _,
                        normals_offset as _,
                        tx_coords_offset as _,
                    ],
                );
            } else {
                geometry_arg_encoder.set_buffer(
                    GEOMETRY_ID_INDICES_BUFFER as _,
                    &indices_buf,
                    indices_offset as _,
                );
                geometry_arg_encoder.set_buffer(
                    GEOMETRY_ID_POSITIONS_BUFFER as _,
                    &positions_buf,
                    positions_offset as _,
                );
                geometry_arg_encoder.set_buffer(
                    GEOMETRY_ID_NORMALS_BUFFER as _,
                    &normals_buf,
                    normals_offset as _,
                );
                geometry_arg_encoder.set_buffer(
                    GEOMETRY_ID_TX_COORDS_BUFFER as _,
                    &tx_coords_buf,
                    tx_coords_offset as _,
                );
            }

            indices_offset = copy_into_buffer(&mesh.indices, indices_ptr, indices_offset);
            normals_offset = copy_into_buffer(&mesh.normals, normals_ptr, normals_offset);
            tx_coords_offset = copy_into_buffer(&mesh.texcoords, tx_coords_ptr, tx_coords_offset);

            let positions = &mesh.positions;
            positions_offset = copy_into_buffer(&positions, positions_ptr, positions_offset);
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
