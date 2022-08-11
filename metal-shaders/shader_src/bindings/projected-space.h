#pragma once

#include "./metal.h"

// Transforms and origin location defining a projected coordinate space in relation to the World
// coordinate space.
//
// Commonly used to define Cameras and Lights (Shadow Maps). Where you want to render something
// defined in the world coordinate space, as if viewed from the perspective of a Camera or Light. As
// such, it is common that this projected cordinate space matches the Metal Normalized Device
// Coordinates:
// - X dimension: [1,-1], Left to Right
// - Y dimension: [1,-1], Top  to Bottom
// - Z dimension: [0, 1], Near to Far
struct ProjectedSpace {
    // Transform a world coordinate to this projected coordinate space.
    float4x4 m_world_to_projection;

    // Transform a screen coordinate, plus projected depth, to a world coordinate.
    // This is useful for Fragment Shader's that want the fragment's world coordinate, without
    // paying the costs of passing it from the Vertex Shader (thread group memory, overworking the
    // hardware interpolator and possibly reduced fragment shader occupancy).
    float4x4 m_screen_to_world;

    // World Space Coordinate of this projected space's origin (0,0,0).
    // Put another way...
    //     position_world = m_world_to_projection.inverse() * float4(0, 0, 0, 0);
    //     position_world = position_world / position_world.w;
    float4   position_world;
};
