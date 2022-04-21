#include "./common.h"
#include <metal_stdlib>

using namespace metal;

inline float4x4
float4x4_row_major(const float4 r1, const float4 r2, const float4 r3, const float4 r4) {
    return float4x4(
        /* col1 */ r1[0], r2[0], r3[0], r4[0],
        /* col2 */ r1[1], r2[1], r3[1], r4[1],
        /* col3 */ r1[2], r2[2], r3[2], r4[2],
        /* col4 */ r1[3], r2[3], r3[3], r4[3]
    );
}

inline float4x4
get_orthographic_matrix(const float l, const float r, const float b, const float t, const float n, const float f) {
    return float4x4_row_major(
        { 2/(r-l),       0,       0, -(r+l)/(r-l) },
        {       0, 2/(t-b),       0, -(t+b)/(t-b) },
        // IMPORTANT: Metal's NDC coordinate space has a z range of [0,1], **NOT [-1,1]** (OpenGL).
        {       0,       0, 1/(f-n),     -n/(f-n) },
        {       0,       0,       0,            1 }
    );
}

inline float4x4
get_rotation_matrix(const float2 r) {
    return (
        // x-axis rotation
        float4x4_row_major(
            {       1,         0,        0, 0},
            {       0,  cos(r.x), sin(r.x), 0},
            {       0, -sin(r.x), cos(r.x), 0},
            {       0,         0,        0, 1}
        )
        *
        // y-axis rotation
        float4x4_row_major(
            {cos(r.y),         0, -sin(r.y), 0},
            {       0,         1,         0, 0},
            {sin(r.y),         0,  cos(r.y), 0},
            {       0,         0,         0, 1}
        )
    );
}

struct VertexOut
{
    float4 position [[position]];
    float  size     [[point_size]];
    half4  color;
};

vertex VertexOut
main_vertex(         uint           vertex_id        [[instance_id]],
            constant packed_float4* mins_maxs        [[buffer(VertexBufferIndexMaxPositionValue)]],
            constant packed_float3* positions        [[buffer(VertexBufferIndexPositions)]],
            constant float2&        screen_size      [[buffer(VertexBufferIndexScreenSize)]],
            constant float2&        camera_rotation  [[buffer(VertexBufferIndexCameraRotation)]],
            constant float&         camera_distance  [[buffer(VertexBufferIndexCameraDistance)]],
            constant bool&          use_perspective  [[buffer(VertexBufferIndexUsePerspective)]])
{
    const float4 model_position = float4(positions[vertex_id], 1.0); // Make homogenous coordinate
    const float4 mins           = mins_maxs[0];
    const float4 maxs           = mins_maxs[1];

    // The input model file actually orients the z-axis runs along the "bottom" to the "top" of the
    // teapot.
    const float height_of_teapot = maxs.z - mins.z;
    const float width_of_teapot  = maxs.x - mins.x;

    // From the asset file (teapot.obj), the center bottom of the teapot is the origin (0,0,0).
    // Translate the teapot where the z-coordinate origin is the centered between the top and bottom
    // of the teapot...
    const float4x4 translate_matrix  = float4x4_row_major(
        {1, 0, 0, 0},
        {0, 1, 0, 0},
        {0, 0, 1, -height_of_teapot / 2.0},
        {0, 0, 0, 1}
    );
    const float    PI              = 3.1415926535897932384626433832795;
    // ... and rotate it... so it... you know... sits normally... instead of looking like it toppled over.
    const float4x4 rotate_matrix   = get_rotation_matrix({ PI / 2, 0.0 });
    const float4x4 scale_matrix    = float4x4(1);
    const float4x4 model_matrix    = scale_matrix * rotate_matrix * translate_matrix;

    // Apply the **inverse** of the camera position/rotation.
    const float4x4 view_matrix = float4x4_row_major(
        {1, 0, 0, 0},
        {0, 1, 0, 0},
        {0, 0, 1, camera_distance},
        {0, 0, 0, 1}
    ) * get_rotation_matrix(-camera_rotation);

    const float n                       = 0.1;
    const float f                       = 1000.0;

    // TODO: I'm not sure this is the correct way to maintain the screen aspect ratio.
    // I don't have a "ground truth" image to compare with, but it seems... just sligthly flatten.
    // This is handled ad-hoc with the 0.8 multiplier.
    const float model_aspect_ratio      =       width_of_teapot    / height_of_teapot;
    const float screen_aspect_ratio     =       screen_size.x      / screen_size.y;
    const float aspect_ratio_correction = 0.8 * model_aspect_ratio / screen_aspect_ratio;

    // I think normally you'd just have perspective projection. As such, that would normally be
    // based on FOV (Field Of View degrees: 60, 90, or 120).
    //
    // For this project, we want to easily switch/compare perspective vs orthographic (space bar key).
    // To ensure perspective and orthographic projections are "in-sync", the view volume is
    // calculated based on the bounding bounding box (mins, maxs) of the teapot.
    const float l = mins.x / INITIAL_CAMERA_DISTANCE;
    const float r = maxs.x / INITIAL_CAMERA_DISTANCE;
    const float b = aspect_ratio_correction * mins.y / INITIAL_CAMERA_DISTANCE;
    const float t = aspect_ratio_correction * maxs.y / INITIAL_CAMERA_DISTANCE;

    float4x4 projection_matrix;
    if (use_perspective) {
        const float4x4 perspective_matrix = float4x4_row_major(
            {n, 0,   0,    0},
            {0, n,   0,    0},
            {0, 0, n+f, -n*f},
            {0, 0,   1,    0}
        );
        const float4x4 orthographic_matrix = get_orthographic_matrix(
            l * n,
            r * n,
            b * n,
            t * n,
            n,
            f
        );
        projection_matrix = orthographic_matrix * perspective_matrix;
    } else {
        // As orthographic projection lacks the ability to make farther objects smaller, we simply
        // widen the view volume x/y components by the same proportion the perspective projection
        // (above) would (divide by the w-component, which is the z-component).
        projection_matrix = get_orthographic_matrix(
            l * camera_distance,
            r * camera_distance,
            b * camera_distance,
            t * camera_distance,
            n,
            f
        );
    }

    const float4 view_position       = view_matrix * model_matrix * model_position;
    const float4 projection_position = projection_matrix * view_position;
    return {
        .position = projection_position,
        // Slight differentiation between points close to the camera from those far away.
        .size     = (200 * screen_size.x / 1280.0)
                        / (use_perspective ? projection_position.w : view_position.z),
        // Differentiate perspective (green) from orthographic (red).
        .color    = use_perspective ? half4(0, 1, 0, 1) : half4(1, 0, 0, 1)
    };
}

fragment half4
main_fragment(VertexOut in [[stage_in]])
{
    return in.color;
};