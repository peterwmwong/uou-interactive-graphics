#include "./common.h"
#include <metal_stdlib>

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
};

vertex VertexOut
main_vertex(         uint            inst_id          [[instance_id]],
                     uint            vertex_id        [[vertex_id]],
            constant uint          * indices          [[buffer(VertexBufferIndexIndices)]],
            constant packed_float3 * positions        [[buffer(VertexBufferIndexPositions)]],
            constant packed_float3 * normals          [[buffer(VertexBufferIndexNormals)]],
            constant float4x4      & mvp_transform    [[buffer(VertexBufferIndexModelViewProjection)]],
            constant float3x3      & normal_transform [[buffer(VertexBufferIndexNormalTransform)]])
{
    const uint   idx            = indices[inst_id * 3 + vertex_id];
    const float4 model_position = float4(positions[idx], 1.0);
    const float4 position       = mvp_transform * model_position;

    // Assumptions:
    // 1. model_normal is a unit vector.
    // 2. normal_transform does NOT translate nor scale.
    const float3 model_normal   = normals[idx];
    const float3 normal         = normal_transform * model_normal;
    return {
        .position = position,
        .normal   = normal
    };
}

fragment half4
main_fragment(         VertexOut  in          [[stage_in]],
              constant FragMode  &mode        [[buffer(FragBufferIndexFragMode)]],
              constant float4x4  &inv_mvp     [[buffer(FragBufferIndexInverseProjection)]],
              constant float2    &screen_size [[buffer(FragBufferIndexScreenSize)]])
{
    if (mode == FragModeNormals) {
        return half4(half3(in.normal * float3(1,1,-1)), 1);
    }

    const float2 screen_pos = in.position.xy;
    const float2 ndc_pos_xy = float2(2.f, -2.f) * ((screen_pos / screen_size) - 0.5);
    const float4 ndc_pos    = float4(ndc_pos_xy, in.position.z, 1.f);

    const float4 view_pos_perspective = inv_mvp * ndc_pos;
    const float3 view_pos             = view_pos_perspective.xyz / view_pos_perspective.w;

    const float3 n  = float3(in.normal);            // Normal - unit vector, world space direction perpendicular to surface
    const float3 w  = float3(-normalize(view_pos)); // Light  - unit vector, world space direction to light
    const float3 v  = w;                            // Camera - unit vector, world space direction to camera
    const float3 h  = (n + v) / length(n + v);      // Half   - unit vector, world space direction half-way Light and Camera
    const float  Il = 1.0f;                         // Light Intensity
    const float  Ia = 0.1f;                         // Ambient Intensity
    const float3 kd = float3(1.f, 0.f, 0.f);        // Material Difuse Color
    const float3 ks = float3(1.f);                  // Material Specular Color
    const float  s  = 100.f;                        // Shineness (Specular)

    /*
    ================================================================
    Rendering Equation: Ambient + Geometry Term (Diffuse + Specular)
    ================================================================

    F(w, v) = Bidirectional Reflectance Distribution Function
            = 1 (diffuse)

    Ambient + Geometry Term (Diffuse    + Specular)
    -------   -------------  ----------   -------------------------------
    Ia k    + Il cos(a)     (kd F(w, v) + (cos(p) ks F(w, v))^s / cos(a))
    Ia k    + Il cos(a)     (kd         + (cos(p) ks)^s         / cos(a))
    Ia k    + Il w.n        (kd         + (w.h ks)^s            / w.n)
    */
    const float  wn       = max(dot(w, n), 0.f); // max() to remove light rays that bounce away from the camera:
                                                    // - Back-facing surfaces, like inside the teapot/spout when viewing teapot from above.
                                                    //      - TODO: Should we render back-faces? Do abs(), instead of max()?
                                                    // - Possibly floating point precision issues.
                                                    //      - If you highlight the fragments with negative `cosTheta0`...
                                                    //      - You'll notice a very small number of pixels around the very edge of the teapot
                                                    //      - Inspecting the value of `cosTheta0`, most are within 3 degrees of 0.
    const float3 geoTerm  = Il * wn;
    const float3 ambient  = select(0, Ia * kd,                     mode == FragModeAmbientDiffuseSpecular || mode == FragModeAmbient || mode == FragModeAmbientDiffuse);
    const float3 diffuse  = select(0, kd,                          mode == FragModeAmbientDiffuseSpecular || mode == FragModeAmbientDiffuse);
    const float3 specular = select(0, pow(dot(w, h) * ks, s) / wn, mode == FragModeAmbientDiffuseSpecular || mode == FragModeSpecular      );
    const half3  color    = half3(ambient + geoTerm * (diffuse + specular));

    return half4(color, 1.0h);
};
