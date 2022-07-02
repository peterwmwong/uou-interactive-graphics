#pragma once

// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#include "../../metal-shaders/shader_src/bindings/geometry.h"
#include "../../metal-shaders/shader_src/bindings/material.h"

enum struct FC: unsigned char
{
    HasAmbient = 0,
    HasDiffuse,
    HasNormal,
    HasSpecular,
};

struct World {
    float4x4 matrix_model_to_projection;
    float3x3 matrix_normal_to_world;
    float4x4 matrix_world_to_projection;
    float4x4 matrix_screen_to_world;
    float4   light_position;
    float4   camera_position;
};

enum struct VertexBufferIndex: unsigned int
{
    Geometry = 0,
    World,
    LENGTH
};

enum struct FragBufferIndex: unsigned int
{
    Material = 0,
    World,
    LENGTH
};

enum struct LightVertexBufferIndex: unsigned int
{
    World = 0,
    LENGTH,
};
