#include <metal_stdlib>
#include "../../metal-shaders/shader_src/shading.h"
#include "./shader_bindings.h"

using namespace metal;
using raytracing::primitive_acceleration_structure;

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
};

vertex VertexOut
main_vertex(         uint             vertex_id             [[vertex_id]],
            constant Geometry       & geometry              [[buffer(0)]],
            constant ProjectedSpace & camera                [[buffer(1)]],
            constant ModelSpace     & model                 [[buffer(2)]])
{
    const uint   idx    = geometry.indices[vertex_id];
    const float4 pos    = model.m_model_to_projection * float4(geometry.positions[idx], 1.0);
    const float3 normal = model.m_normal_to_world     * float3(geometry.normals[idx]);
    return { .position = pos, .normal = normal };
}

fragment half4
main_fragment(         VertexOut                 in                [[stage_in]],
              constant ProjectedSpace          & camera            [[buffer(0)]],
              constant float4                  & light_pos         [[buffer(1)]],
              // The goal is to transform the environment. When rendering the mirrored
              // world, we need to transformed all the objects of the world, including
              // the environment (flip the environment texture). Instead of creating a
              // separate "mirrored" environment texture, we change the sampling
              // direction achieving the same result.
              constant float4x3                * m_model_to_worlds [[buffer(2)]],
              primitive_acceleration_structure   accel_struct      [[buffer(3)]],
                       texturecube<half>         env_texture       [[texture(0)]])
{
    // Calculate the fragment's World Space position from a Metal Viewport Coordinate (screen).
    const float4 pos_w      = camera.m_screen_to_world * float4(in.position.xyz, 1);
    const float3 pos        = pos_w.xyz / pos_w.w;
    const float3 camera_pos = camera.position_world.xyz;
    const float3 camera_dir = normalize(pos - camera_pos.xyz);
    const float3 normal     = normalize(in.normal);
    const float3 ref        = reflect(camera_dir, normal);

    constexpr sampler tx_sampler(mag_filter::linear, address::clamp_to_zero, min_filter::linear);
    half4 color;

    raytracing::ray r;
    r.origin       = pos;
    r.direction    = ref;
    r.min_distance = 0.001;
    r.max_distance = FLT_MAX;
    raytracing::intersector<raytracing::triangle_data> intersector;
    intersector.set_triangle_cull_mode(raytracing::triangle_cull_mode::back);
    intersector.assume_geometry_type(raytracing::geometry_type::triangle);
    auto intersection = intersector.intersect(r, accel_struct);
    if (intersection.type != raytracing::intersection_type::none) {
        // Assumption: Everything in the acceleration structure has the same (mirror) material.
        // intersection

        const float2   b2     = intersection.triangle_barycentric_coord;
        const float3   b3     = float3(1.0 - (b2.x + b2.y), b2.x, b2.y);
        const auto     n      = (const device packed_half3 *) intersection.primitive_data;
        const float3   n0     = float3(n[0]);
        const float3   n1     = float3(n[1]);
        const float3   n2     = float3(n[2]);
        const float4x3 m      = m_model_to_worlds[intersection.geometry_id];
        const float3   normal = normalize(
            float3x3(m[0].xyz, m[1].xyz, m[2].xyz) * ((n0 * b3.x) + (n1 * b3.y) + (n2 * b3.z))
        );

        // Experiment 1: Generate new ray (reflect the reflection ray)
        //
        // r.origin = r.origin + r.direction * intersection.distance;
        // r.direction = reflect(r.direction, normal);
        // auto intersection2 = intersector.intersect(r, accel_struct);
        // if (intersection2.type != raytracing::intersection_type::none) {
        //     return half4(0, 1, 0, 1);
        // }
        // color = env_texture.sample(tx_sampler, float3(reflect(r.direction, normal)));

        // TODO: START HERE
        // TODO: START HERE
        // TODO: START HERE
        // Looking at the reflection of the top lid of the teapot, the visualized normal looks
        // wrong! It's blue all around (rotating the teapot along the y-axis). The normal being
        // reflected should be red (positive x component) when seeing the reflection on the right
        // side.
        color = half4(half3(normal), 1);
    } else {
        color = env_texture.sample(tx_sampler, float3(ref));
    }
    return shade_phong_blinn(
        {
            .frag_pos     = half3(pos),
            .light_pos    = half3(light_pos.xyz),
            .camera_pos   = half3(camera_pos),
            .normal       = half3(normal),
            .has_ambient  = HasAmbient,
            .has_diffuse  = HasDiffuse,
            .has_specular = HasSpecular,
            .only_normals = OnlyNormals,
        },
        ConstantMaterial(color, color, color, 50, 0.5)
    );
};

struct BGVertexOut {
    float4 position [[position]];
};

vertex BGVertexOut
bg_vertex(uint vertex_id [[vertex_id]])
{
    constexpr const float2 plane_triange_strip_vertices[3] = {
        {-1.h,  1.h}, // Top    Left
        {-1.h, -3.h}, // Bottom Left
        { 3.h,  1.h}, // Top    Right
    };
    const float2 position2d = plane_triange_strip_vertices[vertex_id];
    return { .position = float4(position2d, 1, 1) };
}

fragment half4
bg_fragment(         BGVertexOut         in          [[stage_in]],
            constant ProjectedSpace    & camera      [[buffer(0)]],
                     texturecube<half>   env_texture [[texture(0)]])
{
    constexpr sampler tx_sampler(mag_filter::linear, address::clamp_to_zero, min_filter::linear);
    const float4 pos   = camera.m_screen_to_world * float4(in.position.xy, 1, 1);
    const half4  color = env_texture.sample(tx_sampler, pos.xyz);
    return color;
}
