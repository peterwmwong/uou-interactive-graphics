#pragma once

#include "./macros.h"

// A Model object's geometry. Commonly used with `metal_app::model::Model` to load and help
// encode the data to be used by a Vertex Shader.
struct GeometryNoTxCoords {
    ARG_CONSTANT_PTR(uint)          indices;
    ARG_CONSTANT_PTR(packed_float3) positions;
    ARG_CONSTANT_PTR(packed_float3) normals;
};
