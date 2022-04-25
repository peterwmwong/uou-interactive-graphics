#include "./common.h"
#include <metal_stdlib>

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
};

vertex VertexOut
main_vertex(         uint           inst_id          [[instance_id]],
                     uint           vertex_id        [[vertex_id]],
            constant uint          *indices          [[buffer(VertexBufferIndexIndices)]],
            constant packed_float3 *positions        [[buffer(VertexBufferIndexPositions)]],
            constant packed_float3 *normals          [[buffer(VertexBufferIndexNormals)]],
            constant float4x4      &normal_transform [[buffer(VertexBufferIndexNormalTransform)]],
            constant float4x4      &mvp_transform    [[buffer(VertexBufferIndexModelViewProjection)]])
{
    const uint   idx            = indices[inst_id * 3 + vertex_id];
    const float4 model_position = float4(positions[idx], 1.0);
    const float4 model_normal   = float4(normals[idx], 1.0);
    return {
        .position    = mvp_transform * model_position,
        .normal      = (normal_transform * model_normal).xyz
    };
}

fragment half4
main_fragment(VertexOut in [[stage_in]])
{
    return half4(abs(half3(in.normal * float3(1,1,-1))), 1.0h);
};
