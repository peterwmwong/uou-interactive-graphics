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
    // TOOD: Add normal
};

// IMPORTANT: Normally you would **NOT** calculate the model-view-projection matrix in the Vertex
// Shader. For performance, this should be done once (not for every vertex) on the CPU and passed to
// the Vertex Shader as a constant space buffer. It is done in the Vertex Shader for this project as
// a personal excercise to become more familar with the Metal Shading Language.
vertex VertexOut
main_vertex(         uint           inst_id          [[instance_id]],
                     uint           vertex_id        [[vertex_id]],
            constant uint*          indices          [[buffer(VertexBufferIndexIndices)]],
            constant packed_float3* positions        [[buffer(VertexBufferIndexPositions)]],
            constant float4x4&      mvp              [[buffer(VertexBufferIndexModelViewProjection)]],
            constant float2&        screen_size      [[buffer(VertexBufferIndexScreenSize)]],
            constant float2&        camera_rotation  [[buffer(VertexBufferIndexCameraRotation)]],
            constant float&         camera_distance  [[buffer(VertexBufferIndexCameraDistance)]])
{
    const uint   position_idx   = indices[inst_id * 3 + vertex_id];
    const float4 model_position = float4(positions[position_idx], 1.0); // Make homogenous coordinate
    const float4x4 yolo = mvp;
    const float4 mins = float4(yolo[0]);
    const float4 maxs = float4(yolo[1]);

    // The input model file actually orients the z-axis runs along the "bottom" to the "top" of the
    // teapot and the center bottom of the teapot is the origin (0,0,0).
    // Translate the teapot where the z-coordinate origin is the centered between the top and bottom
    // of the teapot...
    const float height_of_teapot    = maxs.z - mins.z;
    const float4x4 translate_matrix = float4x4_row_major(
        {1, 0, 0, 0},
        {0, 1, 0, 0},
        {0, 0, 1, -height_of_teapot / 2.0},
        {0, 0, 0, 1}
    );
    // ... and rotate it... so it... you know... sits normally... instead of looking like it toppled
    // over, spilling tea all over the place...
    const float    PI              = 3.1415926535897932384626433832795;
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

    // I think normally you'd just use perspective projection. As such, that would normally be
    // based on FOV (Field Of View degrees: 60, 90, or 120).
    //
    // For this project, we want to easily switch/compare perspective vs orthographic ("P" key).
    // To ensure perspective and orthographic projections are "in-sync", the view volume is
    // calculated based on the bounding bounding box (mins, maxs) of the teapot.
    const float n                   = 0.1;
    const float f                   = 1000.0;
    const float screen_aspect_ratio = screen_size.x / screen_size.y;
    const float max_bound =
        max(
            max(
                max(abs(mins.x), abs(maxs.x)),
                max(abs(mins.y), abs(maxs.y))
            ),
            max(abs(mins.z), abs(maxs.z))
        )
        + 2.0 /* screen padding */;
    const float l = screen_aspect_ratio * -max_bound / INITIAL_CAMERA_DISTANCE;
    const float r = screen_aspect_ratio *  max_bound / INITIAL_CAMERA_DISTANCE;
    const float b =                       -max_bound / INITIAL_CAMERA_DISTANCE;
    const float t =                        max_bound / INITIAL_CAMERA_DISTANCE;
    float4x4 projection_matrix;
    const float4x4 perspective_matrix = float4x4_row_major(
        {n, 0,   0,    0},
        {0, n,   0,    0},
        {0, 0, n+f, -n*f},
        {0, 0,   1,    0}
    );
    // (n^2 + fn - fn) / n
    // (n   + f  - f)  / n
    const float4x4 orthographic_matrix = get_orthographic_matrix(
        l * n,
        r * n,
        b * n,
        t * n,
        n,
        f
    );
    projection_matrix = orthographic_matrix * perspective_matrix;

    const float4 view_position       = view_matrix * model_matrix * model_position;
    const float4 projection_position = projection_matrix * view_position;
    return {
        .position = projection_position
    };
}

fragment half4
main_fragment(VertexOut in [[stage_in]])
{
    return half4(0, 1, 0, 1);
};
