#include "./common.h"
#include <metal_stdlib>

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    float4 view_position;
    float depth  [[center_no_perspective]];
    // TODO(perf): Can we calculate `view_dir` from `position`?
    // - Try passing an "inverse perspective" transform matrix
    // - `position` has already been mapped to screen space, so it will need to be converted to NDC (view space, [-1,1])
    float3 view_dir_persp [[center_perspective]];
    float3 view_dir       [[center_no_perspective]];
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
    const float3 view_dir       = -normalize(view_position.xyz / view_position.w);

    const float4 model_normal   = float4(normals[idx], 1.0);
    const float4 normal_raw     = normal_transform * model_normal;
    const float3 normal_dir     = normalize(normal_raw.xyz / normal_raw.w);
    return {
        .position       = position,
        .view_position  = view_position,
        .depth          = position.z / position.w,
        .view_dir       = view_dir,
        .view_dir_persp = view_dir,
        .normal_dir     = normal_dir
    };
}

fragment half4
main_fragment(         VertexOut  in          [[stage_in]],
              constant float4x4  &inv_mvp     [[buffer(FragBufferIndexInverseModelViewProjection)]],
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
    const half  I        = 1.0;
    const half3 k        = half3(1, 0, 0);
    const half3 n        = half3(in.normal_dir);


    // TODO: Still trying to figure out the right way to get the view position (camera space)
    // - Currently comparing directional vectors (view_dir and view_dir_persp)...
    // - Let's look at view_position and comparing center_perspective/center_no_perspective
    // - Once we know which one is actually correct...
    //      - How do we calculate this value from screen position (in.position) and depth (see below)

    // const float2 screen_pos = in.position.xy;
    // const float2 ndc_pos_0  = float2(2.0, -2.0) * ((screen_pos / screen_size) - 0.5);
    // const float4 ndc_pos_1  = float4(ndc_pos_0, in.depth, 1.f);
    // const float4 ndc_pos_2  = inv_mvp * ndc_pos_1;
    // const float3 actual     = ndc_pos_2.xyz / ndc_pos_2.w;
    // const float3 expected   = in.view_position.xyz / in.view_position.w;
    // const float3 diff       = abs(actual - expected);
    // return half4(half3(diff.z * 100.0), 1.h);
    // return half4(half2(max((diff.xy * screen_size) - 1.0f, 0.f)), half(diff.z), 1.h);

    const float3 diff = in.view_dir - in.view_dir_persp;
    return half4(half3(diff * 1000.f), 1.h);

    // const half3 w        = half3(in.view_dir);
    // // max() to remove light rays that bounce away from the camera (negative cos(a), a is >90 degrees)
    // const half  cosTheta = max(dot(n, w), 0.h);
    // const half3 color    = I * cosTheta * k;
    // return half4(color, 1.0h);
};
