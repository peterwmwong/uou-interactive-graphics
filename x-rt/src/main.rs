#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    geometry::*,
    launch_application,
    metal::{MTLLoadAction::*, MTLStoreAction::*, *},
    pipeline::{BlendMode, NoDepth, NoDepthState, NoStencil, RenderPipeline},
    RendererDelgate, UserEvent, DEFAULT_COLOR_FORMAT,
};
use shader_bindings::*;
use std::path::{Path, PathBuf};
use tobj::GPU_LOAD_OPTIONS;

const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

struct Draw {
    vertex_offset: u32,
    index_offset: u32,
    triangle_count: u32,
}

pub struct ModelAccelerationStructure {
    heap: Heap,
    instance_acceleration_structure: AccelerationStructure,
}

impl ModelAccelerationStructure {
    fn from_file<P: AsRef<Path>>(
        obj_file: P,
        device: &DeviceRef,
        cmd_queue: &CommandQueueRef,
    ) -> Self {
        let obj_file = obj_file.as_ref();
        let (models, ..) =
            tobj::load_obj(obj_file, &GPU_LOAD_OPTIONS).expect("Failed to load OBJ file");

        let mut draws: Vec<Draw> = vec![];
        let mut geometry: Geometry<u8, u8> =
            Geometry::new(&models, device, |_name, vertex_count, _material_id| {
                assert_eq!(vertex_count % 3, 0);
                draws.push(Draw {
                    triangle_count: (vertex_count / 3) as _,
                    vertex_offset: 0,
                    index_offset: 0,
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

        let mut i = 0;
        let geometry_buffers = geometry.allocate_and_encode(
            &geometry_heap,
            |_,
             GeometryToEncode {
                 indices_buffer_offset,
                 positions_buffer_offset,
                 ..
             }| {
                draws[i].index_offset = indices_buffer_offset;
                draws[i].vertex_offset = positions_buffer_offset;
                i += 1;
            },
        );

        // ======================================
        // Build Primitive Acceleration Structure
        // ======================================
        let as_tris: Vec<AccelerationStructureTriangleGeometryDescriptor> = draws
            .into_iter()
            .map(|draw| {
                let as_geo_tri = AccelerationStructureTriangleGeometryDescriptor::descriptor();
                as_geo_tri.set_vertex_format(MTLAttributeFormat::Float3);
                as_geo_tri.set_vertex_buffer(Some(&geometry_buffers.positions.buffer));
                as_geo_tri.set_vertex_buffer_offset(0);
                as_geo_tri.set_vertex_stride((std::mem::size_of::<f32>() * 3) as _);
                as_geo_tri.set_index_buffer(Some(&geometry_buffers.indices.buffer));
                as_geo_tri.set_index_buffer_offset(0);
                as_geo_tri.set_index_type(MTLIndexType::UInt32);
                as_geo_tri.set_triangle_count(1);
                as_geo_tri
            })
            .collect();

        let as_tri_refs: Vec<&AccelerationStructureGeometryDescriptorRef> = as_tris
            .iter()
            .map(|a| a as &AccelerationStructureGeometryDescriptorRef)
            .collect();
        let as_primitive_desc = PrimitiveAccelerationStructureDescriptor::descriptor();
        as_primitive_desc.set_geometry_descriptors(Array::from_slice(&as_tri_refs[..]));
        let MTLSizeAndAlign { size, align } =
            device.heap_acceleration_structure_size_and_align(&as_primitive_desc);
        let mut as_sizes = device.acceleration_structure_sizes_with_descriptor(&as_primitive_desc);
        as_sizes.acceleration_structure_size = size + align;

        let heap_with_as_primitive = {
            let desc = HeapDescriptor::new();
            desc.set_storage_mode(MTLStorageMode::Private);
            desc.set_size(as_sizes.acceleration_structure_size);
            device.new_heap(&desc)
        };
        let as_primitive = heap_with_as_primitive
            .new_acceleration_structure(size)
            .expect("Failed to allocate acceleration structure");

        let scratch_buffer = device.new_buffer(
            as_sizes.build_scratch_buffer_size,
            MTLResourceOptions::StorageModePrivate,
        );
        let cmd_buf = cmd_queue.new_command_buffer();
        let encoder = cmd_buf.new_acceleration_structure_command_encoder();
        encoder.build_acceleration_structure(&as_primitive, &as_primitive_desc, &scratch_buffer, 0);
        encoder.end_encoding();
        cmd_buf.commit();
        cmd_buf.wait_until_completed();

        // =====================================
        // Build Instance Acceleration Structure
        // =====================================
        let as_instance_desc = InstanceAccelerationStructureDescriptor::descriptor();
        as_instance_desc.set_instanced_acceleration_structures(&Array::from_slice(&[
            &as_primitive as &AccelerationStructureRef,
        ]));
        as_instance_desc.set_instance_count(1);

        let as_instance_descriptor_buffer = device.new_buffer_with_data(
            (&AccelerationStructureInstanceDescriptor {
                // Identity Matrix (column major 4x3)
                transformation_matrix: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [0., 0., 0.]],
                options: AccelerationStructureInstanceOptions::None,
                mask: 0xFF,
                intersection_function_table_offset: 0,
                acceleration_structure_index: 0,
            } as *const AccelerationStructureInstanceDescriptor) as *const _,
            std::mem::size_of::<AccelerationStructureInstanceDescriptor>() as _,
            MTLResourceOptions::StorageModeShared | MTLResourceOptions::HazardTrackingModeUntracked,
        );
        as_instance_desc.set_instance_descriptor_buffer(Some(&as_instance_descriptor_buffer));
        let cmd_buf = cmd_queue.new_command_buffer();
        let as_sizes = device.acceleration_structure_sizes_with_descriptor(&as_instance_desc);
        let scratch_buffer = device.new_buffer(
            as_sizes.build_scratch_buffer_size,
            MTLResourceOptions::StorageModePrivate
                | MTLResourceOptions::HazardTrackingModeUntracked,
        );
        let as_instance = device
            .new_acceleration_structure(as_sizes.acceleration_structure_size)
            .expect("Failed to allocate instance acceleration structure");

        let encoder = cmd_buf.new_acceleration_structure_command_encoder();
        encoder.build_acceleration_structure(&as_instance, &as_instance_desc, &scratch_buffer, 0);
        encoder.end_encoding();
        cmd_buf.commit();
        cmd_buf.wait_until_completed();

        Self {
            instance_acceleration_structure: as_instance,
            heap: heap_with_as_primitive,
        }
    }
}

struct Delegate {
    command_queue: CommandQueue,
    device: Device,
    model_as: ModelAccelerationStructure,
    needs_render: bool,
    pipeline: RenderPipeline<1, main_vertex, main_fragment, (NoDepth, NoStencil)>,
}

impl RendererDelgate for Delegate {
    fn new(device: Device) -> Self {
        let executable_name = std::env::args()
            .nth(0)
            .expect("Failed to access command line executable name");
        let model_file_path = std::env::args().nth(1).expect(&format!(
            "Usage: {executable_name} [Path to Wavefront OBJ file]"
        ));
        let model_file = PathBuf::from(model_file_path);
        let command_queue = device.new_command_queue();
        Self {
            model_as: ModelAccelerationStructure::from_file(model_file, &device, &command_queue),
            command_queue,
            needs_render: false,
            pipeline: RenderPipeline::new(
                "Pipeline",
                &device,
                &device.new_library_with_data(LIBRARY_BYTES).unwrap(),
                [(DEFAULT_COLOR_FORMAT, BlendMode::NoBlend)],
                main_vertex,
                main_fragment,
                (NoDepth, NoStencil),
            ),
            device,
        }
    }

    fn render(&mut self, render_target: &TextureRef) -> &CommandBufferRef {
        self.needs_render = false;
        let command_buffer = self
            .command_queue
            .new_command_buffer_with_unretained_references();
        self.pipeline.new_pass(
            "Render",
            command_buffer,
            [(render_target, (0., 1., 0., 1.), Clear, Store)],
            NoDepth,
            NoStencil,
            NoDepthState,
            &[],
            |p| {},
        );
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        match event {
            UserEvent::WindowFocusedOrResized { size: _ } => {
                self.needs_render = true;
            }
            _ => {}
        }
    }

    #[inline]
    fn needs_render(&self) -> bool {
        true
    }

    #[inline]
    fn device(&self) -> &Device {
        &self.device
    }
}

fn main() {
    launch_application::<Delegate>("x-rt");
}
