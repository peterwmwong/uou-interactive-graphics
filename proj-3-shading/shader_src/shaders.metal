#include "./common.h"
#include <metal_stdlib>

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
};

vertex VertexOut
main_vertex(uint vertex_id [[instance_id]])
{
    return {
        .position = float4(0)
    };
}

fragment half4
main_fragment(VertexOut in [[stage_in]])
{
    return half4(1.0);
};
