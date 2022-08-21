#include <metal_stdlib>
#include "../../metal-types/src/projected-space.h"
#include "../../metal-types/src/tri_normals_index.h"

using namespace metal;
using namespace raytracing;

struct VertexOut
{
    float4 position [[position]];
};

[[vertex]]
VertexOut main_vertex(
    uint vertex_id [[vertex_id]]
) {
    return  (vertex_id == 0) ? VertexOut { .position = float4(-3,  1, 0, 1) }
          : (vertex_id == 1) ? VertexOut { .position = float4( 1,  1, 0, 1) }
          : (vertex_id == 2) ? VertexOut { .position = float4( 1, -3, 0, 1) }
          : VertexOut { .position = float4(0) };
}

[[fragment]]
half4 main_fragment(
             VertexOut                         in                    [[stage_in]],
             primitive_acceleration_structure  accelerationStructure [[buffer(0)]],
    constant ProjectedSpace                  & camera                [[buffer(1)]],
    constant MTLPackedFloat4x3               * m_model_to_worlds     [[buffer(2)]]
) {
    const float4 pos_w = camera.m_screen_to_world * float4(in.position.xyz, 1);
    const float3 pos   = pos_w.xyz / pos_w.w;
    const ray    r(camera.position_world.xyz, normalize(pos - camera.position_world.xyz));

    intersector<triangle_data> inter;
    inter.set_triangle_cull_mode(triangle_cull_mode::back);
    inter.assume_geometry_type(geometry_type::triangle);
    auto hit = inter.intersect(r, accelerationStructure);
    if (hit.type == intersection_type::triangle) {
        const auto p = (device TriNormalsIndex *) hit.primitive_data;
        return half4(p->normal(hit.triangle_barycentric_coord, m_model_to_worlds), 1);
    }
    return 0;
}