#include "./common.h"
#include <metal_stdlib>

using namespace metal;

struct VertexOut
{
    float4 position      [[position]];
    float3 view_position;
    // TODO: Can we read the depth from the depth buffer/texture.
    float  z             [[center_no_perspective]];
    float3 normal_dir;
};

vertex VertexOut
main_vertex(         uint           inst_id          [[instance_id]],
                     uint           vertex_id        [[vertex_id]],
            constant uint          *indices          [[buffer(VertexBufferIndexIndices)]],
            constant packed_float3 *positions        [[buffer(VertexBufferIndexPositions)]],
            constant packed_float3 *normals          [[buffer(VertexBufferIndexNormals)]],
            constant float4x4      &normal_transform [[buffer(VertexBufferIndexNormalTransform)]],
            constant float4x4      &mv_transform     [[buffer(VertexBufferIndexModelView)]],
            constant float4x4      &mvp_transform    [[buffer(VertexBufferIndexModelViewProjection)]])
{
    const uint   idx            = indices[inst_id * 3 + vertex_id];
    const float4 model_position = float4(positions[idx], 1.0);
    const float4 position       = mvp_transform * model_position;
    const float4 view_position  = mv_transform  * model_position;

    const float4 model_normal   = float4(normals[idx], 1.0);
    const float4 normal_raw     = normal_transform * model_normal;
    const float3 normal_dir     = normalize(normal_raw.xyz / normal_raw.w);
    return {
        .position            = position,
        .view_position       = view_position.xyz,
        .z                   = position.z / position.w,
        .normal_dir          = normal_dir
    };
}

fragment half4
main_fragment(         VertexOut  in          [[stage_in]],
              constant float4x4  &inv_mvp     [[buffer(FragBufferIndexInverseProjection)]],
              constant float2    &screen_size [[buffer(FragBufferIndexScreenSize)]])
{
    /*
    Rendering Equation
    ==================

    let n       = Normal - unit vector, world space direction perpendicular to surface
    let w       = Light  - unit vector, world space direction to the light source
    let c       = Camera - unit vector, world space direction to the camera
    let a       = (theta) angle between n and w
    let I       = Light Intensity
    let k       = Material Color
    let Fr(w,c) = Bidirectional Reflectance Distribution Function
                = 1 (diffuse)

    = I cos(a) k Fr(w, c)
    = I n.w    k
    */
    const float  I        = 1.f;
    const float3 k        = float3(1.f, 0.f, 0.f);
    const float3 n        = float3(in.normal_dir);

    // TODO: Still trying to figure out the right way to get the view position (camera space)
    // - Currently comparing directional vectors (view_dir and view_dir_persp)...
    // - Let's look at view_position and comparing center_perspective/center_no_perspective
    // - Once we know which one is actually correct...
    //      - How do we calculate this value from screen position (in.position) and depth (see below)

    const float2 screen_pos = in.position.xy;
    const float2 ndc_pos_xy = float2(2.0, -2.0) * ((screen_pos / screen_size) - 0.5);
    const float4 ndc_pos    = float4(ndc_pos_xy, in.z, 1.f);

    const float4 view_position_perspective = inv_mvp * ndc_pos;
    const float3 view_position_calc = view_position_perspective.xyz / view_position_perspective.w;

    /*
    Verify recalculating fragment view position (camera space coordinate) is correct/accurate.
    - Visualize error from expected (view position passed from Vertex Shader)...
        - xy within a thousandth
        - z  within a hundredth

    const float3 actual     = view_position_calc;
    const float3 expected   = in.view_position;
    const float3 diff_pos       = abs(actual - expected);
    return half4(
        float4(
            max((diff_pos.xy * 1000.f) - 1.0f, 0.f),
            max((diff_pos.z * 100.f) - 1.0f, 0.f),
            1.h)
    );
    */

    /*
    Verify cosine(theta) using recalculated fragment view position is correct/accurate.
    - Visualize error from expected (dot product w/normal using view position passed from Vertex Shader)...
        - Blue:  if error >1e4, should be *none*
        - Red:   if error >5e5, should be very little
        - Green: if error >1e5, should be most. Meaning, overall accuracy is within ~1e-5, nice!

    const float3 w_actual           = float3(-normalize(view_position_calc));
    const float  cosTheta0_actual   = dot(w_actual, n);
    const float3 w_expected         = float3(-normalize(in.view_position));
    const float  cosTheta0_expected = dot(w_expected, n);
    const float  diff_cosTheta      = abs(cosTheta0_expected - cosTheta0_actual);
    if (diff_cosTheta > (0.01745329251 * 1e-4f)) {
        return half4(0, 0, 1, 1);
    }
    if (diff_cosTheta > (0.01745329251 * 5e-5f)) {
        return half4(1, 0, 0, 1);
    }
    if (diff_cosTheta > (0.01745329251 * 1e-5f)) {
        return half4(0, 1, 0, 1);
    }
    return half4(0,0,0,1);
    */

    // max() to remove light rays that bounce away from the camera:
    // - Back-facing surfaces, like inside the teapot/spout when viewing teapot from above.
    //      - TODO: Should we render back-faces? Do abs(), instead of max()?
    // - Possibly floating point precision issues.
    //      - If you highlight the fragments with negative `cosTheta0`...
    //      - You'll notice a very small number of pixels around the very edge of the teapot
    //      - Inspecting the value of `cosTheta0`, most are within 3 degrees of 0.
    const float3 w         = float3(-normalize(view_position_calc));
    const float  cosTheta0 = dot(w, n);
    const float  cosTheta  = max(cosTheta0, 0.f);
    const float3 color     = I * cosTheta * k;
    return half4(half3(color), 1.0h);
};
