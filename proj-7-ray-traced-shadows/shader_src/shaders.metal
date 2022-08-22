#include <metal_stdlib>
#include "../../metal-shaders/shader_src/shading.h"
#include "../../metal-types/src/geometry.h"
#include "../../metal-types/src/material.h"
#include "../../metal-types/src/model-space.h"
#include "../../metal-types/src/projected-space.h"
#include "../../metal-types/src/shading-mode.h"

using namespace metal;
using raytracing::primitive_acceleration_structure;

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
        .tx_coord = geometry.tx_coords[idx]     * float2(1,-1) + float2(0,1)
    };
}

fragment half4
main_fragment(         VertexOut                 in           [[stage_in]],
              constant ProjectedSpace          & camera       [[buffer(0)]],
              constant float4                  & light_pos    [[buffer(1)]],
              constant Material                & material     [[buffer(2)]],
              primitive_acceleration_structure   accel_struct [[buffer(3)]]
)
{
    float4 pos = camera.m_screen_to_world * float4(in.position.xyz, 1);
           pos = pos / pos.w;

    const float3 normal   = normalize(in.normal);
    const float3 to_light = normalize(light_pos.xyz - pos.xyz);

    // If the fragment is facing away from the light, don't even bother tracing a ray to determine
    // whether the fragment shadowed or not.
    bool is_shadow = false;
    if (dot(normal, to_light) >= 0.0) {
        raytracing::ray r(pos.xyz, to_light);
        raytracing::intersector<> intersector;
        // TODO: Figure out what there's a tiny little teapot shadow right behind the light when the
        // light is positioned right above the ground... weird.
        intersector.set_triangle_cull_mode(raytracing::triangle_cull_mode::back);
        intersector.assume_geometry_type(raytracing::geometry_type::triangle);
        auto intersection = intersector.intersect(r, accel_struct);
        is_shadow = intersection.type != raytracing::intersection_type::none;
    }
    return shade_phong_blinn(
        {
            .frag_pos     = half3(pos.xyz),
            .light_pos    = half3(light_pos.xyz),
            .camera_pos   = half3(camera.position_world.xyz),
            .normal       = half3(normal),
            .has_ambient  = HasAmbient,
            .has_diffuse  = HasDiffuse,
            .has_specular = HasSpecular,
            .only_normals = OnlyNormals,
        },
        TexturedMaterial(material, in.tx_coord, is_shadow)
    );
};
