#pragma once

// Header containing types and enum constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#include "../../metal-shaders/shader_src/bindings/shading-mode.h"

enum struct VertexBufferIndex
{
    Indices = 0,
    Positions,
    Normals,
    MatrixModelToProjection,
    MatrixNormalToWorld,
    LENGTH
};

enum struct FragBufferIndex
{
    MatrixProjectionToWorld = 0,
    ScreenSize,
    LightPosition,
    CameraPosition,
    LENGTH,
};

enum struct LightVertexBufferIndex
{
    MatrixWorldToProjection = 0,
    LightPosition,
    LENGTH,
};
