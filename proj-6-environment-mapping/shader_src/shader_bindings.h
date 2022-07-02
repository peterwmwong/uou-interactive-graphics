#pragma once

// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#include "../../metal-shaders/shader_src/bindings/geometry.h"

DEF_CONSTANT constexpr unsigned short MIRRORED_INSTANCE_ID = 1;

struct World {
    float4x4 matrix_model_to_projection;
    float4x4 matrix_model_to_world;
    float3x3 matrix_normal_to_world;
    float4x4 matrix_world_to_projection;
    float4x4 matrix_screen_to_world;
    float4   camera_position;
    float    plane_y;
};

enum struct BGFragBufferIndex: unsigned int
{
    World = 0,
    LENGTH
};

enum struct BGFragTextureIndex: unsigned int
{
    CubeMapTexture = 0
};


enum struct VertexBufferIndex: unsigned int
{
    World = 0,
    Geometry,
    LENGTH
};

enum struct FragBufferIndex: unsigned int
{
    World = 0,
    LENGTH
};

enum struct FragTextureIndex: unsigned int
{
    CubeMapTexture = 0,
    ModelTexture
};
