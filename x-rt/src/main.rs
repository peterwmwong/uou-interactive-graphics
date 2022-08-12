#![feature(generic_associated_types)]
#![feature(portable_simd)]
mod shader_bindings;

use metal_app::{
    components::Camera,
    geometry::*,
    launch_application,
    metal::{MTLLoadAction::*, MTLStoreAction::*, *},
    metal_types::*,
    pipeline::*,
    ModifierKeys, RendererDelgate, UserEvent, DEFAULT_COLOR_FORMAT,
};
use shader_bindings::*;
use std::{
    f32::consts::PI,
    ops::Neg,
    path::{Path, PathBuf},
    simd::{f32x2, SimdFloat},
};

const INITIAL_CAMERA_DISTANCE: f32 = 1.;
const INITIAL_CAMERA_ROTATION: f32x2 = f32x2::from_array([0., 0.]);
const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

struct Draw {
    name: String,
    vertex_byte_offset: u32,
    index_byte_offset: u32,
    triangle_count: u32,
}

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// - Refactor/Extract into metal-app
pub struct ModelAccelerationStructure {
    prim_accel_struct_heap: Heap,
    inst_accel_struct: AccelerationStructure,
}

impl ModelAccelerationStructure {
    fn from_file<P: AsRef<Path>>(
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

        let m_model_to_world = {
            let MaxBounds { center, size } = geometry.max_bounds;
            let [cx, cy, cz, _] = center.neg().to_array();
            let scale = 1. / size.reduce_max();
            let transform = f32x4x4::scale(scale, scale, scale, 1.)
                * f32x4x4::x_rotate(PI / 2.)
                * f32x4x4::translate(cx, cy, cz);
            MTLPackedFloat4x3 {
                columns: [
                    MTLPackedFloat3(
                        transform.columns[0][0],
                        transform.columns[0][1],
                        transform.columns[0][2],
                    ),
                    MTLPackedFloat3(
                        transform.columns[1][0],
                        transform.columns[1][1],
                        transform.columns[1][2],
                    ),
                    MTLPackedFloat3(
                        transform.columns[2][0],
                        transform.columns[2][1],
                        transform.columns[2][2],
                    ),
                    MTLPackedFloat3(
                        transform.columns[3][0],
                        transform.columns[3][1],
                        transform.columns[3][2],
                    ),
                ],
            }
        };

        // ========================================
        // Define Primitive Acceleration Structures
        // ========================================
        let tri_accel_struct_descs: Vec<AccelerationStructureTriangleGeometryDescriptor> = draws
            .into_iter()
            .map(|draw| {
                let as_geo_tri = AccelerationStructureTriangleGeometryDescriptor::descriptor();
                as_geo_tri.set_vertex_format(MTLAttributeFormat::Float3);
                as_geo_tri.set_vertex_buffer(Some(&geometry_buffers.positions.buffer));
                as_geo_tri.set_vertex_buffer_offset(draw.vertex_byte_offset as _);
                as_geo_tri.set_vertex_stride((std::mem::size_of::<f32>() * 3) as _);
                as_geo_tri.set_index_buffer(Some(&geometry_buffers.indices.buffer));
                as_geo_tri.set_index_buffer_offset(draw.index_byte_offset as _);
                as_geo_tri.set_index_type(MTLIndexType::UInt32);
                as_geo_tri.set_triangle_count(draw.triangle_count as _);
                as_geo_tri.set_opaque(true);
                as_geo_tri.set_label(&draw.name);
                as_geo_tri
            })
            .collect();

        let tri_accel_struct_desc_refs: Vec<&AccelerationStructureGeometryDescriptorRef> =
            tri_accel_struct_descs
                .iter()
                .map(|a| a as &AccelerationStructureGeometryDescriptorRef)
                .collect();
        let prim_accel_struct_desc = PrimitiveAccelerationStructureDescriptor::descriptor();
        prim_accel_struct_desc
            .set_geometry_descriptors(Array::from_slice(&tri_accel_struct_desc_refs[..]));
        let MTLSizeAndAlign { size, align } =
            device.heap_acceleration_structure_size_and_align(&prim_accel_struct_desc);
        let mut as_sizes =
            device.acceleration_structure_sizes_with_descriptor(&prim_accel_struct_desc);
        as_sizes.acceleration_structure_size = size + align;
        let prim_accel_struct_heap = {
            let desc = HeapDescriptor::new();
            desc.set_storage_mode(MTLStorageMode::Private);
            desc.set_size(as_sizes.acceleration_structure_size);
            device.new_heap(&desc)
        };
        let prim_accel_struct = prim_accel_struct_heap
            .new_acceleration_structure(size)
            .expect("Failed to allocate acceleration structure");
        let prim_scratch_buffer = device.new_buffer(
            as_sizes.build_scratch_buffer_size,
            MTLResourceOptions::StorageModePrivate
                | MTLResourceOptions::HazardTrackingModeUntracked,
        );
        prim_scratch_buffer.set_label("prim_scratch_buffer");

        // ======================================
        // Define Instance Acceleration Structure
        // ======================================
        let inst_accel_struct_desc = InstanceAccelerationStructureDescriptor::descriptor();
        inst_accel_struct_desc.set_instanced_acceleration_structures(&Array::from_slice(&[
            &prim_accel_struct as &AccelerationStructureRef,
        ]));
        inst_accel_struct_desc.set_instance_count(1);

        let as_instance_descriptor_buffer = device.new_buffer_with_data(
            (&MTLAccelerationStructureInstanceDescriptor {
                // Identity Matrix (column major 4x3)
                transformation_matrix: m_model_to_world,
                options: MTLAccelerationStructureInstanceOptions::Opaque,
                // options: MTLAccelerationStructureInstanceOptions::None,
                mask: 0xFF,
                intersection_function_table_offset: 0,
                acceleration_structure_index: 0,
            } as *const MTLAccelerationStructureInstanceDescriptor) as *const _,
            std::mem::size_of::<MTLAccelerationStructureInstanceDescriptor>() as _,
            MTLResourceOptions::StorageModeShared | MTLResourceOptions::HazardTrackingModeUntracked,
        );
        as_instance_descriptor_buffer.set_label("as_instance_descriptor_buffer");
        inst_accel_struct_desc.set_instance_descriptor_buffer(Some(&as_instance_descriptor_buffer));
        let cmd_buf = cmd_queue.new_command_buffer();
        let as_sizes = device.acceleration_structure_sizes_with_descriptor(&inst_accel_struct_desc);
        let inst_accel_struct = device
            .new_acceleration_structure(as_sizes.acceleration_structure_size)
            .expect("Failed to allocate instance acceleration structure");
        let inst_scratch_buffer = device.new_buffer(
            as_sizes.build_scratch_buffer_size,
            MTLResourceOptions::StorageModePrivate
                | MTLResourceOptions::HazardTrackingModeUntracked,
        );
        inst_scratch_buffer.set_label("inst_scratch_buffer");

        // ======================================
        // Initiate build Acceleration Structures
        // ======================================
        {
            let encoder = cmd_buf.new_acceleration_structure_command_encoder();
            encoder.build_acceleration_structure(
                &prim_accel_struct,
                &prim_accel_struct_desc,
                &prim_scratch_buffer,
                0,
            );
            encoder.build_acceleration_structure(
                &inst_accel_struct,
                &inst_accel_struct_desc,
                &inst_scratch_buffer,
                0,
            );
            encoder.end_encoding();
            cmd_buf.commit();
            cmd_buf.wait_until_completed();
        }

        Self {
            prim_accel_struct_heap,
            inst_accel_struct,
        }
    }
}

struct Delegate {
    camera: Camera,
    camera_space: ProjectedSpace,
    camera_position: float4,
    command_queue: CommandQueue,
    device: Device,
    model_accel_struct: ModelAccelerationStructure,
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
            camera: Camera::new(
                INITIAL_CAMERA_DISTANCE,
                INITIAL_CAMERA_ROTATION,
                ModifierKeys::empty(),
                false,
                0.,
            ),
            camera_space: ProjectedSpace::default(),
            camera_position: float4 { xyzw: [0.; 4] },
            model_accel_struct: ModelAccelerationStructure::from_file(
                model_file,
                &device,
                &command_queue,
            ),
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
            &[&HeapUsage(
                &self.model_accel_struct.prim_accel_struct_heap,
                MTLRenderStages::Fragment,
            )],
            |p| {
                p.draw_primitives_with_bind(
                    NoBinds,
                    main_fragment_binds {
                        accelerationStructure: BindAccelerationStructure::AccelerationStructure(
                            &self.model_accel_struct.inst_accel_struct,
                        ),
                        camera: Bind::Value(&self.camera_space),
                        camera_pos: Bind::Value(&self.camera_position),
                    },
                    MTLPrimitiveType::Triangle,
                    0,
                    3,
                )
            },
        );
        command_buffer
    }

    fn on_event(&mut self, event: UserEvent) {
        if let Some(u) = self.camera.on_event(event) {
            self.camera_position = u.position_world.into();
            self.camera_space = ProjectedSpace {
                m_world_to_projection: u.m_world_to_projection,
                m_screen_to_world: u.m_screen_to_world,
                position_world: self.camera_position,
            };
            self.needs_render = true;
        }
    }

    #[inline]
    fn needs_render(&self) -> bool {
        self.needs_render
    }

    #[inline]
    fn device(&self) -> &Device {
        &self.device
    }
}

fn main() {
    launch_application::<Delegate>("x-rt");
}
