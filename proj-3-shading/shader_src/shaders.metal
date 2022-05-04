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
            constant float4x4      & normal_transform [[buffer(VertexBufferIndexNormalTransform)]],
            constant float4x4      & mvp_transform    [[buffer(VertexBufferIndexModelViewProjection)]])
{
    const uint   idx            = indices[inst_id * 3 + vertex_id];
    const float4 model_position = float4(positions[idx], 1.0);
    const float4 position       = mvp_transform * model_position;

    // Assumptions:
    // 1. normal_transform does NOT produce a `w` component (no perspective).
    // 2. normal_transform does NOT translate nor scale.
    // 3. model_normal is a unit vector.
    const float4 model_normal   = float4(normals[idx], 1.0);
    const float3 normal         = (normal_transform * model_normal).xyz;
    return {
        .position = position,
        .normal   = normal
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
    const float  I = 1.f;
    const float3 k = float3(1.f, 0.f, 0.f);
    const float3 n = float3(in.normal);

    // TODO: Still trying to figure out the right way to get the view position (camera space)
    // - Currently comparing directional vectors (view_dir and view_dir_persp)...
    // - Let's look at view_position and comparing center_perspective/center_no_perspective
    // - Once we know which one is actually correct...
    //      - How do we calculate this value from screen position (in.position) and depth (see below)

    const float2 screen_pos = in.position.xy;
    const float2 ndc_pos_xy = float2(2.f, -2.f) * ((screen_pos / screen_size) - 0.5);
    const float4 ndc_pos    = float4(ndc_pos_xy, in.position.z, 1.f);

    const float4 view_pos_perspective = inv_mvp * ndc_pos;
    const float3 view_pos             = view_pos_perspective.xyz / view_pos_perspective.w;

    // max() to remove light rays that bounce away from the camera:
    // - Back-facing surfaces, like inside the teapot/spout when viewing teapot from above.
    //      - TODO: Should we render back-faces? Do abs(), instead of max()?
    // - Possibly floating point precision issues.
    //      - If you highlight the fragments with negative `cosTheta0`...
    //      - You'll notice a very small number of pixels around the very edge of the teapot
    //      - Inspecting the value of `cosTheta0`, most are within 3 degrees of 0.
    const float3 w         = float3(-normalize(view_pos));
    const float  cosTheta0 = dot(w, n);
    const float  cosTheta  = max(cosTheta0, 0.f);
    const float3 color     = I * cosTheta * k;
    return half4(half3(color), 1.0h);
};
