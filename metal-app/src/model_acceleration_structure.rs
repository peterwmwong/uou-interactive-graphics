use crate::{
    debug_time,
    geometry::*,
    metal::*,
    metal_types::*,
    pipeline::{BindAccelerationStructure, HeapUsage, ResourceUsage},
    typed_buffer::TypedBuffer,
};
use std::path::Path;

#[allow(dead_code)]
pub enum AccelerationStructureUpdateStrategy {
    // TODO: Try again when Beta 6 is released
    // - Currently does not work on Beta 5
    // Refit,
    Rebuild,
}
use AccelerationStructureUpdateStrategy::*;

const ACCELERATION_STRUCTURE_UPDATE_STRATEGY: AccelerationStructureUpdateStrategy = Rebuild;

struct Draw {
    name: String,
    vertex_byte_offset: u32,
    index_byte_offset: u32,
    triangle_count: u32,
}
pub struct ModelAccelerationStructure {
    geometry_heap: Heap,
    model_to_world_transform_buffer: TypedBuffer<MTLPackedFloat4x3>,
    prim_as_desc: PrimitiveAccelerationStructureDescriptor,
    prim_as_heap: Heap,
    prim_as_rebuild_buffer: Buffer,
    prim_as: AccelerationStructure,
}

impl ModelAccelerationStructure {
    pub fn from_file<P: AsRef<Path>>(
        obj_file: P,
        device: &DeviceRef,
        cmd_queue: &CommandQueueRef,
        init_m_model_to_world: impl FnOnce(&MaxBounds) -> f32x4x4,
    ) -> Self {
        let obj_file = obj_file.as_ref();
        let (models, ..) =
            tobj::load_obj(obj_file, &tobj::GPU_LOAD_OPTIONS).expect("Failed to load OBJ file");

        let mut draws: Vec<Draw> = vec![];
        let mut geometry: Geometry<u8, u8> =
            Geometry::new(&models, device, |name, vertex_count, _material_id| {
                assert_eq!(vertex_count % 3, 0);
                draws.push(Draw {
                    name,
                    triangle_count: (vertex_count / 3) as _,
                    vertex_byte_offset: 0,
                    index_byte_offset: 0,
                });
                0
            });
        let geometry_heap = {
            let desc = HeapDescriptor::new();
            desc.set_size(geometry.heap_size() as _);
            desc.set_cpu_cache_mode(MTLCPUCacheMode::WriteCombined);
            desc.set_storage_mode(MTLStorageMode::Shared);
            device.new_heap(&desc)
        };
        geometry_heap.set_label("geometry_heap");

        let mut i = 0;
        let geometry_buffers = geometry.allocate_and_encode(
            &geometry_heap,
            |_,
             GeometryToEncode {
                 indices_buffer_offset,
                 positions_buffer_offset,
                 ..
             }| {
                draws[i].index_byte_offset = indices_buffer_offset;
                draws[i].vertex_byte_offset = positions_buffer_offset;
                i += 1;
            },
        );

        let m_model_to_world = init_m_model_to_world(&geometry.max_bounds);
        let model_to_world_transform_buffer = TypedBuffer::from_data(
            "Triangle Transform Matrix",
            device,
            &[m_model_to_world.into()],
            MTLResourceOptions::StorageModeShared,
        );

        // ========================================
        // Define Primitive Acceleration Structures
        // ========================================
        let tri_as_descs: Vec<AccelerationStructureTriangleGeometryDescriptor> = draws
            .into_iter()
            .map(|draw| {
                let tri_as_desc = AccelerationStructureTriangleGeometryDescriptor::descriptor();
                tri_as_desc.set_vertex_format(MTLAttributeFormat::Float3);
                tri_as_desc.set_vertex_buffer(Some(&geometry_buffers.positions.raw));
                tri_as_desc.set_vertex_buffer_offset(draw.vertex_byte_offset as _);
                tri_as_desc.set_vertex_stride(std::mem::size_of::<[f32; 3]>() as _);
                tri_as_desc.set_index_buffer(Some(&geometry_buffers.indices.raw));
                tri_as_desc.set_index_buffer_offset(draw.index_byte_offset as _);
                tri_as_desc.set_index_type(MTLIndexType::UInt32);
                tri_as_desc.set_triangle_count(draw.triangle_count as _);
                tri_as_desc.set_opaque(true);
                tri_as_desc.set_label(&draw.name);
                tri_as_desc
                    .set_transformation_matrix_buffer(Some(&model_to_world_transform_buffer.raw));
                tri_as_desc
            })
            .collect();

        let tri_as_desc_refs: Vec<&AccelerationStructureGeometryDescriptorRef> = tri_as_descs
            .iter()
            .map(|a| a as &AccelerationStructureGeometryDescriptorRef)
            .collect();
        let prim_as_desc = PrimitiveAccelerationStructureDescriptor::descriptor();
        prim_as_desc.set_geometry_descriptors(Array::from_slice(&tri_as_desc_refs[..]));
        let MTLSizeAndAlign { size, align } =
            device.heap_acceleration_structure_size_and_align(&prim_as_desc);
        let mut prim_as_sizes = device.acceleration_structure_sizes_with_descriptor(&prim_as_desc);
        prim_as_sizes.acceleration_structure_size = size + align;
        let prim_as_heap = {
            let desc = HeapDescriptor::new();
            desc.set_storage_mode(MTLStorageMode::Private);
            desc.set_size(prim_as_sizes.acceleration_structure_size);
            device.new_heap(&desc)
        };
        // TODO: Why can't we use the Metal API `MTLHeap::makeAccelerationStructure(descriptor:)`?
        let prim_accel_struct = prim_as_heap
            .new_acceleration_structure(size)
            .expect("Failed to allocate acceleration structure");
        let prim_as_rebuild_buffer = device.new_buffer(
            prim_as_sizes
                .build_scratch_buffer_size
                .max(prim_as_sizes.refit_scratch_buffer_size),
            MTLResourceOptions::StorageModePrivate,
        );
        prim_as_rebuild_buffer.set_label("prim_as_rebuild_buffer");

        // ============================
        // Build Acceleration Structure
        // ============================
        {
            let cmd_buf = cmd_queue.new_command_buffer_with_unretained_references();
            let encoder = cmd_buf.new_acceleration_structure_command_encoder();
            encoder.use_heap(&prim_as_heap);
            encoder.use_heap(&geometry_heap);
            encoder.build_acceleration_structure(
                &prim_accel_struct,
                &prim_as_desc,
                &prim_as_rebuild_buffer,
                0,
            );
            encoder.end_encoding();
            cmd_buf.commit();
            cmd_buf.wait_until_completed();
            assert_eq!(cmd_buf.status(), MTLCommandBufferStatus::Completed);
        }

        Self {
            geometry_heap,
            model_to_world_transform_buffer,
            prim_as_desc,
            prim_as_heap,
            prim_as: prim_accel_struct,
            prim_as_rebuild_buffer,
        }
    }

    pub fn update_model_to_world_matrix(
        &mut self,
        transform_matrix_to_mul: f32x4x4,
        cmd_queue: &CommandQueueRef,
    ) {
        let m: f32x4x4 = self.model_to_world_transform_buffer.get_mut()[0].into();
        self.set_model_to_world_matrix(transform_matrix_to_mul * m, cmd_queue);
    }

    pub fn set_model_to_world_matrix(
        &mut self,
        model_to_world_matrix: f32x4x4,
        cmd_queue: &CommandQueueRef,
    ) {
        self.model_to_world_transform_buffer.get_mut()[0] = model_to_world_matrix.into();
        self.update(cmd_queue);
    }

    fn update(&mut self, cmd_queue: &CommandQueueRef) {
        let cmd_buf = cmd_queue.new_command_buffer();
        let e = cmd_buf.new_acceleration_structure_command_encoder();
        e.use_heap(&self.prim_as_heap);
        e.use_heap(&self.geometry_heap);
        match ACCELERATION_STRUCTURE_UPDATE_STRATEGY {
            // Refit => e.refit(
            //     &self.prim_as,
            //     &self.prim_as_desc,
            //     None,
            //     &self.prim_as_rebuild_buffer,
            //     0,
            // ),
            Rebuild => e.build_acceleration_structure(
                &self.prim_as,
                &self.prim_as_desc,
                &self.prim_as_rebuild_buffer,
                0,
            ),
        }
        e.end_encoding();
        debug_time("Update Acceleration Structure", || {
            cmd_buf.commit();
            cmd_buf.wait_until_completed();
        });
        assert_eq!(cmd_buf.status(), MTLCommandBufferStatus::Completed);
    }

    #[inline]
    pub fn resource<'a>(&'a self) -> impl ResourceUsage + 'a {
        HeapUsage(&self.prim_as_heap, MTLRenderStages::Fragment)
    }

    #[inline]
    pub fn bind<'a>(&'a self) -> BindAccelerationStructure<'a> {
        BindAccelerationStructure(&self.prim_as)
    }
}