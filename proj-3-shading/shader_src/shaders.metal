#include "./common.h"
#include <metal_stdlib>

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    float3 view_dir;
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
        .position   = position,
        .view_dir   = view_dir,
        .normal_dir = normal_dir
    };
}

fragment half4
main_fragment(VertexOut in [[stage_in]])
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
    const half3 w        = half3(in.view_dir);
    // max() to remove light rays that bounce away from the camera (negative cos(a), a is >90 degrees)
    const half  cosTheta = max(dot(n, w), 0.h);
    const half3 color    = I * cosTheta * k;
    return half4(color, 1.0h);
};
