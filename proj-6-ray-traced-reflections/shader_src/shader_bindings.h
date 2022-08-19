#pragma once

// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#include "../../metal-shaders/shader_src/bindings/geometry.h"
#include "../../metal-shaders/shader_src/bindings/macros.h"
#include "../../metal-shaders/shader_src/bindings/model-space.h"
#include "../../metal-shaders/shader_src/bindings/projected-space.h"
#include "../../metal-shaders/shader_src/bindings/shading-mode.h"

DEF_CONSTANT constexpr unsigned int MAX_DEBUG_RAY_POINTS = 8;

struct DebugRay {
    float4 points[MAX_DEBUG_RAY_POINTS];
    float2 screen_pos;
    bool disabled;
};