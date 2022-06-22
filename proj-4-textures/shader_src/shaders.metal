#include <metal_stdlib>
#include "./common.h"

using namespace metal;

constant bool  HasAmbient        [[function_constant(static_cast<uint>(FC::HasAmbient))]];
constant bool  HasDiffuse        [[function_constant(static_cast<uint>(FC::HasDiffuse))]];
constant bool  HasNormal         [[function_constant(static_cast<uint>(FC::HasNormal))]];
constant bool  HasSpecular       [[function_constant(static_cast<uint>(FC::HasSpecular))]];

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
    float2 tx_coord;
};

vertex VertexOut
main_vertex(         uint       vertex_id [[vertex_id]],
            constant Geometry & geometry  [[buffer(VertexBufferIndex::Geometry)]],
            constant World    & world     [[buffer(VertexBufferIndex::World)]])
{
    const uint   idx      = geometry.indices[vertex_id];
    const float4 position = float4(geometry.positions[idx], 1.0);
    const float3 normal   = geometry.normals[idx];
    const float2 tx_coord = geometry.tx_coords[idx];
    return {
        .position  = world.matrix_model_to_projection * position,
        .normal    = world.matrix_normal_to_world * normal,
        // TODO: Should flipping-x be determined by some data in the material?
        .tx_coord  = float2(tx_coord.x, 1. - tx_coord.y)
    };
}

fragment half4
main_fragment(         VertexOut   in       [[stage_in]],
              constant Material  & material [[buffer(FragBufferIndex::Material)]],
              constant World     & world    [[buffer(FragBufferIndex::World)]])
{
    // Calculate the fragment's World Space position from a Metal Viewport Coordinate.
    const float4 pos_w = world.matrix_screen_to_world * float4(in.position.xyz, 1);
    const half3  pos   = half3(pos_w.xyz / pos_w.w);

    /*
    ================================================================
    Rendering Equation: Ambient + Geometry Term (Diffuse + Specular)
    ================================================================

    F(l, c) = Bidirectional Reflectance Distribution Function

    Ambient + Geometry Term (Diffuse    + Specular)
    -------   ------------- ----------   -------------------------------
    Ia Kd   + Il cos(a)     (Kd F(l, c) + (cos(t) Ks F(l, c))^s / cos(a))
    Ia Kd   + Il cos(a)     (Kd         + (cos(t) Ks)^s         / cos(a))
    Ia Kd   + Il l.n        (Kd         + (h.n Ks)^s            / l.n)

    ...distribute the Geometry Term...

    Ambient + Diffuse   + Specular
    -------   ---------   ---------------
    Ia Kd   + Il l.n Kd   + Il (h.n Ks)^s
    */
    const half3 l = normalize(half3(world.light_position.xyz) - pos);  // Light  - world space direction from fragment to light
    const half3 c = normalize(half3(world.camera_position.xyz) - pos); // Camera - world space direction from fragment to camera
    const half3 h = normalize(l + c);                                  // Half   - half-way vector between Light and Camera
    const half3 n = half3(normalize(in.normal));                       // Normal - unit vector, world space direction perpendicular to surface
    if (HasNormal) {
        return half4(n.xy, n.z * -1, 1);
    }
    const half hn = dot(h, n);
    // Cosine angle between Light and Normal
    // - max() to remove Diffuse/Specular when the Light is hitting the back of the surface.
    const half ln = max(dot(l, n), 0.h);

    // Diffuse/Specular Light Intensity of 1.0 for camera facing surfaces, otherwise 0.0.
    // - Use Cosine angle between Camera and Normal (positive <90d, negative >90d)
    // - Using the XCode Shader Profiler, this performed the best compared to...
    //      - ceil(saturate(v))
    //      - trunc(fma(v, .5h, 1.h))
    const half Il = step(0.h, dot(c, n));

    constexpr sampler tx_sampler(mag_filter::linear, address::repeat, min_filter::linear);
    half4 color = 0;
    if (HasSpecular) {
        const half4 Ks = material.specular_texture.sample(tx_sampler, in.tx_coord);
        color += Il * pow(hn * Ks, material.specular_shineness);
    }
    if (HasAmbient) {
        const half4 Ka = material.ambient_texture.sample(tx_sampler, in.tx_coord);
        const half  Ia = 0.1;
        color += Ia * Ka;
    }
    if (HasDiffuse) {
        const half4 Kd = material.diffuse_texture.sample(tx_sampler, in.tx_coord);
        color += Il * ln * Kd;
    }
    return color;
};


struct LightVertexOut {
    float4 position [[position]];
    float  size     [[point_size]];
};

vertex LightVertexOut
light_vertex(constant World & world [[buffer(LightVertexBufferIndex::World)]])
{
    return {
        .position = world.matrix_world_to_projection * world.light_position,
        .size = 50,
    };
}

fragment half4
light_fragment(const float2 point_coord [[point_coord]])
{
    half dist_from_center = length(half2(point_coord) - 0.5h);
    if (dist_from_center > 0.5) discard_fragment();
    return half4(1);
};
