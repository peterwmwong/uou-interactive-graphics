#pragma once

// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#include "../../metal-shaders/shader_src/bindings/geometry.h"
#include "../../metal-shaders/shader_src/bindings/material.h"
#include "../../metal-shaders/shader_src/bindings/projected-space.h"
#include "../../metal-shaders/shader_src/bindings/shading-mode.h"

enum struct TesselComputeBufferIndex: unsigned int
{
    TessellFactor = 0,
    OutputTessellFactors
};

enum struct VertexBufferIndex: unsigned int
{
    MatrixWorldToProjection = 0,
    DisplacementScale,
    LENGTH
};

enum struct VertexTextureIndex: unsigned int
{
    Displacement = 0
};


enum struct FragBufferIndex: unsigned int
{
    CameraSpace = 0,
    LightSpace,
    ShadeTriangulation,
    LENGTH
};

enum struct FragTextureIndex: unsigned int
{
    Normal = 0,
    ShadowMap
};

enum struct LightVertexBufferIndex: unsigned int
{
    MatrixModelToProjection = 0,
    Geometry,
    LENGTH
};

enum struct LightFragBufferIndex: unsigned int
{
    Material = 0,
    LENGTH
};