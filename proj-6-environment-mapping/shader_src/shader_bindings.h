#pragma once

// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#include "../../metal-shaders/shader_src/bindings/geometry.h"
#include "../../metal-shaders/shader_src/bindings/macros.h"
#include "../../metal-shaders/shader_src/bindings/model-space.h"
#include "../../metal-shaders/shader_src/bindings/projected-space.h"
#include "../../metal-shaders/shader_src/bindings/shading-mode.h"

enum struct VertexBufferIndex: unsigned int
{
    Geometry = 0,
    Camera,
    Model,
    PlaneY,
    LENGTH
};

enum struct FragBufferIndex: unsigned int
{
    Camera = 0,
    LightPosition,
    MatrixEnvironment,
    LENGTH
};

enum struct FragTextureIndex: unsigned int
{
    EnvTexture = 0,
    ModelTexture
};
