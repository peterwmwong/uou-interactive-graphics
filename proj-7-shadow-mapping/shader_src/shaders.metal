#include <metal_stdlib>
#include "../../proj-6-environment-mapping/shader_src/shading.h"
#include "./common.h"

using namespace metal;

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// Render Light Camera

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
};

vertex VertexOut
main_vertex(         uint       vertex_id              [[vertex_id]],
            constant Space    & camera                 [[buffer(VertexBufferIndex::Space)]],
            constant Geometry & geometry               [[buffer(VertexBufferIndex::Geometry)]])
{
    const uint   idx    = geometry.indices[vertex_id];
    const float4 pos    = camera.matrix_model_to_projection * float4(geometry.positions[idx], 1.0);
    const float3 normal = camera.matrix_normal_to_world * float3(geometry.normals[idx]);
    return { .position = pos, .normal = normal };
}

fragment half4
main_fragment(         VertexOut         in        [[stage_in]],
              constant Space           & camera    [[buffer(FragBufferIndex::CameraSpace)]],
              constant Space           & light     [[buffer(FragBufferIndex::LightSpace)]],
              constant float4          & diffuse   [[buffer(FragBufferIndex::DiffuseColor)]],
                       depth2d<float, access::sample> shadow_tx [[texture(FragTextureIndex::ShadowMap)]])
{
    float4 pos = camera.matrix_screen_to_world * float4(in.position.xyz, 1);
           pos = pos / pos.w;

    float4 pos_light = light.matrix_world_to_projection * pos;
           pos_light = pos_light / pos_light.w;

    constexpr sampler sampler(address::clamp_to_zero,
                              filter::linear,
                              compare_func::less_equal);
    const half not_shadow = select(
        1.h,
        half(shadow_tx.sample_compare(sampler, pos_light.xy, pos_light.z)),
        all(pos_light.xy >= 0.0) && all(pos_light.xy <= 1.0)
    );

    // TODO: Investigate shadow AA methods. This is smooths... a little teensy bit.
    // const float shadow_amt = 1.0 - length_squared(shadow_tx.gather_compare(sampler,
    //                                                  shadow_tx_coord,
    //                                                  pos_in_shadow_space.z - BIAS)) * 0.25;
    // const half4 color = half4(half3(shadow_amt), 1.);
    const half4 diffuse_color = half4(diffuse) * not_shadow;
    const half4 specular_color = half4(1 * not_shadow);
    return shade_phong_blinn(half3(pos.xyz),
                             half3(light.position_world.xyz),
                             half3(camera.position_world.xyz),
                             half3(normalize(in.normal)),
                             Material(half4(0.75), diffuse_color, specular_color, 50));
};

vertex VertexOut
plane_vertex(        uint       vertex_id     [[vertex_id]],
                     uint       plane_y_unorm [[instance_id]],
            constant Space    & camera        [[buffer(VertexBufferIndex::Space)]])
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
        .position = camera.matrix_world_to_projection * float4(v[0], plane_y, v[1], 1.0),
        .normal   = float3(0, 1, 0),
    };
}
