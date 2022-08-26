#pragma once

#ifdef __METAL_VERSION__

#include <metal_stdlib>
using namespace metal;

constant constexpr bool   A_Bool   [[function_constant(0)]];
constant constexpr float  A_Float  [[function_constant(1)]];
constant constexpr float4 A_Float4 [[function_constant(2)]];
constant constexpr uint   A_Uint   [[function_constant(3)]];

#endif