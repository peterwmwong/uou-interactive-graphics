#include <metal_stdlib>
#include "./common.h"

using namespace metal;

constant float SPECULAR_SHINENESS [[function_constant(FC_SPECULAR_SHINENESS)]];

struct VertexOut
{
    float4 position [[position]];
    half3  normal;
    half2  tx_coord;
};

vertex VertexOut
main_vertex(         uint            inst_id         [[instance_id]],
                     uint            vertex_id       [[vertex_id]],
            constant uint          * indices         [[buffer(VertexBufferIndex_Indices)]],
            constant packed_float3 * positions       [[buffer(VertexBufferIndex_Positions)]],
            constant packed_float3 * normals         [[buffer(VertexBufferIndex_Normals)]],
            constant packed_float2 * texcoords       [[buffer(VertexBufferIndex_Texcoords)]],
            constant float4x4      & model_to_proj   [[buffer(VertexBufferIndex_MatrixModelToProjection)]],
            constant float3x3      & normal_to_world [[buffer(VertexBufferIndex_MatrixNormalToWorld)]])
{
    const uint   idx      = indices[inst_id * 3 + vertex_id];
    const float4 position = float4(positions[idx], 1.0);
    const float3 normal   = normals[idx];
    const float2 tx_coord = texcoords[idx];
    return {
        .position  = model_to_proj * position,
        .normal    = half3(normal_to_world * normal),
        .tx_coord  = half2(tx_coord)
    };
}

fragment half4
main_fragment(         VertexOut   in            [[stage_in]],
              constant FragMode  & mode          [[buffer(FragBufferIndex_FragMode)]],
              constant float4x4  & proj_to_world [[buffer(FragBufferIndex_MatrixProjectionToWorld)]],
              constant float2    & screen_size   [[buffer(FragBufferIndex_ScreenSize)]],
              constant float3    & light_pos     [[buffer(FragBufferIndex_LightPosition)]],
              constant float3    & cam_pos       [[buffer(FragBufferIndex_CameraPosition)]],
              texture2d<half>      tx_ambient    [[texture(FragBufferIndex_AmbientTexture)]],
              texture2d<half>      tx_specular   [[texture(FragBufferIndex_Specular)]])
{
    const half3 n = normalize(in.normal); // Normal - unit vector, world space direction perpendicular to surface
    if (mode == FragMode_Normals) {
        return half4(n.xy, n.z * -1, 1);
    }

    // Calculate the fragment's World Space position from a Metal Viewport Coordinate.
    // 1. Viewport Coordinate -> Normalized Device Coordinate (aka Projected w/Perspective)
    const half2  screen_pos   = half2(in.position.xy);
    const half2  proj_pos_xy  = fma(half2(2, -2), (screen_pos / half2(screen_size)), half2(-1, 1));
    // 2. Projected Coordinate -> World Space position
    const float4 proj_pos     = float4(float2(proj_pos_xy), in.position.z, 1);
    const float4 pos_w_persp  = proj_to_world * proj_pos;
    const half3  pos          = half3(pos_w_persp.xyz / pos_w_persp.w);

    /*
    ================================================================
    Rendering Equation: Ambient + Geometry Term (Diffuse + Specular)
    ================================================================

    F(l, c) = Bidirectional Reflectance Distribution Function

    Ambient + Geometry Term (Diffuse    + Specular)
    -------   ------------- ----------   -------------------------------
    Ia Kd   + Il cos(a)     (Kd F(l, c) + (cos(t) Ks F(l, c))^s / cos(a))
    Ia Kd   + Il cos(a)     (Kd         + (cos(t) Ks)^s         / cos(a))
    Ia Kd   + Il l.n        (Kd         + (h.n Ks)^s            / l.n)

    ...distribute the Geometry Term...

    Ambient + Diffuse   + Specular
    -------   ---------   ---------------
    Ia Kd   + Il l.n Kd   + Il (h.n Ks)^s
    */
    const half3 l  = normalize(half3(light_pos.xyz) - pos); // Light  - world space direction from fragment to light
    const half3 c  = normalize(half3(cam_pos.xyz) - pos);   // Camera - world space direction from fragment to camera
    const half3 h  = normalize(l + c);               // Half   - half-way vector between Light and Camera
    const half  ln = dot(l, n); // Cosine angle between Light and Normal
    const half  cn = dot(c, n); // Cosine angle between Camera and Normal
    const half  Il = select(    // Light Intensity
                        0,
                        1,
                        // Remove Diffuse and Specular (set Light Intensity to 0) if either...
                        // 1. Light is hitting the back of the surface
                        // 2. Camera is viewing the back of the surface
                        ln > 0 && cn > 0
                      );

    constexpr sampler tx_sampler(mag_filter::linear, address::repeat, min_filter::linear);
    const float2 tx_coord = float2(in.tx_coord);
    const half4 Kd        = tx_ambient.sample(tx_sampler, tx_coord);  // Diffuse Material Color
    const half4 Ks        = tx_specular.sample(tx_sampler, tx_coord); // Specular Material Color

    // TODO: Try collecting final color, instead of storing diffuse/specular/ambient and summing at the end
    // - See if this improves the generated code/instruction count/temporary registers.
    // TODO: Try function specialization with `mode`.
    // - How much of an improvement?
    const half4 diffuse  = select(
                                0,
                                Il * ln * Kd,
                                mode == FragMode_AmbientDiffuseSpecular || mode == FragMode_AmbientDiffuse
                            );

    const half  s        = SPECULAR_SHINENESS;
    const half4 specular = select(
                                0,
                                Il * pow(dot(h, n) * Ks, s),
                                mode == FragMode_AmbientDiffuseSpecular || mode == FragMode_Specular
                            );

    const half  Ia       = 0.1; // Ambient Intensity
    const half4 Ka       = Kd;  // Ambient Material Color
    const half4 ambient  = select(
                                0,
                                Ia * Ka,
                                mode == FragMode_AmbientDiffuseSpecular || mode == FragMode_Ambient || mode == FragMode_AmbientDiffuse
                            );

    return half4(ambient + diffuse + specular);
};


struct LightVertexOut {
    float4 position [[position]];
    float  size     [[point_size]];
};

vertex LightVertexOut
light_vertex(constant float4x4 & model_to_proj [[buffer(LightVertexBufferIndex_MatrixWorldToProjection)]],
             constant float4   & light_pos     [[buffer(LightVertexBufferIndex_LightPosition)]])
{
    return {
        .position = model_to_proj * light_pos,
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