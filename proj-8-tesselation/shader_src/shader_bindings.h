#pragma once

// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#ifdef __METAL_VERSION__

#include <metal_stdlib>
using namespace metal;

#define ARG_CONSTANT_PTR(x) constant x*
#define ARG_TEXTURE(x) x
#define DEF_CONSTANT constant

#else

#define ARG_CONSTANT_PTR(x) unsigned long
#define ARG_TEXTURE(x) unsigned long
#define DEF_CONSTANT

#endif

struct Space {
    float4x4 matrix_world_to_projection;
    float4x4 matrix_screen_to_world;
    float4   position_world;
};

enum struct VertexBufferIndex: unsigned int
{
    CameraSpace = 0,
    LENGTH
};

enum struct FragBufferIndex: unsigned int
{
    CameraSpace = 0,
    LightSpace,
    LENGTH
};
