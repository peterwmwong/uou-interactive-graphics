#include <metal_stdlib>

using namespace metal;

struct TestStruct {
    float one;
};

[[vertex]]
float4 test_vertex(
    constant float  *        buf0      [[buffer(0)]],
    constant float2 &        buf1      [[buffer(1)]],
             uint            vertex_id [[vertex_id]],
    device   float3 *        buf2      [[buffer(2)]],
    device   float3 &        buf3      [[buffer(3)]],
             texture2d<half> tex1      [[texture(1)]],
    constant TestStruct &    buf5      [[buffer(5)]],
    constant TestStruct *    buf4      [[buffer(4)]]
) {
    return float4(buf1, buf2[0].x, buf4->one);
}


[[fragment]]
float4 test_fragment(
             uint            prim_id   [[primitive_id]],
    constant float  *        buf0      [[buffer(0)]],
    constant float2 &        buf1      [[buffer(1)]],
    device   float3 *        buf2      [[buffer(2)]],
    device   float3 &        buf3      [[buffer(3)]],
             texture2d<half> tex1      [[texture(1)]],
    constant TestStruct &    buf5      [[buffer(5)]],
    constant TestStruct *    buf4      [[buffer(4)]],
             float4          position  [[position]]
) {
    return float4(buf1, buf2[0].x, buf4->one);;
}