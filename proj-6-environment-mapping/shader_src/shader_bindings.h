#pragma once

// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#include "../../metal-shaders/shader_src/bindings/geometry.h"
#include "../../metal-shaders/shader_src/bindings/macros.h"
#include "../../metal-shaders/shader_src/bindings/model-space.h"
#include "../../metal-shaders/shader_src/bindings/projected-space.h"
#include "../../metal-shaders/shader_src/bindings/shading-mode.h"

DEF_CONSTANT constexpr unsigned short MIRRORED_INSTANCE_ID = 1;

enum struct VertexBufferIndex: unsigned int
{
    Geometry = 0,
    Camera,
    Model,
    MatrixModelToWorld,
    PlaneY,
    LENGTH
};

enum struct FragBufferIndex: unsigned int
{
    Camera = 0,
    LENGTH
};

enum struct FragTextureIndex: unsigned int
{
    CubeMapTexture = 0,
    ModelTexture
};
