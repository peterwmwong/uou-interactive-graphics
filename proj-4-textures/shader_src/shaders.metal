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

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// Use ProjectedSpace and ModelSpace ()

vertex VertexOut
main_vertex(         uint       vertex_id [[vertex_id]],
            constant Geometry & geometry  [[buffer(VertexBufferIndex::Geometry)]],
            constant World    & world     [[buffer(VertexBufferIndex::World)]])
{
    const uint   idx      = geometry.indices[vertex_id];
    const float4 position = float4(geometry.positions[idx], 1.0);
    const float3 normal   = geometry.normals[idx];
    const float2 tx_coord = geometry.tx_coords[idx];
    return {
        .position  = world.matrix_model_to_projection * position,
        .normal    = world.matrix_normal_to_world * normal,
        // TODO: Should flipping-x be determined by some data in the material?
        .tx_coord  = float2(tx_coord.x, 1. - tx_coord.y)
    };
}

fragment half4
main_fragment(         VertexOut   in       [[stage_in]],
              constant Material  & material [[buffer(FragBufferIndex::Material)]],
              constant World     & world    [[buffer(FragBufferIndex::World)]])
{
    // Calculate the fragment's World Space position from a Metal Viewport Coordinate.
    float4 pos = world.matrix_screen_to_world * float4(in.position.xyz, 1);
           pos   = pos / pos.w;
    return shade_phong_blinn(
        {
            .frag_pos     = half3(pos.xyz),
            .light_pos    = half3(world.light_position.xyz),
            .camera_pos   = half3(world.camera_position.xyz),
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
light_vertex(constant World & world [[buffer(LightVertexBufferIndex::World)]])
{
    return {
        .position = world.matrix_world_to_projection * world.light_position,
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
