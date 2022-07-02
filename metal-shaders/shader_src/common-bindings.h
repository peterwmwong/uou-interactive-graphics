#pragma once

// Common structs and macros to help share data between Metal and Rust.

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

// A Model object's geometry. Commonly used with `metal_app::model::Model` to load and help
// encode the data to be used by a Vertex Shader.
struct Geometry {
    ARG_CONSTANT_PTR(uint)          indices;
    ARG_CONSTANT_PTR(packed_float3) positions;
    ARG_CONSTANT_PTR(packed_float3) normals;
    ARG_CONSTANT_PTR(packed_float2) tx_coords;
};

// A Model object's material. Commonly used with `metal_app::model::Model` to load and help
// encode textures to be used by a Fragment Shader.
struct Material {
    ARG_TEXTURE(texture2d<half>) ambient_texture;
    ARG_TEXTURE(texture2d<half>) diffuse_texture;
    ARG_TEXTURE(texture2d<half>) specular_texture;
    float                        specular_shineness;
    float                        ambient_amount;
};

// Transforms and origin location defining a projected coordinate space in relation to the World
// coordinate space.
//
// Commonly used to define Cameras and Lights (Shadow Maps). Where you want to render something
// defined in the world coordinate space, as if viewed from the perspective of a Camera or Light. As
// such, it is common that this projected cordinate space matches the Metal Normalized Device
// Coordinates:
// - X dimension: [1,-1], Left to Right
// - Y dimension: [1,-1], Top  to Bottom
// - Z dimension: [0, 1], Near to Far
struct ProjectedSpace {
    // Transform a world coordinate to this projected coordinate space.
    float4x4 matrix_world_to_projection;

    // Transform a screen coordinate, plus projected depth, to a world coordinate.
    // This is useful for Fragment Shader's that want the fragment's world coordinate, without
    // paying the costs of passing it from the Vertex Shader (thread group memory, overworking the
    // hardware interpolator and possibly reduced fragment shader occupancy).
    float4x4 matrix_screen_to_world;

    // World Space Coordinate of this projected space's origin (0,0,0).
    // Put another way...
    //     position_world = matrix_world_to_projection.inverse() * float4(0, 0, 0, 0);
    //     position_world = position_world / position_world.w;
    float4   position_world;
};

// Transforms for converting a coordinate or normal direction from Model space to World space.
struct ModelSpace {
    float4x4 matrix_model_to_projection;
    float3x3 matrix_normal_to_world;
};