#include "./common.h"
#include <metal_stdlib>

using namespace metal;

struct VertexOut
{
    float4 position             [[position]];
};

vertex VertexOut
main_vertex(uint inst_id [[instance_id]],
            uint vert_id [[vertex_id]])
{
    const float2 position = ((float2[]) {
        {  0.0,  1.0 },
        {  1.0, -1.0 },
        { -1.0, -1.0 }
    })[vert_id];
    return { .position = float4(position, 0.0, 1.0) };
}

struct FragOut
{
    half4 color [[color(AttachmentIndexColor)]];
};

fragment FragOut
main_fragment(VertexOut in [[stage_in]])
{
    return { .color = half4(1.0, 1.0, 0.0, 1.0) };
};
