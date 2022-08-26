#include <metal_stdlib>
#include "../../metal-shaders/src/shading.h"
#include "../../metal-types/src/geometry.h"
#include "../../metal-types/src/material.h"
#include "../../metal-types/src/model-space.h"
#include "../../metal-types/src/projected-space.h"
#include "../../metal-types/src/shading-mode.h"

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
    float2 tx_coord;
};

vertex VertexOut
main_vertex(         uint         vertex_id [[vertex_id]],
            constant ModelSpace & model     [[buffer(0)]],
            constant Geometry   & geometry  [[buffer(1)]])
{
    const uint idx = geometry.indices[vertex_id];
    return {
        .position = model.m_model_to_projection * float4(geometry.positions[idx], 1.0),
        .normal   = model.m_normal_to_world     * float3(geometry.normals[idx]),
        .tx_coord = geometry.tx_coords[idx]          * float2(1,-1) + float2(0,1)
    };
}

fragment half4
main_fragment(         VertexOut                 in        [[stage_in]],
              constant ProjectedSpace          & camera    [[buffer(0)]],
              constant ProjectedSpace          & light     [[buffer(1)]],
              constant Material                & material  [[buffer(2)]],
                       depth2d<float,
                               access::sample>   shadow_tx [[texture(0)]])
{
    float4 pos = camera.m_screen_to_world * float4(in.position.xyz, 1);
           pos = pos / pos.w;

    float4 pos_light = light.m_world_to_projection * pos;
           pos_light = pos_light / pos_light.w;

    // Samples outside the shadow map are considered *NOT* in shadow (border_color::opaque_white).
    // When the light is close to the model, the shadow map projects on a smaller area than the
    // visible range.
    constexpr sampler sampler(address::clamp_to_border,
                              border_color::opaque_white,
                              compare_func::less_equal,
                              filter::linear);

    // Used when rendering the light, no shadow map texture is bound (null) allowing the light to be
    // lit ambiently.
    const bool is_shadow = is_null_texture(shadow_tx)
                                ? false
                                : shadow_tx.sample_compare(sampler, pos_light.xy, pos_light.z) < 1;

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
