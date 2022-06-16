#include <metal_stdlib>
#include "./common.h"

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    float2 tx_coord;
};

[[vertex]]
VertexOut main_vertex(         uint       vertex_id                  [[vertex_id]],
            constant float4x4 & matrix_model_to_projection [[buffer(VertexBufferIndex::MatrixModelToProjection)]])
{
    constexpr const float2 plane_triange_strip_vertices[4] = {
        {-1.h, -1.h}, // Bottom Left
        {-1.h,  1.h}, // Top    Left
        { 1.h, -1.h}, // Bottom Right
        { 1.h,  1.h}, // Top    Right
    };
    const float2 position2d = plane_triange_strip_vertices[vertex_id];
    const float4 position   = float4(position2d, 0, 1);
    const float2 tx_coord   = fma(position2d, 0.5, 0.5);
    return {
        .position  = matrix_model_to_projection * position,
        .tx_coord  = float2(tx_coord.x, 1. - tx_coord.y)
    };
}

[[fragment]]
half4 main_fragment(VertexOut       in      [[stage_in]],
              texture2d<half> texture [[texture(FragBufferIndex::Texture)]])
{
    constexpr sampler tx_sampler(mag_filter::linear, address::repeat, min_filter::linear);
    const half4 color   = texture.sample(tx_sampler, in.tx_coord);
    const half4 ambient = 0.1;
    return color + ambient;
};
