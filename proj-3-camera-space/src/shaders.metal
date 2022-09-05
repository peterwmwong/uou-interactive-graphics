#include <metal_stdlib>
#include "../../metal-types/src/model-space.h"
#include "../../metal-types/src/geometry-no-tx-coords.h"
#include "../../metal-types/src/projected-space.h"
#include "../../metal-types/src/shading-mode.h"

using namespace metal;

struct GBufVertex
{
    float4 position [[position]];
    float3 position_in_camera_space;
    float3 normal;
};

[[vertex]]
GBufVertex gbuf_vertex(
             uint                 vertex_id         [[vertex_id]],
    constant GeometryNoTxCoords & geometry          [[buffer(0)]],
    constant ModelSpace         & model             [[buffer(1)]],
    constant float4x4           & m_model_to_world  [[buffer(2)]],
    constant float4x4           & m_world_to_camera [[buffer(3)]]
) {
    const uint   idx      = geometry.indices[vertex_id];
    const float4 position = float4(geometry.positions[idx], 1.0);
    const float3 normal   = geometry.normals[idx];
    // TODO: START HERE
    // TODO: START HERE
    // TODO: START HERE
    // 1. Precalculate as much as possible (matrices)
    // 2. Can we reduce the position_in_camera_space down to just z?
    //    a. GBufVertex storage
    //    b. Calculation, we should be able to just send a vec3 or vec4 right? not a whole matrix
    const float4 position_cam = m_world_to_camera * m_model_to_world * position;
    const float3x3 normal_to_camera = float3x3(m_world_to_camera[0].xyz, m_world_to_camera[1].xyz, m_world_to_camera[2].xyz);
    return {
        .position                 = model.m_model_to_projection * position,
        .position_in_camera_space = position_cam.xyz,
        .normal                   = normalize(normal_to_camera * model.m_normal_to_world * normal),
    };
}

struct GBuf {
    half4 color     [[color(0), raster_order_group(0)]];
    half4 normal    [[color(1), raster_order_group(1)]];
    float neg_depth [[color(2), raster_order_group(1)]];
};

[[fragment]]
GBuf gbuf_fragment(GBufVertex in [[stage_in]]) {
    return GBuf {
        .normal = half4(half3(normalize(in.normal)), 1.),
        .neg_depth  = -in.position_in_camera_space.z
    };
}

struct LightingVertex {
    float4 position [[position]];
    float2 position_in_camera_space;
};

[[vertex]]
LightingVertex lighting_vertex(
             uint       vertex_id              [[vertex_id]],
    constant float4x4 & m_projection_to_camera [[buffer(0)]]
) {
    float4 position = 0;

    // TODO: START HERE 2
    // TODO: START HERE 2
    // TODO: START HERE 2
    // Can we do a single giant triangle?
    switch (vertex_id) {
        case 0: position = float4(-1,  1, 0, 1); break;
        case 1: position = float4( 1,  1, 0, 1); break;
        case 2: position = float4(-1, -1, 0, 1); break;
        case 3: position = float4( 1, -1, 0, 1); break;
    }

    // TODO: START HERE 3
    // TODO: START HERE 3
    // TODO: START HERE 3
    // Since we're calculating just for the xy, is there a reduced 3x3 matrix (or even smaller)?
    const float4 position_in_camera_space = m_projection_to_camera * position;
    return { position, position_in_camera_space.xy / position_in_camera_space.z };
}

struct LightingFragment {
    half4 color [[color(0), raster_order_group(0)]];
};

[[fragment]]
LightingFragment lighting_fragment(
             LightingVertex   in            [[stage_in]],
    constant ProjectedSpace & camera        [[buffer(0)]],
    constant float3         & light_pos_cam [[buffer(1)]],
             GBuf             gbuf
) {
    const float  neg_depth = float(gbuf.neg_depth);
    const float3 pos_cam   = float3(in.position_in_camera_space * neg_depth, neg_depth);

    const float3 n = normalize(float3(gbuf.normal.xyz)); // Normal - unit vector, world space direction perpendicular to surface
    if (OnlyNormals) return { half4(half3(n), 1) };

    /*
    ================================================================
    Rendering Equation: Ambient + Geometry Term (Diffuse + Specular)
    ================================================================

    F(l, c) = Bidirectional Reflectance Distribution Function

    Ambient + Geometry Term (Diffuse    + Specular)
    -------   ------------- ----------   -------------------------------
    Ia Kd   + Il cos(a)     (Kd F(l, c) + Ks (cos(t) F(l, c))^s / cos(a))
    Ia Kd   + Il cos(a)     (Kd         + Ks cos(t)^s           / cos(a))
    Ia Kd   + Il (l·n)      (Kd         + Ks (h·n)^s            / (l·n))

    ...distribute the Geometry Term...

    Ambient + Diffuse     + Specular
    -------   -----------   -------------
    Ia Kd   + Il (l·n) Kd + Il Ks (h·n)^s
    */
    const float3 l  = normalize(light_pos_cam.xyz + pos_cam); // Light  - world space direction from fragment to light
    const float3 c  = normalize(pos_cam);                     // Camera - world space direction from fragment to camera
    const float3 h  = normalize(l + c);                       // Half   - half-way vector between Light and Camera

    const float hn = max(dot(h, n), 0.f);
    // Cosine angle between Light and Normal
    // - max() to remove Diffuse/Specular when the Light is hitting the back of the surface.
    const float ln = max(dot(l, n), 0.f);
    // - step() to remove Diffuse/Specular when the Camera is viewing the back of the surface
    // - Using the XCode Shader Profiler, this performed the best compared to...
    //      - ceil(saturate(v))
    //      - trunc(fma(v, .5h, 1.h))
    const float Il = 1;


    const float3 Kd       = float3(1, 0, 0); // Diffuse Material Color
    const float3 diffuse  = select(
                                0,
                                Il * ln * Kd,
                                HasDiffuse
                            );

    const float3 Ks       = 1;   // Specular Material Color
    const float  s        = 200; // Specular Shineness
    const float3 specular = select(
                                0,
                                Il * Ks * powr(hn, s),
                                HasSpecular
                            );

    const float  Ia       = 0.1; // Ambient Intensity
    const float3 Ka       = Kd;  // Ambient Material Color
    const float3 ambient  = select(
                                0,
                                Ia * Ka,
                                HasAmbient
                            );

    return { half4(half3(ambient + diffuse + specular), 1.0h) };
};

struct LightVertexOut {
    float4 position [[position]];
    float  size     [[point_size]];
};

[[vertex]]
LightVertexOut light_vertex(constant ProjectedSpace & camera    [[buffer(0)]],
                            constant float4         & light_pos [[buffer(1)]]) {
    return {
        .position = camera.m_world_to_projection * light_pos,
        .size = 50,
    };
}

[[fragment]]
half4 light_fragment(const float2 point_coord [[point_coord]]) {
    float dist_from_center = length(point_coord - 0.5);
    if (dist_from_center > 0.5) discard_fragment();
    return 1;
};