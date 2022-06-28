// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

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

struct Geometry {
    ARG_CONSTANT_PTR(uint)          indices;
    ARG_CONSTANT_PTR(packed_float3) positions;
    ARG_CONSTANT_PTR(packed_float3) normals;
    ARG_CONSTANT_PTR(packed_float2) tx_coords;
};

struct World {
    float4x4 matrix_model_to_projection;
    float3x3 matrix_normal_to_world;
    float4x4 matrix_world_to_projection;
    float4x4 matrix_screen_to_world;
    float4   camera_position;
    float4   light_position;
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
    ShadowMapWorld,
    LENGTH
};

enum struct FragTextureIndex: unsigned int
{
    ShadowMap = 0,
    LENGTH
};

#endif