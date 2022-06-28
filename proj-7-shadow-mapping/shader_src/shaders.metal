#include <metal_stdlib>
#include "../../proj-6-environment-mapping/shader_src/shading.h"
#include "./common.h"

using namespace metal;

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// Figure out why light is leaking through the bottom of the teapot
// - Something with the rendered depth into shadow map for the small ring around the teapot cover

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
    float4 pos = world.matrix_screen_to_world * float4(in.position.xyz, 1);
           pos = pos / pos.w;

    float4 pos_in_shadow_space = shadow.matrix_world_to_projection * pos;
           pos_in_shadow_space = pos_in_shadow_space / pos_in_shadow_space.w;

    float2 shadow_tx_coord   = (pos_in_shadow_space.xy * 0.5) + 0.5;
           shadow_tx_coord.y = 1 - shadow_tx_coord.y;

    constexpr float BIAS = 0.004;
    constexpr sampler sampler(coord::normalized,
                              address::clamp_to_edge,
                              filter::linear,
                              compare_func::greater_equal);
    const float is_shadow = shadow_tx.sample_compare(sampler,
                                                     shadow_tx_coord,
                                                     pos_in_shadow_space.z - BIAS);
    const half4 color = half4(is_shadow > 0 ? 0 : 1);

    // TODO: Investigate shadow AA methods. This is smooths... a little teensy bit.
    // const float shadow_amt = 1.0 - length_squared(shadow_tx.gather_compare(sampler,
    //                                                  shadow_tx_coord,
    //                                                  pos_in_shadow_space.z - BIAS)) * 0.25;
    // const half4 color = half4(half3(shadow_amt), 1.);
    return shade_phong_blinn(half3(pos.xyz),
                             half3(shadow.light_position.xyz),
                             half3(shadow.camera_position.xyz),
                             half3(normalize(in.normal)),
                             Material(half4(0.75), color, color, 50));
};

vertex VertexOut
plane_vertex(        uint      vertex_id     [[vertex_id]],
                     uint      plane_y_unorm [[instance_id]],
            constant World    & world        [[buffer(VertexBufferIndex::World)]])
{
    constexpr float MAXUINT = 4294967295;
    const float plane_y = -(float(plane_y_unorm) / MAXUINT);
    // Vertices of Plane laying flat on the ground, along the x/z axis.
    constexpr const float s = 0.9;
    constexpr const float2 verts_xz[4] = {
        {-s, -s}, // Bottom Left
        {-s,  s}, // Top    Left
        { s, -s}, // Bottom Rigt
        { s,  s}, // Top    Right
    };
    const float2 v = verts_xz[vertex_id];
    return {
        .position = world.matrix_world_to_projection * float4(v[0], plane_y, v[1], 1.0),
        .normal   = float3(0, 1, 0),
    };
}
