#include <metal_stdlib>
#include "../../metal-shaders/shader_src/shading.h"
#include "./shader_bindings.h"

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
    float2 tx_coord;
};

vertex VertexOut
main_vertex(         uint         vertex_id [[vertex_id]],
            constant ModelSpace & model     [[buffer(VertexBufferIndex::ModelSpace)]],
            constant Geometry   & geometry  [[buffer(VertexBufferIndex::Geometry)]])
{
    const uint idx = geometry.indices[vertex_id];
    return {
        .position = model.matrix_model_to_projection * float4(geometry.positions[idx], 1.0),
        .normal   = model.matrix_normal_to_world     * float3(geometry.normals[idx]),
        .tx_coord = geometry.tx_coords[idx]          * float2(1,-1) + float2(0,1)
    };
}

fragment half4
main_fragment(         VertexOut                 in        [[stage_in]],
              constant ProjectedSpace          & camera    [[buffer(FragBufferIndex::CameraSpace)]],
              constant ProjectedSpace          & light     [[buffer(FragBufferIndex::LightSpace)]],
              constant Material                & material  [[buffer(FragBufferIndex::Material)]],
                       depth2d<float,
                               access::sample>   shadow_tx [[texture(FragTextureIndex::ShadowMap)]])
{
    float4 pos = camera.matrix_screen_to_world * float4(in.position.xyz, 1);
           pos = pos / pos.w;

    float4 pos_light = light.matrix_world_to_projection * pos;
           pos_light = pos_light / pos_light.w;

    // Samples outside the shadow map are considered *NOT* in shadow (border_color::opaque_white).
    // When the light is close to the model, the shadow map projects on a smaller area than the
    // visible range.
    constexpr sampler sampler(address::clamp_to_border,
                              border_color::opaque_white,
                              compare_func::less_equal,
                              filter::linear);
    const bool is_shadow = shadow_tx.sample_compare(sampler, pos_light.xy, pos_light.z) < 1;

    return shade_phong_blinn(
        {
            .frag_pos     = half3(pos.xyz),
            .light_pos    = half3(light.position_world.xyz),
            .camera_pos   = half3(camera.position_world.xyz),
            .normal       = half3(normalize(in.normal)),
            .has_ambient  = HasAmbient,
            .has_diffuse  = HasDiffuse,
            .has_specular = HasSpecular,
            .only_normals = OnlyNormals,
        },
        TexturedMaterial(material, in.tx_coord, is_shadow)
    );
};
