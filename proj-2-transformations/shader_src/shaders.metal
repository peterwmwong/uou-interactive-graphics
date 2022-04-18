#include "./common.h"
#include <metal_stdlib>

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    float  size     [[point_size]];
};

vertex VertexOut
main_vertex(         uint           instance_id [[instance_id]],
            constant float&         max_value   [[buffer(VertexBufferIndexMaxPositionValue)]],
            constant packed_float3* positions   [[buffer(VertexBufferIndexPositions)]])
{
    const float3 pos = positions[instance_id] / max_value;
    return { .position = float4(pos, 1.0), .size = 2.0 };
}

fragment half4
main_fragment(VertexOut in [[stage_in]])
{
    return half4(1);
};
