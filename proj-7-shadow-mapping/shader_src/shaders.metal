#include <metal_stdlib>
#include "./common.h"

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
};

vertex VertexOut
main_vertex(         uint       vertex_id [[vertex_id]],
            constant World    & world     [[buffer(VertexBufferIndex::World)]],
            constant Geometry & geometry  [[buffer(VertexBufferIndex::Geometry)]])
{
    const uint   idx    = geometry.indices[vertex_id];
    const float4 pos    = world.matrix_model_to_projection * float4(geometry.positions[idx], 1.0);
    const float3 normal = world.matrix_normal_to_world * float3(geometry.normals[idx]);
    return { .position = pos, .normal = normal };
}

fragment half4
main_fragment(         VertexOut   in    [[stage_in]],
              constant World     & world [[buffer(FragBufferIndex::World)]])
{
    return half4(0,1,0,1);
};

struct PlaneVertexOut
{
    float4 position [[position]];
    float3 normal;
};

vertex PlaneVertexOut
plane_vertex(         uint      vertex_id [[vertex_id]],
                     uint       inst_id   [[instance_id]],
            constant World    & world     [[buffer(VertexBufferIndex::World)]])
{
    // Vertices of Plane laying flat on the ground, along the x/z axis.
    constexpr const float plane_size = 0.9;
    constexpr const float2 verts_xz[4] = {
        {-1, -1}, // Bottom Left
        {-1,  1}, // Top    Left
        { 1, -1}, // Bottom Rigt
        { 1,  1}, // Top    Right
    };
    const float2 v = verts_xz[vertex_id] * plane_size;
    return {
        .position = world.matrix_world_to_projection * float4(v[0], world.plane_y, v[1], 1.0),
        .normal   = float3(0, 1, 0),
    };
}

fragment half4
plane_fragment(        PlaneVertexOut   in    [[stage_in]],
              constant World          & world [[buffer(FragBufferIndex::World)]])
{
    return half4(0,1,0,1);
};
