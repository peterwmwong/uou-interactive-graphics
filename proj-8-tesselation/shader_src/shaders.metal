#include <metal_stdlib>
#include "../../metal-shaders/shader_src/shading.h"
#include "./shader_bindings.h"

using namespace metal;

typedef MTLQuadTessellationFactorsHalf QuadTessellFactors;

kernel void tessell_compute(constant float              & factor  [[buffer(TesselComputeBufferIndex::TessellFactor)]],
                            device   QuadTessellFactors * out     [[buffer(TesselComputeBufferIndex::OutputTessellFactors)]],
                                     uint                 pid     [[thread_position_in_grid]])
{
    out[pid].edgeTessellationFactor[0] = factor;
    out[pid].edgeTessellationFactor[1] = factor;
    out[pid].edgeTessellationFactor[2] = factor;
    out[pid].edgeTessellationFactor[3] = factor;
    out[pid].insideTessellationFactor[0] = factor;
    out[pid].insideTessellationFactor[1] = factor;
}

struct VertexOut
{
    float4 position [[position]];
    float2 tx_coord;
};

[[patch(quad, 4)]]
[[vertex]] VertexOut
main_vertex(         float2  patch_coord [[position_in_patch]],
            constant Space & camera      [[buffer(VertexBufferIndex::CameraSpace)]])
{
    // Control Points
    constexpr float  size = 0.5;
    constexpr float2 tl = float2(-1, 1);  // top-left
    constexpr float2 tr = float2(1, 1);   // top-right
    constexpr float2 br = float2(1, -1);  // bottom-right
    constexpr float2 bl = float2(-1, -1); // bottom-left

    // Linear interpolation
    const float2 upper_middle = mix(tl, tr, patch_coord.x);
    const float2 lower_middle = mix(br, bl, 1-patch_coord.x);
    const float4 position     = float4(mix(upper_middle, lower_middle, patch_coord.y) * size, 0.0, 1.0);
    return {
        .position = camera.matrix_world_to_projection * position,
        .tx_coord = patch_coord,
    };
}

fragment half4
main_fragment(         VertexOut   in        [[stage_in]],
              constant Space     & camera    [[buffer(FragBufferIndex::CameraSpace)]],
              constant Space     & light     [[buffer(FragBufferIndex::LightSpace)]],
              texture2d<half>      normal_tx [[texture(FragTextureIndex::Normal)]])
{
    const float4 pos_w       = camera.matrix_screen_to_world * float4(in.position.xyz, 1);
    constexpr sampler tx_sampler(mag_filter::linear, address::clamp_to_edge, min_filter::linear);
          half3  normal      = normal_tx.sample(tx_sampler, in.tx_coord).xyz * 2 - 1;
                 normal.z    = -normal.z;
    const half4  plane_color = 1;
    return shade_phong_blinn(
        {
            .frag_pos   = half3(pos_w.xyz / pos_w.w),
            .light_pos  = half3(light.position_world.xyz),
            .camera_pos = half3(camera.position_world.xyz),
            .normal     = normalize(normal),
        },
        ConstantMaterial(plane_color, plane_color, plane_color, 50)
    );
};
