#pragma once

// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#include "../../metal-shaders/shader_src/common-bindings.h"

enum struct VertexBufferIndex: unsigned int
{
    ModelSpace = 0,
    Geometry,
    LENGTH
};

enum struct FragBufferIndex: unsigned int
{
    CameraSpace = 0,
    LightSpace,
    Material,
    LENGTH
};

enum struct FragTextureIndex: unsigned int
{
    ShadowMap = 0
};
