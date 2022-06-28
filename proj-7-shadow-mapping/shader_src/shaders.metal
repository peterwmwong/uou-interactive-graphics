#include <metal_stdlib>
#include "./common.h"

using namespace metal;

struct ShadowVertexOut
{
    float4 position [[position]];
};

vertex ShadowVertexOut
shadow_map_vertex(         uint       vertex_id [[vertex_id]],
                  constant World    & shadow    [[buffer(VertexBufferIndex::World)]],
                  constant Geometry & geometry  [[buffer(VertexBufferIndex::Geometry)]])
{
    const uint   idx    = geometry.indices[vertex_id];
    const float4 pos    = shadow.matrix_model_to_projection * float4(geometry.positions[idx], 1.0);
    return { .position = pos };
}

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
main_fragment(         VertexOut         in        [[stage_in]],
              constant World           & world     [[buffer(FragBufferIndex::World)]],
              constant World           & shadow    [[buffer(FragBufferIndex::ShadowMapWorld)]],
                       depth2d<float>    shadow_tx [[texture(FragTextureIndex::ShadowMap)]])
{
    const float4 pos_w           = world.matrix_screen_to_world * float4(in.position.xyz, 1);
    const float4 pos             = float4(pos_w.xyz / pos_w.w, 1.0);
    const float4 shadow_pos_w    = shadow.matrix_world_to_projection * pos;
          float2 shadow_tx_coord = (shadow_pos_w.xy / shadow_pos_w.w * 0.5) + 0.5;
    shadow_tx_coord.y = 1 - shadow_tx_coord.y;

    const float2  pos_from_shadow_w = (shadow.matrix_world_to_projection * pos).zw;
    const float   frag_depth = pos_from_shadow_w.x / pos_from_shadow_w.y;

    // constexpr sampler tx_sampler(mag_filter::linear, address::clamp_to_zero, min_filter::linear);
    // const float depth = shadow_tx.sample(tx_sampler, float2(shadow_tx_coord.x, 1.0 - shadow_tx_coord.y));
    constexpr float BIAS = 0.004;
    constexpr sampler tx_sampler(coord::normalized,
                                 address::clamp_to_edge,
                                 filter::linear,
                                 compare_func::greater_equal);
    const float is_shadow = shadow_tx.sample_compare(tx_sampler,
                                                     shadow_tx_coord,
                                                     frag_depth - BIAS);

    if (is_shadow > 0.0) {
        return half4(0,0,1,1);
    } else {
        return half4(0,1,0,1);
    }
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
plane_fragment(        PlaneVertexOut    in        [[stage_in]],
              constant World           & world     [[buffer(FragBufferIndex::World)]],
              constant World           & shadow    [[buffer(FragBufferIndex::ShadowMapWorld)]],
                       texture2d<half>   shadow_tx [[texture(FragTextureIndex::ShadowMap)]])
{
    return half4(0,1,0,1);
};
