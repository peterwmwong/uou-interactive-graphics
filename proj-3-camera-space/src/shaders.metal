#include <metal_stdlib>
#include "../../metal-types/src/model-space.h"
#include "../../metal-types/src/geometry-no-tx-coords.h"
#include "../../metal-types/src/projected-space.h"
#include "../../metal-types/src/shading-mode.h"
#include "./shader_bindings.h"

using namespace metal;

struct GBufVertex
{
    float4 position [[position]];
    float3 normal;
    float  position_cam_z;
};

[[vertex]]
GBufVertex gbuf_vertex(
             uint                 vertex_id         [[vertex_id]],
    constant GeometryNoTxCoords & geometry          [[buffer(0)]],
    constant ModelSpace2        & model             [[buffer(1)]]
) {
    const uint   idx      = geometry.indices[vertex_id];
    const float4 position = float4(geometry.positions[idx], 1);
    const float4 normal   = float4(geometry.normals[idx], 0);
    return {
        .position       = model.m_model_to_projection * position,
        .position_cam_z = (model.m_model_to_camera * position).z,
        .normal         = normalize((model.m_model_to_camera * normal).xyz),
    };
}

struct GBuf {
    half4  color     [[color(0), raster_order_group(0)]];
    float4 normal    [[color(1), raster_order_group(1)]];
    half   neg_depth [[color(2), raster_order_group(1)]];
};

[[fragment]]
GBuf gbuf_fragment(GBufVertex in [[stage_in]]) {
    return GBuf {
        .normal    = float4(normalize(in.normal), 1),
        .neg_depth = -half(in.position_cam_z)
    };
}

struct LightingVertex {
    float4 position [[position]];
    half2  position_cam;
};

[[vertex]]
LightingVertex lighting_vertex(
             ushort     vertex_id              [[vertex_id]],
    constant float4x4 & m_projection_to_camera [[buffer(0)]]
) {
    float2 position = 0;
    switch (vertex_id) {
        case 0: position = float2(-3,  1); break;
        case 1: position = float2( 1,  1); break;
        case 2: position = float2( 1, -3); break;
    }
    const float2x2 m_xy_projection_to_camera = float2x2(m_projection_to_camera[0].xy, m_projection_to_camera[1].xy);
    return { float4(position, 0, 1), half2(m_xy_projection_to_camera * position) };
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
    const half3 neg_pos_cam  = half3(in.position_cam, 1) * gbuf.neg_depth;

    const float3 n = gbuf.normal.xyz; // Normal - unit vector, world space direction perpendicular to surface
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
    const float3 l  = float3(normalize(half3(light_pos_cam.xyz) + neg_pos_cam)); // Light  - world space direction from fragment to light
    const half3  c  = normalize(neg_pos_cam);                            // Camera - world space direction from fragment to camera
    const float3 h  = normalize(l + float3(c));                  // Half   - half-way vector between Light and Camera
    const float  hn = max(dot(h, n), 0.f);
    const half   ln = half(max(dot(half3(l), half3(n)), 0.h));

    const half   Ia = 0.1;     // Ambient Intensity
    const half   Il = 1. - Ia; // Diffuse/Specular Intensity

    const half3 Kd       = half3(1, 0, 0); // Diffuse Material Color
    const half3 diffuse  = select(
                                0,
                                Il * ln * Kd,
                                HasDiffuse
                            );

    const half3 Ks       = 1;   // Specular Material Color
    const float s        = 200; // Specular Shineness
    const half3 specular = Il * Ks * half(select(
                                0.f,
                                powr(hn, s),
                                HasSpecular
                            ));

    const half3 Ka       = Kd;  // Ambient Material Color
    const half3 ambient  = select(
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