#include <metal_stdlib>
#include "../../metal-shaders/shader_src/shading.h"
#include "./shader_bindings.h"

using namespace metal;

typedef MTLQuadTessellationFactorsHalf QuadTessellFactors;

[[kernel]] void
tessell_compute(constant float              & factor  [[buffer(TesselComputeBufferIndex::TessellFactor)]],
                device   QuadTessellFactors * out     [[buffer(TesselComputeBufferIndex::OutputTessellFactors)]],
                         uint                 pid     [[thread_position_in_grid]])
{
    device QuadTessellFactors* o = &out[pid];
    o->edgeTessellationFactor[0] = factor;
    o->edgeTessellationFactor[1] = factor;
    o->edgeTessellationFactor[2] = factor;
    o->edgeTessellationFactor[3] = factor;
    o->insideTessellationFactor[0] = factor;
    o->insideTessellationFactor[1] = factor;
}

struct VertexOut
{
    float4 position [[position]];
    float2 tx_coord;
};

[[patch(quad, 4)]]
[[vertex]] VertexOut
main_vertex(         float2     patch_coord                [[position_in_patch]],
            constant float4x4 & matrix_world_to_projection [[buffer(VertexBufferIndex::MatrixWorldToProjection)]],
            constant float    & displacement_scale         [[buffer(VertexBufferIndex::DisplacementScale)]],
            texture2d<half>     disp_tx                    [[texture(VertexTextureIndex::Displacement)]])
{
    constexpr sampler tx_sampler(mag_filter::linear, address::clamp_to_edge, min_filter::linear);
    const float disp_amount = is_null_texture(disp_tx)
                                ? 0
                                : disp_tx.sample(tx_sampler, patch_coord).z * -displacement_scale;

    // Control Points
    constexpr float2 tl = float2(-1, 1);  // top-left
    constexpr float2 tr = float2(1, 1);   // top-right
    constexpr float2 br = float2(1, -1);  // bottom-right
    constexpr float2 bl = float2(-1, -1); // bottom-left

    // Linear interpolation
    const float2 upper_middle = mix(tl, tr, patch_coord.x);
    const float2 lower_middle = mix(br, bl, 1-patch_coord.x);
    const float2 position_xy  = mix(upper_middle, lower_middle, patch_coord.y);
    return {
        .position = matrix_world_to_projection * float4(position_xy, disp_amount, 1),
        .tx_coord = patch_coord
    };
}

[[fragment]] half4
main_fragment(         VertexOut        in        [[stage_in]],
              constant ProjectedSpace & camera    [[buffer(FragBufferIndex::CameraSpace)]],
              constant ProjectedSpace & light     [[buffer(FragBufferIndex::LightSpace)]],
              constant bool           & shade_tri [[buffer(FragBufferIndex::ShadeTriangulation)]],
              texture2d<half>           normal_tx [[texture(FragTextureIndex::Normal)]],
              depth2d<float,
                      access::sample>   shadow_tx [[texture(FragTextureIndex::ShadowMap)]])
{
    if (shade_tri) return half4(1, 1, 0, 1);

    constexpr sampler tx_sampler(mag_filter::linear,
                                 address::clamp_to_edge,
                                 min_filter::linear);
          half3  normal   = normal_tx.sample(tx_sampler, in.tx_coord).xyz * 2 - 1; // [0,1] -> [-1,1]
                 normal.z = -normal.z;
    const float4 pos_w    = camera.matrix_screen_to_world * float4(in.position.xyz, 1);
    const float3 pos      = pos_w.xyz / pos_w.w;

    constexpr sampler shadow_sampler(address::clamp_to_border,
                                     border_color::opaque_white,
                                     compare_func::less_equal,
                                     filter::linear);
          float4 shadow_tx_coord = light.matrix_world_to_projection * float4(pos, 1);
                 shadow_tx_coord = shadow_tx_coord / shadow_tx_coord.w;
    const float  is_lit          = shadow_tx.sample_compare(shadow_sampler, shadow_tx_coord.xy, shadow_tx_coord.z);
    const half4  diffuse_color   = 0.5 * is_lit;
    const half4  specular_color  = 1.0 * is_lit;
    return shade_phong_blinn(
        {
            .frag_pos   = half3(pos),
            .light_pos  = half3(light.position_world.xyz),
            .camera_pos = half3(camera.position_world.xyz),
            .normal     = half3(normalize(normal)),
        },
        ConstantMaterial(0, diffuse_color, specular_color, 100, 0)
    );
};

struct LightVertexOut
{
    float4 position [[position]];
    float2 tx_coord;
};

vertex LightVertexOut
light_vertex(        uint       vertex_id                  [[vertex_id]],
            constant float4x4 & matrix_model_to_projection [[buffer(LightVertexBufferIndex::MatrixModelToProjection)]],
            constant Geometry & geometry                   [[buffer(LightVertexBufferIndex::Geometry)]])
{
    const uint idx = geometry.indices[vertex_id];
    return {
        .position = matrix_model_to_projection * float4(geometry.positions[idx], 1.0),
        .tx_coord = geometry.tx_coords[idx]    * float2(1,-1) + float2(0,1)
    };
}

fragment half4
light_fragment(         LightVertexOut   in       [[stage_in]],
               constant Material       & material [[buffer(LightFragBufferIndex::Material)]])
{
    constexpr sampler tx_sampler(mag_filter::linear, address::repeat, min_filter::linear);
    return material.ambient_texture.sample(tx_sampler, in.tx_coord);
};
