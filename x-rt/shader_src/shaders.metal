#include <metal_stdlib>
#include "./shader_bindings.h"

using namespace metal;
using raytracing::instance_acceleration_structure;

[[vertex]]
float4 main_vertex(
    uint vertex_id [[vertex_id]]
) {
    switch (vertex_id) {
        case 0: return { -1, -1, 0, 0 };
        case 1: return {  0,  1, 0, 0 };
        case 2: return {  1, -1, 0, 0 };
    }
    return 0;
}

[[fragment]]
half4 main_fragment(
    float4                          position              [[stage_in]],
    instance_acceleration_structure accelerationStructure [[buffer(0)]]
) {
    // raytracing::ray r;
    // r.origin       = float3(0);
    // r.direction    = normalize(float3(position.xy, 1));
    // r.min_distance = 0.1;
    // r.max_distance = FLT_MAX;

    // raytracing::intersector<raytracing::instancing, raytracing::triangle_data> intersector;
    // intersector.assume_geometry_type(raytracing::geometry_type::triangle);
    // auto intersection = intersector.intersect(r, accelerationStructure, 0xFF);
    // return (intersection.type == raytracing::intersection_type::triangle) ? 1 : 0;
    return 0;
}