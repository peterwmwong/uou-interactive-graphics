use crate::{
    geometry::*,
    metal::*,
    metal_types::*,
    pipeline::{BindAccelerationStructure, HeapUsage, ResourceUsage},
    typed_buffer::TypedBuffer,
};
use std::{
    f32::consts::PI,
    ops::{Deref, Neg},
    path::Path,
    simd::SimdFloat,
};

#[allow(dead_code)]
pub enum AccelerationStructureUpdateStrategy {
    Refit,
    Rebuild,
}
use AccelerationStructureUpdateStrategy::*;

const ACCELERATION_STRUCTURE_UPDATE_STRATEGY: AccelerationStructureUpdateStrategy = Rebuild;

#[inline]
fn into_mtlpacked_float4x3(m: f32x4x4) -> MTLPackedFloat4x3 {
    MTLPackedFloat4x3 {
        columns: [
            MTLPackedFloat3(m.columns[0][0], m.columns[0][1], m.columns[0][2]),
            MTLPackedFloat3(m.columns[1][0], m.columns[1][1], m.columns[1][2]),
            MTLPackedFloat3(m.columns[2][0], m.columns[2][1], m.columns[2][2]),
            MTLPackedFloat3(m.columns[3][0], m.columns[3][1], m.columns[3][2]),
        ],
    }
}

struct Draw {
    name: String,
    vertex_byte_offset: u32,
    index_byte_offset: u32,
    triangle_count: u32,
}

// TODO:
// - Consider splitting up creation
//   - primitive vs instance
// - Consider creating a single heap
#[allow(dead_code)]
pub struct ModelAccelerationStructure {
    inst_as_desc_buffer: TypedBuffer<MTLAccelerationStructureInstanceDescriptor>,
    inst_as_desc: InstanceAccelerationStructureDescriptor,
    inst_as_rebuild_buffer: Buffer,
    pub inst_as: AccelerationStructure,
    m_model_to_world: f32x4x4,
    prim_as_desc: PrimitiveAccelerationStructureDescriptor,
    pub prim_as_heap: Heap,
    prim_as_rebuild_buffer: Buffer,
    prim_as: AccelerationStructure,
}

impl ModelAccelerationStructure {
    pub fn from_file<P: AsRef<Path>>(
        obj_file: P,
        device: &DeviceRef,
        cmd_queue: &CommandQueueRef,
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

        let m_model_to_world: f32x4x4 = {
            let MaxBounds { center, size } = geometry.max_bounds;
            let [cx, cy, cz, _] = center.neg().to_array();
            let scale = 1. / size.reduce_max();
            f32x4x4::scale(scale, scale, scale, 1.)
                * f32x4x4::x_rotate(PI / 2.)
                * f32x4x4::translate(cx, cy, cz)
        };

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
                tri_as_desc.set_vertex_stride((std::mem::size_of::<f32>() * 3) as _);
                tri_as_desc.set_index_buffer(Some(&geometry_buffers.indices.raw));
                tri_as_desc.set_index_buffer_offset(draw.index_byte_offset as _);
                tri_as_desc.set_index_type(MTLIndexType::UInt32);
                tri_as_desc.set_triangle_count(draw.triangle_count as _);
                tri_as_desc.set_opaque(true);
                tri_as_desc.set_label(&draw.name);
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
            prim_as_sizes.build_scratch_buffer_size,
            MTLResourceOptions::StorageModePrivate,
        );
        prim_as_rebuild_buffer.set_label("prim_scratch_buffer");

        // ======================================
        // Define Instance Acceleration Structure
        // ======================================
        let inst_as_desc = InstanceAccelerationStructureDescriptor::descriptor();
        inst_as_desc.set_instanced_acceleration_structures(&Array::from_slice(&[
            &prim_accel_struct as &AccelerationStructureRef,
        ]));
        inst_as_desc.set_instance_count(1);

        let inst_as_desc_buffer = TypedBuffer::from_data(
            "Instance Acceleration Structure Descriptor",
            device.deref(),
            &[MTLAccelerationStructureInstanceDescriptor {
                // Identity Matrix (column major 4x3)
                transformation_matrix: into_mtlpacked_float4x3(m_model_to_world),
                options: MTLAccelerationStructureInstanceOptions::Opaque,
                mask: 0xFF,
                intersection_function_table_offset: 0,
                acceleration_structure_index: 0,
            }],
            MTLResourceOptions::StorageModeShared,
        );
        inst_as_desc.set_instance_descriptor_buffer(Some(&inst_as_desc_buffer.raw));
        let inst_as_sizes = device.acceleration_structure_sizes_with_descriptor(&inst_as_desc);
        let inst_as = device
            .new_acceleration_structure(inst_as_sizes.acceleration_structure_size)
            .expect("Failed to allocate instance acceleration structure");
        let inst_as_rebuild_buffer = device.new_buffer(
            inst_as_sizes
                .build_scratch_buffer_size
                .max(inst_as_sizes.refit_scratch_buffer_size),
            MTLResourceOptions::StorageModePrivate,
        );
        inst_as_rebuild_buffer.set_label("inst_accel_rebuild_buffer");

        // ======================================
        // Initiate build Acceleration Structures
        // ======================================
        {
            let cmd_buf = cmd_queue.new_command_buffer_with_unretained_references();
            let encoder = cmd_buf.new_acceleration_structure_command_encoder();
            encoder.use_heap(&prim_as_heap);
            encoder.use_resource(&inst_as_desc_buffer.raw, MTLResourceUsage::Read);
            encoder.build_acceleration_structure(
                &prim_accel_struct,
                &prim_as_desc,
                &prim_as_rebuild_buffer,
                0,
            );
            encoder.build_acceleration_structure(
                &inst_as,
                &inst_as_desc,
                &inst_as_rebuild_buffer,
                0,
            );
            encoder.end_encoding();
            cmd_buf.commit();
            cmd_buf.wait_until_completed();
            assert_eq!(cmd_buf.status(), MTLCommandBufferStatus::Completed);
        }

        Self {
            inst_as_desc_buffer,
            inst_as_desc,
            inst_as,
            inst_as_rebuild_buffer,
            m_model_to_world,
            prim_as_desc,
            prim_as_heap,
            prim_as: prim_accel_struct,
            prim_as_rebuild_buffer,
        }
    }

    pub fn update_model_to_world_matrix(
        &mut self,
        transform_matrix_to_mul: f32x4x4,
        device: &DeviceRef,
        cmd_queue: &CommandQueueRef,
    ) {
        self.set_model_to_world_matrix(
            transform_matrix_to_mul * self.m_model_to_world,
            device,
            cmd_queue,
        );
    }

    pub fn set_model_to_world_matrix(
        &mut self,
        model_to_world_matrix: f32x4x4,
        device: &DeviceRef,
        cmd_queue: &CommandQueueRef,
    ) {
        self.m_model_to_world = model_to_world_matrix;
        self.inst_as_desc_buffer.get_mut()[0].transformation_matrix =
            into_mtlpacked_float4x3(self.m_model_to_world);
        self.refit(device, cmd_queue);
    }

    fn refit(&mut self, device: &DeviceRef, cmd_queue: &CommandQueueRef) {
        if matches!(ACCELERATION_STRUCTURE_UPDATE_STRATEGY, Rebuild) {
            let as_sizes = device.acceleration_structure_sizes_with_descriptor(&self.inst_as_desc);
            self.inst_as = device
                .new_acceleration_structure(as_sizes.acceleration_structure_size)
                .expect("Failed to allocate instance acceleration structure");
        }

        let cmd_buf = cmd_queue.new_command_buffer();
        let e = cmd_buf.new_acceleration_structure_command_encoder();
        e.use_heap(&self.prim_as_heap);
        e.use_resource(&self.inst_as_desc_buffer.raw, MTLResourceUsage::Read);
        match ACCELERATION_STRUCTURE_UPDATE_STRATEGY {
            Refit => e.refit(
                &self.inst_as,
                &self.inst_as_desc,
                None,
                &self.inst_as_rebuild_buffer,
                0,
            ),
            Rebuild => e.build_acceleration_structure(
                &self.inst_as,
                &self.inst_as_desc,
                &self.inst_as_rebuild_buffer,
                0,
            ),
        }
        e.end_encoding();
        cmd_buf.commit();
        cmd_buf.wait_until_completed();
        assert_eq!(cmd_buf.status(), MTLCommandBufferStatus::Completed);
    }

    #[inline]
    pub fn resource<'a>(&'a self) -> impl ResourceUsage + 'a {
        HeapUsage(&self.prim_as_heap, MTLRenderStages::Fragment)
    }

    #[inline]
    pub fn bind<'a>(&'a self) -> BindAccelerationStructure<'a> {
        BindAccelerationStructure(&self.inst_as)
    }
}
