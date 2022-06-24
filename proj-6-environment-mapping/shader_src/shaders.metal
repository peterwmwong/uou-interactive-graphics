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
            constant Geometry & geometry  [[buffer(VertexBufferIndex::Geometry)]],
            constant World    & world     [[buffer(VertexBufferIndex::World)]])
{
    const uint   idx      = geometry.indices[vertex_id];
    const float4 position = float4(geometry.positions[idx], 1.0);
    const float3 normal   = geometry.normals[idx];
    return {
        .position  = world.matrix_model_to_projection * position,
        .normal    = world.matrix_normal_to_world * normal
    };
}

fragment half4
main_fragment(         VertexOut   in       [[stage_in]],
              constant World     & world    [[buffer(FragBufferIndex::World)]])
{
    // Calculate the fragment's World Space position from a Metal Viewport Coordinate.
    // const float4 pos_w = world.matrix_screen_to_world * float4(in.position.xyz, 1);
    // const half3  pos   = half3(pos_w.xyz / pos_w.w);
    return half4(0, 1, 0, 1);
};
