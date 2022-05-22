#include <metal_stdlib>
#include "./common.h"

using namespace metal;

struct VertexOut
{
    float4 position  [[position]];
    float3 normal;
};

vertex VertexOut
main_vertex(         uint            inst_id          [[instance_id]],
                     uint            vertex_id        [[vertex_id]],
            constant uint          * indices          [[buffer(VertexBufferIndexIndices)]],
            constant packed_float3 * positions        [[buffer(VertexBufferIndexPositions)]],
            constant packed_float3 * normals          [[buffer(VertexBufferIndexNormals)]],
            constant float4x4      & model_to_proj    [[buffer(VertexBufferIndexMatrixModelToProjection)]],
            constant float3x3      & normal_to_world  [[buffer(VertexBufferIndexMatrixNormalToWorld)]])
{
    const uint   idx      = indices[inst_id * 3 + vertex_id];
    const float4 position = float4(positions[idx], 1.0);
    const float3 normal   = normals[idx];
    return {
        .position  = model_to_proj * position,
        .normal    = normal_to_world * normal
    };
}

fragment half4
main_fragment(         VertexOut       in            [[stage_in]],
              constant FragMode      & mode          [[buffer(FragBufferIndexFragMode)]],
              constant float4x4      & proj_to_world [[buffer(FragBufferIndexMatrixProjectionToWorld)]],
              constant packed_float2 & screen_size   [[buffer(FragBufferIndexScreenSize)]],
              // TODO: Figure out how to pass a packed_float3
              constant packed_float4 & light_pos     [[buffer(FragBufferIndexLightPosition)]],
              // TODO: Figure out how to pass a packed_float3
              constant packed_float4 & cam_pos       [[buffer(FragBufferIndexCameraPosition)]])
{
    const float3 n = normalize(in.normal); // Normal - unit vector, world space direction perpendicular to surface
    if (mode == FragModeNormals) {
        return half4(half3(n * float3(1,1,-1)), 1);
    }

    // Calculate the fragment's World Space position from a Metal Viewport Coordinate.
    // 1. Viewport Coordinate -> Normalized Device Coordinate (aka Projected w/Perspective)
    const float2 screen_pos  = in.position.xy;
    const float2 proj_pos_xy = fma(float2(2, -2), (screen_pos / screen_size), float2(-1, 1));
    // 2. Projected Coordinate -> World Space position
    const float4 proj_pos    = float4(proj_pos_xy, in.position.z, 1);
    const float4 pos_w_persp = proj_to_world * proj_pos;
    const float3 pos         = pos_w_persp.xyz / pos_w_persp.w;

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
    const float3 l  = normalize(light_pos.xyz - pos); // Light  - world space direction from fragment to light
    const float3 c  = normalize(cam_pos.xyz - pos);   // Camera - world space direction from fragment to camera
    const float3 h  = normalize(l + c);               // Half   - half-way vector between Light and Camera
    const float  ln = dot(l, n); // Cosine angle between Light and Normal
    const float  cn = dot(c, n); // Cosine angle between Camera and Normal
    const float  Il = select(    // Light Intensity
                        0,
                        1,
                        // Remove Diffuse and Specular (set Light Intensity to 0) if either...
                        // 1. Light is hitting the back of the surface
                        // 2. Camera is viewing the back of the surface
                        ln > 0 && cn > 0
                      );

    const float3 Kd       = float3(1, 0, 0); // Diffuse Material Color
    const float3 diffuse  = select(
                                0,
                                Il * ln * Kd,
                                mode == FragModeAmbientDiffuseSpecular || mode == FragModeAmbientDiffuse
                            );

    const float3 Ks       = 1;   // Specular Material Color
    const float  s        = 200; // Specular Shineness
    const float3 specular = select(
                                0,
                                Il * pow(dot(h, n) * Ks, s),
                                mode == FragModeAmbientDiffuseSpecular || mode == FragModeSpecular
                            );

    const float  Ia       = 0.1; // Ambient Intensity
    const float3 Ka       = Kd;  // Ambient Material Color
    const float3 ambient  = select(
                                0,
                                Ia * Ka,
                                mode == FragModeAmbientDiffuseSpecular || mode == FragModeAmbient || mode == FragModeAmbientDiffuse
                            );

    return half4(half3(ambient + diffuse + specular), 1.0h);
};

struct LightVertexOut {
    float4 position [[position]];
    float size      [[point_size]];
};

vertex LightVertexOut
light_vertex(constant float4x4      & model_to_proj [[buffer(LightVertexBufferIndexMatrixWorldToProjection)]],
             constant packed_float4 & light_pos_    [[buffer(LightVertexBufferIndexLightPosition)]])
{
    const float4 light_pos = light_pos_;
    return {
        .position = model_to_proj * light_pos,
        .size = 50,
    };
}


fragment half4
light_fragment(const float2 point_coord [[point_coord]])
{
    float circle_sd = 1.0 - length(point_coord - float2(0.5));
    return half4(1, 1, 1, half(round(circle_sd)));
};