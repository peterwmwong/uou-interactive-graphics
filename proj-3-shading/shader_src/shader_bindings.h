#pragma once

// Header containing types and enum constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#include "../../metal-shaders/shader_src/bindings/geometry.h"
#include "../../metal-shaders/shader_src/bindings/model-space.h"
#include "../../metal-shaders/shader_src/bindings/projected-space.h"
#include "../../metal-shaders/shader_src/bindings/shading-mode.h"

enum struct VertexBufferIndex
{
    Geometry = 0,
    ModelSpace,
    LENGTH
};

enum struct FragBufferIndex
{
    CameraSpace = 0,
    LightPosition,
    LENGTH,
};

enum struct LightVertexBufferIndex
{
    CameraSpace = 0,
    LightPosition,
    LENGTH,
};
