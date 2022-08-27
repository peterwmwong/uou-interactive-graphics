#include <metal_stdlib>
#include "../../metal-types/src/projected-space.h"
#include "../../metal-types/src/tri_normals.h"

using namespace metal;
using namespace raytracing;

struct VertexOut
{
    float4 position [[position]];
    half3  raydir;
};

[[vertex]]
VertexOut main_vertex(
             uint       vertex_id             [[vertex_id]],
    constant float4x4 & m_projection_to_world [[buffer(0)]]
) {
    VertexOut out;
    switch (vertex_id) {
        case 0: out.position = float4(-3,  1, 1, 1); break;
        case 1: out.position = float4( 1,  1, 1, 1); break;
        case 2: out.position = float4( 1, -3, 1, 1); break;
    }
    out.raydir = half3((m_projection_to_world * out.position).xyz);
    return out;
}

[[fragment]]
half4 main_fragment(
             VertexOut                         in                    [[stage_in]],
             primitive_acceleration_structure  accelerationStructure [[buffer(0)]],
    constant ProjectedSpace                  & camera                [[buffer(1)]],
    constant half3x3                         * m_normal_to_worlds    [[buffer(2)]]
) {
    const ray r(camera.position_world.xyz, float3(normalize(in.raydir)));

    intersector<triangle_data> inter;
    inter.set_triangle_cull_mode(triangle_cull_mode::back);
    inter.assume_geometry_type(geometry_type::triangle);
    auto hit = inter.intersect(r, accelerationStructure);
    if (hit.type == intersection_type::triangle) {
        const auto p = (device TriNormals *) hit.primitive_data;
        return half4(p->normal(hit.triangle_barycentric_coord, &m_normal_to_worlds[hit.geometry_id]), 1);
    }
    return 0;
}