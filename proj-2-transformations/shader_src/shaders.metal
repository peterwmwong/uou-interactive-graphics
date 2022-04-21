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
    return float4x4_row_major(
        {       1,         0,        0, 0},
        {       0,  cos(r.x), sin(r.x), 0},
        {       0, -sin(r.x), cos(r.x), 0},
        {       0,         0,        0, 1}
    ) * float4x4_row_major(
        {cos(r.y),         0, -sin(r.y), 0},
        {       0,         1,         0, 0},
        {sin(r.y),         0,  cos(r.y), 0},
        {       0,         0,         0, 1}
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
            constant float&         camera_distance  [[buffer(VertexBufferIndexCameraDistance)]])
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
    // of the teapot.
    const float4x4 translate_matrix  = float4x4_row_major(
        {1, 0, 0, 0},
        {0, 1, 0, 0},
        {0, 0, 1, -height_of_teapot / 2.0},
        {0, 0, 0, 1}
    );
    const float    PI              = 3.1415926535897932384626433832795;
    const float4x4 rotate_matrix   = get_rotation_matrix({ PI / 2, 0.0 });
    const float4x4 scale_matrix    = float4x4(1);
    const float4x4 model_matrix    = scale_matrix * rotate_matrix * translate_matrix;

    // Apply the **inverse** of the camera position/rotation.
    const float4x4 view_matrix = float4x4_row_major(
        {1, 0, 0, 0},
        {0, 1, 0, 0},
        {0, 0, 1, -camera_distance},
        {0, 0, 0, 1}
    ) * get_rotation_matrix(-camera_rotation);

    // TODO: Make this toggleable (keyDown?)
    const bool  use_perspective         = true;
    const float n                       = 0.1;
    const float f                       = 1000.0;
    const float model_aspect_ratio      = width_of_teapot    / height_of_teapot;
    const float screen_aspect_ratio     = screen_size.x      / screen_size.y;
    const float aspect_ratio_correction = model_aspect_ratio / screen_aspect_ratio;
    float4x4 projection_matrix;
    if (use_perspective) {

        // TODO: This isn't exactly correct, even for the teapot.
        // - "50.0" is based on the initial camera position.
        // - Currently under estimating as given a rotation (~135 deg), the tip of the spout goes
        //   offscreen
        // - To keep the keep the teapot's spout in view, we need to find the actual z-coordinate
        //   where the tip of the spout (max y) actually resides.
        const float assumed_z_when_xy_are_at_max = 50.0;
        const float r = (maxs.x * n) / assumed_z_when_xy_are_at_max;
        const float l = (mins.x * n) / assumed_z_when_xy_are_at_max;
        const float t = aspect_ratio_correction * (maxs.y * n) / assumed_z_when_xy_are_at_max;
        const float b = aspect_ratio_correction * (mins.y * n) / assumed_z_when_xy_are_at_max;
        const float4x4 perspective_matrix = float4x4_row_major(
            {n, 0,   0,    0},
            {0, n,   0,    0},
            {0, 0, n+f, -n*f},
            {0, 0,   1,    0}
        );
        const float4x4 orthographic_matrix = get_orthographic_matrix(l, r, b, t, n, f);
        projection_matrix = orthographic_matrix * perspective_matrix;
    } else {
        // TODO: Zooming doesn't work in orthographic.
        // - Camera distance should somehow be factored into these bounding box calculations.
        projection_matrix = get_orthographic_matrix(
            mins.x, maxs.x,
            mins.y * aspect_ratio_correction,
            maxs.y * aspect_ratio_correction,
            n, f
        );
    }

    const float4 position = projection_matrix * view_matrix * model_matrix * model_position;
    return {
        .position = position,
        // Give a slight visual difference between points close vs far away
        .size     = (screen_size.x * 200.0 / 1280.0) / position.w,
        .color    = use_perspective ? half4(0, 1, 0, 1) : half4(1, 0, 0, 1)
    };
}

fragment half4
main_fragment(VertexOut in [[stage_in]])
{
    return in.color;
};
