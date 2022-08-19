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

[[vertex]]
VertexOut main_vertex(         uint             vertex_id             [[vertex_id]],
                      constant Geometry       & geometry              [[buffer(0)]],
                      constant ProjectedSpace & camera                [[buffer(1)]],
                      constant ModelSpace     & model                 [[buffer(2)]])
{
    const uint   idx    = geometry.indices[vertex_id];
    const float4 pos    = model.m_model_to_projection * float4(geometry.positions[idx], 1.0);
    const float3 normal = model.m_normal_to_world     * float3(geometry.normals[idx]);
    return { .position = pos, .normal = normal };
}

inline float3 get_normal(raytracing::intersection_result<raytracing::triangle_data> intersection, constant MTLPackedFloat4x3 *m_model_to_worlds) {
    const float2   b2     = intersection.triangle_barycentric_coord;
    const float3   b3     = float3(1.0 - (b2.x + b2.y), b2.x, b2.y);
    const auto     n      = (const device packed_half3 *) intersection.primitive_data;
    const float3   n0     = float3(n[0]);
    const float3   n1     = float3(n[1]);
    const float3   n2     = float3(n[2]);
    constant MTLPackedFloat4x3 * m = &m_model_to_worlds[intersection.geometry_id];
    return normalize(
        float3x3((*m)[0], (*m)[1], (*m)[2])
        * ((n0 * b3.x) + (n1 * b3.y) + (n2 * b3.z))
    );
}

[[early_fragment_tests]]
[[fragment]]
half4 main_fragment(         VertexOut                 in                [[stage_in]],
                    constant ProjectedSpace          & camera            [[buffer(0)]],
                    constant float4                  & light_pos         [[buffer(1)]],
                    // The goal is to transform the environment. When rendering the mirrored
                    // world, we need to transformed all the objects of the world, including
                    // the environment (flip the environment texture). Instead of creating a
                    // separate "mirrored" environment texture, we change the sampling
                    // direction achieving the same result.
                    constant MTLPackedFloat4x3       * m_model_to_worlds [[buffer(2)]],
                    primitive_acceleration_structure   accel_struct      [[buffer(3)]],
                    device   DebugRay                * dbg_ray           [[buffer(4)]],
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
    r.min_distance = 0.0;
    r.max_distance = FLT_MAX;
    raytracing::intersector<raytracing::triangle_data> intersector;
    intersector.set_triangle_cull_mode(raytracing::triangle_cull_mode::back);
    intersector.assume_geometry_type(raytracing::geometry_type::triangle);
    auto intersection = intersector.intersect(r, accel_struct);

    // Are we debugging this screen position? (within a half pixel)
    const float2 screen_pos       = in.position.xy;
    const float2 debug_screen_pos = dbg_ray->screen_pos;
    const float2 diff             = abs(debug_screen_pos - screen_pos);
    const bool   is_debug         = !dbg_ray->disabled && all(diff <= float2(0.5));
    // TODO: START HERE
    // TODO: START HERE
    // TODO: START HERE
    // Try to figure out why the primary ray going into teapot spout punches through.
    // - Xcode acceleration structure viewer seems alright (model is okay)
    // - Is this "terminator" problem RT Gems II mentioned?
    //   - Is the intersection happening "below" the geometry in the AS and then secondary/third ray
    //     bounces internally?
    // - Look at the original `normal` does it look right?
    if (is_debug) {
        dbg_ray->points[0]   = float4(camera_pos, 0.0);
        dbg_ray->points[1]   = float4(pos, 1);
        dbg_ray->points[1].w = length(dbg_ray->points[0].xyz - dbg_ray->points[1].xyz);
        dbg_ray->points[2]   = dbg_ray->points[1];
        dbg_ray->points[3]   = dbg_ray->points[1];
        dbg_ray->points[4]   = dbg_ray->points[1];
    }
    if (intersection.type != raytracing::intersection_type::none) {
        // Assumption: Everything in the acceleration structure has the same (mirror) material.
        // intersection
        const float3 normal = get_normal(intersection, m_model_to_worlds);
        if (is_debug) {
            dbg_ray->points[2]   = float4(r.origin + (r.direction * intersection.distance), 1);
            dbg_ray->points[2].w = length(dbg_ray->points[1].xyz - dbg_ray->points[2].xyz);
            dbg_ray->points[3]   = float4(dbg_ray->points[1].xyz + reflect(r.direction, normal), 1);
            dbg_ray->points[3].w = length(dbg_ray->points[2].xyz - dbg_ray->points[3].xyz);
        }

        r.origin    = r.origin + (r.direction * intersection.distance);
        r.direction = reflect(r.direction, normal);
        auto intersection2 = intersector.intersect(r, accel_struct);
        if (is_debug){
            if (intersection2.type != raytracing::intersection_type::none) {
                dbg_ray->points[3]   = float4(r.origin + (r.direction * intersection2.distance), 1);
                dbg_ray->points[3].w = length(dbg_ray->points[2].xyz - dbg_ray->points[3].xyz);
                const float3 normal2 = get_normal(intersection2, m_model_to_worlds);
                dbg_ray->points[4]   = float4(dbg_ray->points[3].xyz + reflect(r.direction, normal2), 1);
                dbg_ray->points[4].w = length(dbg_ray->points[3].xyz - dbg_ray->points[4].xyz);
            } else {
                dbg_ray->points[4] = dbg_ray->points[3];
            }
        }

        if (intersection2.type != raytracing::intersection_type::none) {
            color = half4(0, 0, 1, 1);
        } else {
            color = 0;
        }

        // TODO: START HERE
        // TODO: START HERE
        // TODO: START HERE
        // 1. Update DebugRay to show the incoming camera ray
        // 2. Verify w/DebugRay
        // 3. Add a 2nd bounce ray instersection (if intersection exists)
        // 4. Verify results with w/DebugRay
        // 5. Use 2nd bounce ray intersection for shading
    } else {
        color = env_texture.sample(tx_sampler, float3(ref));
    }
    if (is_debug) {
        return half4(1, 0, 0, 1);
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

[[vertex]]
BGVertexOut bg_vertex(uint vertex_id [[vertex_id]])
{
    constexpr const float2 plane_triange_strip_vertices[3] = {
        {-1.h,  1.h}, // Top    Left
        {-1.h, -3.h}, // Bottom Left
        { 3.h,  1.h}, // Top    Right
    };
    const float2 position2d = plane_triange_strip_vertices[vertex_id];
    return { .position = float4(position2d, 1, 1) };
}

[[fragment]]
half4 bg_fragment(         BGVertexOut         in          [[stage_in]],
                  constant ProjectedSpace    & camera      [[buffer(0)]],
                           texturecube<half>   env_texture [[texture(0)]])
{
    constexpr sampler tx_sampler(mag_filter::linear, address::clamp_to_zero, min_filter::linear);
    const float4 pos   = camera.m_screen_to_world * float4(in.position.xy, 1, 1);
    const half4  color = env_texture.sample(tx_sampler, pos.xyz);
    return color;
}

struct DbgVertexOut {
    float4 position [[position]];
};

[[vertex]]
DbgVertexOut dbg_vertex(         uint             vertex_id [[vertex_id]],
                     constant ProjectedSpace & camera    [[buffer(1)]],
                     constant DebugRay       & dbg_ray   [[buffer(0)]]) {
    if (vertex_id >= 0 && vertex_id <= 4) {
        return { .position = (camera.m_world_to_projection * float4(dbg_ray.points[vertex_id].xyz, 1)) };
    }
    return { .position = 0 };
}

[[fragment]]
half4 dbg_fragment() { return half4(1, 0, 0, 1); }