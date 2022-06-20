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

enum struct FC
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

enum struct WorldID
{
    MatrixModelToProjection = 0,
    MatrixNormalToWorld,
    MatrixWorldToProjection,
    MatrixScreenToWorld,
    LightPosition,
    CameraPosition,
};

struct Material {
    TEXTURE(texture2d<half>) ambient_texture;
    TEXTURE(texture2d<half>) diffuse_texture;
    TEXTURE(texture2d<half>) specular_texture;
    float                    specular_shineness;
};

enum struct VertexBufferIndex
{
    Geometry = 0,
    World,
    LENGTH
};

enum struct FragBufferIndex
{
    Material = 0,
    World,
    LENGTH
};

enum struct LightVertexBufferIndex
{
    World = 0,
    LENGTH,
};

#endif