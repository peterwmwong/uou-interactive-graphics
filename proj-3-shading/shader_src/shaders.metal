#include "./common.h"
#include <metal_stdlib>

using namespace metal;

struct VertexOut
{
    float4 position  [[position]];
    float3 world_pos [[center_no_perspective]];
    float3 normal;
};

vertex VertexOut
main_vertex(         uint            inst_id          [[instance_id]],
                     uint            vertex_id        [[vertex_id]],
            constant uint          * indices          [[buffer(VertexBufferIndexIndices)]],
            constant packed_float3 * positions        [[buffer(VertexBufferIndexPositions)]],
            constant packed_float3 * normals          [[buffer(VertexBufferIndexNormals)]],
            constant float4x4      & model_to_proj    [[buffer(VertexBufferIndexMatrixModelToProjection)]],
            constant float4x4      & model_to_world   [[buffer(VertexBufferIndexMatrixModelToWorld)]])
{
    const uint   idx       = indices[inst_id * 3 + vertex_id];
    const float4 model_pos = float4(positions[idx], 1.0);
    const float4 proj_pos  = model_to_proj * model_pos;
    const float4 world_pos = model_to_world * model_pos;

    const float3 model_normal    = normals[idx];
    // Same as model_to_world, except NO translation.
    // Since normals are directions (not positions), translations are meaningless and should not be
    // applied.
    const float3x3 normal_to_world = float3x3(
        model_to_world[0][0],
        model_to_world[0][1],
        model_to_world[0][2],
        model_to_world[1][0],
        model_to_world[1][1],
        model_to_world[1][2],
        model_to_world[2][0],
        model_to_world[2][1],
        model_to_world[2][2]
    );
    const float3 normal = normal_to_world * model_normal;
    return {
        .position  = proj_pos,
        .world_pos = world_pos.xyz,
        .normal    = normal
    };
}

fragment half4
main_fragment(         VertexOut       in            [[stage_in]],
              constant FragMode      & mode          [[buffer(FragBufferIndexFragMode)]],
              constant float4x4      & proj_to_world [[buffer(FragBufferIndexMatrixProjectionToWorld)]],
              constant packed_float2 & screen_size   [[buffer(FragBufferIndexScreenSize)]],
              // TODO: Figure out how to pass a packed_float3
              constant packed_float4 & light_pos_    [[buffer(FragBufferIndexLightPosition)]],
              // TODO: Figure out how to pass a packed_float3
              constant packed_float4 & cam_pos_      [[buffer(FragBufferIndexCameraPosition)]])
{
    const float3 n = normalize(in.normal); // Normal - unit vector, world space direction perpendicular to surface
    if (mode == FragModeNormals) {
        return half4(half3(n * float3(1,1,-1)), 1);
    }

    const float2 screen_pos  = in.position.xy;
    const float2 proj_pos_xy = float2(2.f, -2.f) * ((screen_pos / screen_size) - 0.5);
    const float4 proj_pos    = float4(proj_pos_xy, in.position.z, 1.f);
    const float4 pos_w_persp = proj_to_world * proj_pos;
    const float3 pos         = pos_w_persp.xyz / pos_w_persp.w;
    // FOR DEBUGGING ONLY
    // const float3 world_position_truth = in.world_pos;
    // const float3 world_position_calc_diff = abs(pos - world_position_truth);
    // return half4(half3(world_position_calc_diff), 1.h);

    const float3 light_pos         = light_pos_.xyz;

    // TODO: START HERE
    // TODO: START HERE
    // TODO: START HERE
    // 1. When the light is *BEHIND* the teapot, something doesn't look right
    // 2. When the light is *IN FRONT* the camera, something doesn't look right
    // 3. Rename w -> l
    // 3. Rename v -> c
    const float3 w  = normalize(light_pos - pos);         // Light  - unit vector, world space direction to light
    const float3 v  = normalize(cam_pos_.xyz + pos);      // Camera - unit vector, world space direction to camera
    const float3 h  = normalize((w + v) / length(w + v)); // Half   - unit vector, world space direction half-way Light and Camera
    const float  Il = 1.0f;                               // Light Intensity
    const float  Ia = 0.1f;                               // Ambient Intensity
    const float3 kd = float3(1.f, 0.f, 0.f);              // Material Difuse Color
    const float3 ks = float3(1.f);                        // Material Specular Color
    const float  s  = 200.f;                              // Shineness (Specular)

    /*
    ================================================================
    Rendering Equation: Ambient + Geometry Term (Diffuse + Specular)
    ================================================================

    F(w, v) = Bidirectional Reflectance Distribution Function
            = 1 (diffuse)

    Ambient + Geometry Term (Diffuse    + Specular)
    -------   -------------  ----------   -------------------------------
    Ia k    + Il cos(a)     (kd F(w, v) + (cos(t) ks F(w, v))^s / cos(a))
    Ia k    + Il cos(a)     (kd         + (cos(t) ks)^s         / cos(a))
    Ia k    + Il w.n        (kd         + (n.h ks)^s            / w.n)
    */
    const float  wn       = max(dot(w, n), 0.f); // Cosine angle between Light and Normal
                                                 // max() to remove light rays that bounce away from the camera:
                                                 // - Back-facing surfaces, like inside the teapot/spout when viewing teapot from above.
                                                 //      - TODO: Should we render back-faces? Do abs(), instead of max()?
                                                 // - Possibly floating point precision issues.
                                                 //      - If you highlight the fragments with negative `cosTheta0`...
                                                 //      - You'll notice a very small number of pixels around the very edge of the teapot
                                                 //      - Inspecting the value of `cosTheta0`, most are within 3 degrees of 0.
    const float3 geoTerm  = Il * wn;
    const float3 ambient  = select(0, Ia * kd,                     mode == FragModeAmbientDiffuseSpecular || mode == FragModeAmbient || mode == FragModeAmbientDiffuse);
    const float3 diffuse  = select(0, kd,                          mode == FragModeAmbientDiffuseSpecular || mode == FragModeAmbientDiffuse);
    const float3 specular = select(0, pow(dot(n, h) * ks, s) / wn, mode == FragModeAmbientDiffuseSpecular || mode == FragModeSpecular      );
    const half3  color    = half3(ambient + geoTerm * (diffuse + specular));

    // TODO: BUG! When the light is positioned somewhere above the teapot, the very bottom edge of the teapot lights up!
    return half4(color, 1.0h);
};

struct LightVertexOut {
    float4 position [[position]];
    float size      [[point_size]];
};

vertex LightVertexOut
light_vertex(constant float4x4      & vp        [[buffer(LightVertexBufferIndexViewProjection)]],
             constant packed_float4 & light_pos [[buffer(LightVertexBufferIndexLightPosition)]])
{
    // TODO: light_pos is now in world coordinates, MVP needs to be applied
    return {
        .position = light_pos,
        .size = 50.f,
    };
}


fragment half4
light_fragment(const float2 point_coord [[point_coord]])
{
    float circle_sd = 1.0 - length(point_coord - float2(0.5));
    return half4(1, 1, 1, half(round(circle_sd)));
};