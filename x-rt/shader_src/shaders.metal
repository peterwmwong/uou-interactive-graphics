#include <metal_stdlib>
#include "./shader_bindings.h"

using namespace metal;
using raytracing::instance_acceleration_structure;

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
             instance_acceleration_structure   accelerationStructure [[buffer(0)]],
    constant ProjectedSpace                  & camera                [[buffer(1)]],
    constant float4                          & camera_pos            [[buffer(2)]]
) {
    const float4 pos_w       = camera.m_screen_to_world * float4(in.position.xyz, 1);
    const float3 pos         = pos_w.xyz / pos_w.w;
    const float3 camera_pos_ = camera_pos.xyz;

    raytracing::ray r;
    r.origin       = float3(camera_pos_);
    r.direction    = normalize(pos - camera_pos_);
    r.min_distance = 0.001;
    r.max_distance = FLT_MAX;

    raytracing::intersector<raytracing::instancing> intersector;
    intersector.assume_geometry_type(raytracing::geometry_type::triangle);
    auto intersection = intersector.intersect(r, accelerationStructure);
    return (intersection.type == raytracing::intersection_type::triangle) ? half4(half3(intersection.distance), 1) : 0;
}