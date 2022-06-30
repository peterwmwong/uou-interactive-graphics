#include <metal_stdlib>
#include "../../metal-shaders/shader_src/shading.h"
#include "./shader_bindings.h"

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
};

vertex VertexOut
main_vertex(         uint    vertex_id [[vertex_id]],
            constant Space & camera    [[buffer(FragBufferIndex::CameraSpace)]])
{
    // Vertices of Plane front and upright (like a wall), along the x/y axis.
    constexpr const float plane_size = 0.9;
    constexpr const float2 verts_xy[4] = {
        {-1, -1}, // Bottom Left
        {-1,  1}, // Top    Left
        { 1, -1}, // Bottom Rigt
        { 1,  1}, // Top    Right
    };
    const float2 v = verts_xy[vertex_id] * plane_size;
    return {
        .position = camera.matrix_world_to_projection * float4(v[0], v[1], 0, 1),
        .normal   = float3(0, 0, -1),
    };
}

fragment half4
main_fragment(         VertexOut   in     [[stage_in]],
              constant Space     & camera [[buffer(FragBufferIndex::CameraSpace)]],
              constant Space     & light  [[buffer(FragBufferIndex::LightSpace)]])
{
    const float4 pos_w       = camera.matrix_screen_to_world * float4(in.position.xyz, 1);
    const half4  plane_color = 1;
    return shade_phong_blinn(
        {
            .frag_pos   = half3(pos_w.xyz / pos_w.w),
            .light_pos  = half3(light.position_world.xyz),
            .camera_pos = half3(camera.position_world.xyz),
            .normal     = half3(normalize(in.normal)),
        },
        ConstantMaterial(plane_color, plane_color, plane_color, 50)
    );
};
