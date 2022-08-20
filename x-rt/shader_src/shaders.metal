#include <metal_stdlib>
#include "./shader_bindings.h"
#include "../../metal-types/src/tri_normals_index.h"

using namespace metal;
using namespace raytracing;

struct VertexOut
{
    float4 position [[position]];
};

[[vertex]]
VertexOut main_vertex(
    uint vertex_id [[vertex_id]]
) {
    return  (vertex_id == 0) ? VertexOut { .position = float4(-3,  1, 0, 1) }
          : (vertex_id == 1) ? VertexOut { .position = float4( 1,  1, 0, 1) }
          : (vertex_id == 2) ? VertexOut { .position = float4( 1, -3, 0, 1) }
          : VertexOut { .position = float4(0) };
}

[[fragment]]
half4 main_fragment(
             VertexOut                         in                    [[stage_in]],
             primitive_acceleration_structure  accelerationStructure [[buffer(0)]],
    constant ProjectedSpace                  & camera                [[buffer(1)]],
    constant MTLPackedFloat4x3               * m_model_to_worlds     [[buffer(4)]],
    constant float4                          & camera_pos            [[buffer(3)]]
) {
    const float4 pos_w       = camera.m_screen_to_world * float4(in.position.xyz, 1);
    const float3 pos         = pos_w.xyz / pos_w.w;
    const ray    r(camera_pos.xyz, normalize(pos - camera_pos.xyz));

    intersector<triangle_data> intersector;
    intersector.set_triangle_cull_mode(raytracing::triangle_cull_mode::back);
    intersector.assume_geometry_type(geometry_type::triangle);
    auto intersection = intersector.intersect(r, accelerationStructure);
    if (intersection.type == intersection_type::triangle) {
        const auto    p      = (const device TriNormalsIndex *) intersection.primitive_data;
        const auto    m      = &(m_model_to_worlds[p->index]);
        const half2   b2     = half2(intersection.triangle_barycentric_coord);
        const half3   b      = half3(1.0 - (b2.x + b2.y), b2.x, b2.y);
        const auto    n      = p->normals;
        const half3   normal = (n[0] * b.x) + (n[1] * b.y) + (n[2] * b.z);
        return half4(
            // IMPORTANT: Converting to float before normalize may seem redundant, but for models
            // like yoda, small half precision normals seems to cause normalize to go bonkers.
            half3(normalize(float3(
                half3x3(half3((*m)[0]), half3((*m)[1]), half3((*m)[2]))
                * normal
            ))),
            1
        );
    }
    return 0;
}