#pragma once

// Header containing types and enum constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#include "../../metal-shaders/shader_src/bindings/macros.h"

DEF_CONSTANT constexpr float INITIAL_CAMERA_DISTANCE = 50.0;

struct Geometry {
    ARG_CONSTANT_PTR(uint)          indices;
    ARG_CONSTANT_PTR(packed_float3) positions;
};

struct VertexInput {
    float4 mins;
    float4 maxs;
    float2 screen_size;
    float2 camera_rotation;
    float  camera_distance;
    bool   use_perspective;
};

enum struct VertexBufferIndex
{
    VertexInput = 0,
    Geometry,
    LENGTH,
};
