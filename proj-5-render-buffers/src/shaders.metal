#include <metal_stdlib>
#include "./shader_bindings.h"

using namespace metal;

struct CheckerboardVertexOut
{
    float4 position [[position]];
};

[[vertex]]
CheckerboardVertexOut checkerboard_vertex(uint vertex_id [[vertex_id]])
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

[[fragment]]
half4 checkerboard_fragment(CheckerboardVertexOut in [[stage_in]])
{
    const float square_size = 8.0;
    const uint2 alt = uint2(in.position.xy / square_size) & 1;
    return half4(alt.x == alt.y);
};

struct VertexOut
{
    float4 position [[position]];
    float2 tx_coord;
};

[[vertex]]
VertexOut main_vertex(         uint       vertex_id                  [[vertex_id]],
                      constant float4x4 & m_model_to_projection [[buffer(0)]])
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
        .position  = m_model_to_projection * position,
        .tx_coord  = 1. - tx_coord
    };
}

[[fragment]]
half4 main_fragment(         VertexOut           in      [[stage_in]],
                             texture2d<half>     texture [[texture(0)]],
                    constant TextureFilterMode & mode    [[buffer(0)]])
{
    const sampler tx_sampler =
          mode == TextureFilterMode::Nearest    ? sampler(address::clamp_to_edge, mag_filter::nearest, min_filter::nearest, mip_filter::nearest)
        : mode == TextureFilterMode::Linear     ? sampler(address::clamp_to_edge, mag_filter::nearest, min_filter::linear,  mip_filter::nearest)
        : mode == TextureFilterMode::Mipmap     ? sampler(address::clamp_to_edge, mag_filter::nearest, min_filter::linear,  mip_filter::linear)
        : mode == TextureFilterMode::Anistropic ? sampler(address::clamp_to_edge, mag_filter::nearest, min_filter::linear,  mip_filter::linear, max_anisotropy(4))
        : sampler();
    const half4 color   = texture.sample(tx_sampler, in.tx_coord);
    const half4 ambient = 0.1;
    return color + ambient;
};
