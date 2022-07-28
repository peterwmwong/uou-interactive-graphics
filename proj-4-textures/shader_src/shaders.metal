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
            constant Geometry   & geometry  [[buffer(0)]],
            constant ModelSpace & model     [[buffer(1)]])
{
    const uint   idx      = geometry.indices[vertex_id];
    const float4 position = float4(geometry.positions[idx], 1.0);
    const float3 normal   = geometry.normals[idx];
    const float2 tx_coord = geometry.tx_coords[idx];
    return {
        .position  = model.m_model_to_projection * position,
        .normal    = model.m_normal_to_world * normal,
        // TODO: Should flipping-x be determined by some data in the material?
        .tx_coord  = float2(tx_coord.x, 1. - tx_coord.y)
    };
}

fragment half4
main_fragment(         VertexOut        in        [[stage_in]],
              constant Material       & material  [[buffer(0)]],
              constant ProjectedSpace & camera    [[buffer(1)]],
              constant float4         & light_pos [[buffer(2)]])
{
    // Calculate the fragment's World Space position from a Metal Viewport Coordinate (screen).
    float4 pos = camera.m_screen_to_world * float4(in.position.xyz, 1);
           pos = pos / pos.w;
    return shade_phong_blinn(
        {
            .frag_pos     = half3(pos.xyz),
            .light_pos    = half3(light_pos.xyz),
            .camera_pos   = half3(camera.position_world.xyz),
            .normal       = half3(normalize(in.normal)),
            .has_ambient  = HasAmbient,
            .has_diffuse  = HasDiffuse,
            .has_specular = HasSpecular,
            .only_normals = OnlyNormals,
        },
        TexturedMaterial(material, in.tx_coord)
    );
};


struct LightVertexOut {
    float4 position [[position]];
    float  size     [[point_size]];
};

vertex LightVertexOut
light_vertex(constant ProjectedSpace & camera    [[buffer(0)]],
             constant float4         & light_pos [[buffer(1)]])
{
    return {
        .position = camera.m_world_to_projection * light_pos,
        .size = 50,
    };
}

fragment half4
light_fragment(const float2 point_coord [[point_coord]])
{
    half dist_from_center = length(half2(point_coord) - 0.5h);
    if (dist_from_center > 0.5) discard_fragment();
    return half4(1);
};
