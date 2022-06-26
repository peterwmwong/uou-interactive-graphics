// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

#ifdef __METAL_VERSION__

#include <metal_stdlib>
using namespace metal;

#define CONSTANT_PTR(x) constant x*
#define TEXTURE(x) x

#else

#define CONSTANT_PTR(x) unsigned long
#define TEXTURE(x) unsigned long

#endif

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
    float4   camera_position;
};

enum struct BGFragBufferIndex: unsigned int
{
    World = 0,
    LENGTH
};

enum struct BGFragTextureIndex: unsigned int
{
    CubeMapTexture = 0,
    LENGTH
};


enum struct VertexBufferIndex: unsigned int
{
    Geometry = 0,
    World,
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
    LENGTH
};


#endif