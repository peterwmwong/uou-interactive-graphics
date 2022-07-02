#pragma once

// Transforms for converting a coordinate or normal direction from Model space to World space.
struct ModelSpace {
    float4x4 matrix_model_to_projection;
    float3x3 matrix_normal_to_world;
};