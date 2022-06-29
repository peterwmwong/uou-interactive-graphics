#include <metal_stdlib>
#include "../../proj-6-environment-mapping/shader_src/shading.h"
#include "./common.h"

using namespace metal;

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// Render Light Camera

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
    float2 tx_coord;
};

vertex VertexOut
main_vertex(         uint         vertex_id [[vertex_id]],
            constant ModelSpace & model     [[buffer(VertexBufferIndex::ModelSpace)]],
            constant Geometry   & geometry  [[buffer(VertexBufferIndex::Geometry)]])
{
    const uint idx = geometry.indices[vertex_id];
    return {
        .position = model.matrix_model_to_projection * float4(geometry.positions[idx], 1.0),
        .normal   = model.matrix_normal_to_world     * float3(geometry.normals[idx]),
        .tx_coord = geometry.tx_coords[idx]          * float2(1,-1) + float2(0,1)
    };
}

fragment half4
main_fragment(         VertexOut         in       [[stage_in]],
              constant Space           & camera   [[buffer(FragBufferIndex::CameraSpace)]],
              constant Space           & light    [[buffer(FragBufferIndex::LightSpace)]],
              constant Material        & material [[buffer(FragBufferIndex::Material)]],
                       depth2d<float, access::sample> shadow_tx [[texture(FragTextureIndex::ShadowMap)]])
{
    float4 pos = camera.matrix_screen_to_world * float4(in.position.xyz, 1);
           pos = pos / pos.w;

    float4 pos_light = light.matrix_world_to_projection * pos;
           pos_light = pos_light / pos_light.w;

    constexpr sampler sampler(address::clamp_to_zero, filter::linear, compare_func::less_equal);
    const half not_shadow = select(
        1.h,
        half(shadow_tx.sample_compare(sampler, pos_light.xy, pos_light.z)),
        all(pos_light.xy >= 0.0) && all(pos_light.xy <= 1.0)
    );

    struct TexturedMaterial {
        const    float2     tx_coord;
        const    float      not_shadow;
        constant Material & m;

        inline TexturedMaterial(         float2     tx_coord,
                                         float      not_shadow,
                                constant Material & m):
            tx_coord(tx_coord),
            not_shadow(not_shadow),
            m(m) {}

        inline constexpr const struct sampler s() {
            constexpr struct sampler tx_sampler(mag_filter::linear, address::repeat, min_filter::linear);
            return tx_sampler;
        }

        inline half4 ambient_color()      { return m.ambient_texture.sample(s(), tx_coord); }
        inline half4 diffuse_color()      { return not_shadow > 0.h ? m.diffuse_texture.sample(s(), tx_coord)  : 0; }
        inline half4 specular_color()     { return not_shadow > 0.h ? m.specular_texture.sample(s(), tx_coord) : 0; }
        inline half  specular_shineness() { return m.specular_shineness; }
    };
    return shade_phong_blinn(half3(pos.xyz),
                             half3(light.position_world.xyz),
                             half3(camera.position_world.xyz),
                             half3(normalize(in.normal)),
                             TexturedMaterial(in.tx_coord, not_shadow, material));
};
