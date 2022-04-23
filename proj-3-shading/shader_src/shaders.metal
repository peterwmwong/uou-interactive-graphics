#include "./common.h"
#include <metal_stdlib>

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    // TOOD: Add normal
};

vertex VertexOut
main_vertex(         uint           inst_id          [[instance_id]],
                     uint           vertex_id        [[vertex_id]],
            constant uint*          indices          [[buffer(VertexBufferIndexIndices)]],
            constant packed_float3* positions        [[buffer(VertexBufferIndexPositions)]],
            constant float4x4&      mvp              [[buffer(VertexBufferIndexModelViewProjection)]])
{
    const uint   position_idx   = indices[inst_id * 3 + vertex_id];
    const float4 model_position = float4(positions[position_idx], 1.0); // Make homogenous coordinate
    const float4 position = mvp * model_position;
    return {
        .position = position
    };
}

fragment half4
main_fragment(VertexOut in [[stage_in]])
{
    return half4(0, 1, 0, 1);
};
