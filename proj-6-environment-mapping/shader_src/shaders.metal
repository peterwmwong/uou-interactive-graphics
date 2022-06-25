#include <metal_stdlib>
#include "./common.h"

using namespace metal;

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// Compare with a sky box approach
// - With the current approach, haven't quite figured out how to nicely "zoom out"
// - It might help to try the sky box just to see what is "correct" and try to reverse engineer it.
struct BGVertexOut {
    float4 position [[position]];
};

vertex BGVertexOut
bg_vertex(uint vertex_id [[vertex_id]])
{
    constexpr const float2 plane_triange_strip_vertices[4] = {
        {-1.h, -1.h}, // Bottom Left
        {-1.h,  1.h}, // Top    Left
        { 1.h, -1.h}, // Bottom Right
        { 1.h,  1.h}, // Top    Right
    };
    const float2 position2d = plane_triange_strip_vertices[vertex_id];
    return { .position = float4(position2d, 0, 1) };
}


fragment half4
bg_fragment(         BGVertexOut         in       [[stage_in]],
            constant World             & world    [[buffer(BGFragBufferIndex::World)]],
                     texturecube<half>   texture  [[texture(BGFragTextureIndex::CubeMapTexture)]])
{
    constexpr sampler tx_sampler(mag_filter::linear, address::clamp_to_zero, min_filter::linear);
    const float4 pos   = world.matrix_screen_to_world * float4(in.position.xy, 1, 1);
    const half4  color = texture.sample(tx_sampler, pos.xyz);
    return color;
}

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
