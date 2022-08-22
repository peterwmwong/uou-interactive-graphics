#include <metal_stdlib>
#include "../../metal-shaders/shader_src/shading.h"
#include "../../metal-types/src/debug-path.h"
#include "../../metal-types/src/geometry-no-tx-coords.h"
#include "../../metal-types/src/macros.h"
#include "../../metal-types/src/model-space.h"
#include "../../metal-types/src/projected-space.h"
#include "../../metal-types/src/shading-mode.h"
#include "../../metal-types/src/tri_normals_index.h"

using namespace metal;
using namespace raytracing;

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
};

[[vertex]]
VertexOut main_vertex(         uint                 vertex_id [[vertex_id]],
                      constant GeometryNoTxCoords & geometry  [[buffer(0)]],
                      constant ProjectedSpace     & camera    [[buffer(1)]],
                      constant ModelSpace         & model     [[buffer(2)]])
{
    const uint   idx    = geometry.indices[vertex_id];
    const float4 pos    = model.m_model_to_projection * float4(geometry.positions[idx], 1.0);
    const float3 normal = model.m_normal_to_world     * float3(geometry.normals[idx]);
    return { .position = pos, .normal = normal };
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
                    device   DebugPath               & dbg_path          [[buffer(4)]],
                             texturecube<half>         env_texture       [[texture(0)]])
{
    // Calculate the fragment's World Space position from a Metal Viewport Coordinate (screen).
    const float4 pos_w      = camera.m_screen_to_world * float4(in.position.xyz, 1);
    const half3  pos        = half3(pos_w.xyz / pos_w.w);
    const half3  camera_pos = half3(camera.position_world.xyz);
    const half3  camera_dir = normalize(pos - camera_pos);
    const half3  normal     = half3(normalize(in.normal));

    // Are we debugging this screen position? (within a half pixel)
    DebugPathHelper dbg;
    if (UpdateDebugPath) {
        dbg = dbg_path.activate_if_screen_pos(in.position.xy);
        dbg.add_point(camera_pos);
        dbg.add_point(pos);
    }

    half3 r_origin = pos;
    half3 r_dir    = reflect(camera_dir, normal);
    intersector<triangle_data> inter;
    inter.set_triangle_cull_mode(triangle_cull_mode::back);
    inter.assume_geometry_type(geometry_type::triangle);
    for (uint bounce = 4; bounce > 0; bounce--) {
        const auto hit = inter.intersect(ray(float3(r_origin), float3(r_dir)), accel_struct);
        if (hit.type != intersection_type::none) {
            // Assumption: Everything in the acceleration structure has the same (mirror) material.
            // intersection.
            r_origin = r_origin + (r_dir * half(hit.distance));
            const auto p = (device TriNormals *) hit.primitive_data;
            r_dir = reflect(r_dir, p->normal(hit.triangle_barycentric_coord, &m_model_to_worlds[hit.geometry_id]));
            if (UpdateDebugPath) dbg.add_point(r_origin);
        } else {
            break;
        }
    }
    if (UpdateDebugPath) dbg.add_relative_point(r_dir);

    constexpr sampler tx_sampler(mag_filter::linear, address::clamp_to_zero, min_filter::linear);
    const half4 color = env_texture.sample(tx_sampler, float3(r_dir));

    // Render a single red pixel, so this pixel can easily be targetted for the shader debugger in
    // the GPU Frame Capture.
    if (UpdateDebugPath && dbg.active) {
        return half4(1, 0, 0, 1);
    }
    return shade_phong_blinn(
        {
            .frag_pos     = pos,
            .light_pos    = half3(light_pos.xyz),
            .camera_pos   = camera_pos,
            .normal       = normal,
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

[[vertex]]
float4 dbg_vertex(         uint             vertex_id [[vertex_id]],
                  constant DebugPath      & dbg_path  [[buffer(0)]],
                  constant ProjectedSpace & camera    [[buffer(1)]]) {
    const uint vid = clamp(vertex_id, (uint) 0, (uint) (dbg_path.num_points - 1));
    return (camera.m_world_to_projection * float4(dbg_path.points[vid], 1));
}

[[fragment]]
half4 dbg_fragment() { return half4(1, 0, 0, 1); }
