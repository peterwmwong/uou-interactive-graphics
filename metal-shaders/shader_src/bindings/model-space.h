#pragma once

// Transforms for converting a coordinate or normal direction from Model space to World space.
struct ModelSpace {
    float4x4 m_model_to_projection;
    float3x3 m_normal_to_world;
};