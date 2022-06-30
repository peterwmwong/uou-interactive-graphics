#pragma once

// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#ifdef __METAL_VERSION__

#include <metal_stdlib>
using namespace metal;

#define CONSTANT_PTR(x) constant x*
#define TEXTURE(x) x

#else

#define CONSTANT_PTR(x) unsigned long
#define TEXTURE(x) unsigned long

#endif

enum struct FC: unsigned char
{
    HasAmbient = 0,
    HasDiffuse,
    HasNormal,
    HasSpecular,
};

struct Geometry {
    CONSTANT_PTR(uint)          indices;
    CONSTANT_PTR(packed_float3) positions;
    CONSTANT_PTR(packed_float3) normals;
    CONSTANT_PTR(packed_float2) tx_coords;
};

struct World {
    float4x4 matrix_model_to_projection;
    float3x3 matrix_normal_to_world;
    float4x4 matrix_world_to_projection;
    float4x4 matrix_screen_to_world;
    float4   light_position;
    float4   camera_position;
};

struct Material {
    TEXTURE(texture2d<half>) ambient_texture;
    TEXTURE(texture2d<half>) diffuse_texture;
    TEXTURE(texture2d<half>) specular_texture;
    float                    specular_shineness;
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
