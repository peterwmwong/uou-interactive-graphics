#include "./common.h"
#include <metal_stdlib>

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    float  size     [[point_size]];
};

inline float4x4
float4x4_row_major(const float4 r1, const float4 r2, const float4 r3, const float4 r4) {
    return float4x4(
        /* col1 */ r1[0], r2[0], r3[0], r4[0],
        /* col2 */ r1[1], r2[1], r3[1], r4[1],
        /* col3 */ r1[2], r2[2], r3[2], r4[2],
        /* col4 */ r1[3], r2[3], r3[3], r4[3]
    );
}

vertex VertexOut
main_vertex(         uint           vertex_id   [[instance_id]],
            constant packed_float4* mins_maxs   [[buffer(VertexBufferIndexMaxPositionValue)]],
            constant packed_float3* positions   [[buffer(VertexBufferIndexPositions)]],
            constant float&         time_s      [[buffer(VertexBufferIndexTime)]])
{
    const packed_float4 mins             = mins_maxs[0];
    const packed_float4 maxs             = mins_maxs[1];
    const float         height_of_teapot = maxs.z - mins.z;
    const float         depth_of_teapot  = maxs.y - mins.y;
    const float         bounding_radius  = length(float2(height_of_teapot, depth_of_teapot));

    const float4 model_position = float4(positions[vertex_id], 1.0); // Make homogenous coordinate

    // Translate the teapot where the origin is in the middle of the teapot.
    // From the asset file (teapot.obj), the center bottom of the teapot is the origin (0,0,0).
    const float4x4 translate_matrix  = float4x4_row_major(
        {1, 0, 0, 0},
        {0, 1, 0, 0},
        {0, 0, 1, -height_of_teapot / 2.0},
        {0, 0, 0, 1}
    );
    const float    PI              = 3.1415926535897932384626433832795;
    const float    a_deg           = time_s * 180.0;
    const float    a               = PI * (a_deg / 180.0);
    const float4x4 rotate_matrix   = float4x4_row_major(
        {1,       0,      0, 0},
        {0,  cos(a), sin(a), 0},
        {0, -sin(a), cos(a), 0},
        {0,       0,      0, 1}
    );
    const float4x4 scale_matrix    = float4x4(1);
    const float4x4 model_matrix    = scale_matrix * rotate_matrix * translate_matrix;

    // Place the teapot into world space infront of the camera (assumed to be (0,0,0)).
    const float    distance_from_camera_to_screen = 10.0;
    const float    distance_from_camera           = bounding_radius + distance_from_camera_to_screen;
    const float4x4 view_matrix = float4x4_row_major(
        {1, 0, 0, 0},
        {0, 1, 0, 0},
        {0, 0, 1, distance_from_camera},
        {0, 0, 0, 1}
    );
    const float    r = maxs.x;
    const float    l = mins.x;
    const float    t = maxs.y;
    const float    b = mins.y;
    const float    f = 50.0;
    const float    n = distance_from_camera_to_screen;
    const float4x4 perspective_matrix = float4x4_row_major(
        {n, 0,   0,    0},
        {0, n,   0,    0},
        {0, 0, n+f, -n*f},
        {0, 0,   1,    0}
    );
    const float4x4 orthographic_matrix = float4x4_row_major(
        { 2/(r-l),     0.0,     0.0, -(r+l)/(r-l) },
        {     0.0, 2/(t-b),     0.0, -(t+b)/(t-b) },
        {     0.0,     0.0, 2/(f-n), -(f+n)/(f-n) },
        {     0.0,     0.0,     0.0,          1.0 }
    );
    const float4 position = orthographic_matrix * perspective_matrix * view_matrix * model_matrix * model_position;
    return {
        .position = position,
        .size     = 5.0
    };
}

fragment half4
main_fragment(VertexOut in [[stage_in]])
{
    return half4(1);
};
