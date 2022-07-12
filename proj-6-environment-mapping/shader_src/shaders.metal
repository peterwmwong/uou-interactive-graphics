#include <metal_stdlib>
#include "../../metal-shaders/shader_src/shading.h"
#include "./shader_bindings.h"

using namespace metal;

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
};

vertex VertexOut
main_vertex(         uint             vertex_id             [[vertex_id]],
            constant Geometry       & geometry              [[buffer(VertexBufferIndex::Geometry)]],
            constant ProjectedSpace & camera                [[buffer(VertexBufferIndex::Camera)]],
            constant ModelSpace     & model                 [[buffer(VertexBufferIndex::Model)]])
{
    const uint idx      = geometry.indices[vertex_id];
    const float4 pos    = model.matrix_model_to_projection * float4(geometry.positions[idx], 1.0);
    const float3 normal = model.matrix_normal_to_world     * float3(geometry.normals[idx]);
    return { .position = pos, .normal = normal };
}

fragment half4
main_fragment(         VertexOut           in          [[stage_in]],
              constant ProjectedSpace    & camera      [[buffer(FragBufferIndex::Camera)]],
              constant float4            & light_pos   [[buffer(FragBufferIndex::LightPosition)]],
              // The goal is to transform the environment. When rendering the mirrored
              // world, we need to transformed all the objects of the world, including
              // the environment (flip the environment texture). Instead of creating a
              // separate "mirrored" environment texture, we change the sampling
              // direction achieving the same result.
              constant float3x3          & matrix_env  [[buffer(FragBufferIndex::MatrixEnvironment)]],
              constant float             & darken      [[buffer(FragBufferIndex::Darken)]],
                       texturecube<half>   env_texture [[texture(FragTextureIndex::EnvTexture)]])
{
    // Calculate the fragment's World Space position from a Metal Viewport Coordinate (screen).
    const float4 pos_w      = camera.matrix_screen_to_world * float4(in.position.xyz, 1);
    const half3  pos        = half3(pos_w.xyz / pos_w.w);
    const half3  camera_pos = half3(camera.position_world.xyz);
    const half3  camera_dir = normalize(pos - camera_pos.xyz);
    const half3  normal     = half3(normalize(in.normal));
    const half3  ref        = half3x3(matrix_env) * reflect(camera_dir, normal);

    // Performance: Reduce texture sampling (and overall shader time), by only sampling when needed.
    // If `shade_phong_blinn()` determines there is not enough light intensity (ex. fragments that
    // are back facing or facing away from light), none of the material methods will be called.
    struct CachedEnvTextureMaterial {
        const texturecube<half> t;
        const float3            tx_coord;
              half4             sampled;

        inline CachedEnvTextureMaterial(const texturecube<half> t,
                                 const float3            tx_coord):
            t(t),
            tx_coord(tx_coord),
            sampled(half4(-1)){}

        inline constexpr const struct sampler s() const {
            constexpr struct sampler tx_sampler(mag_filter::linear, address::clamp_to_zero, min_filter::linear);
            return tx_sampler;
        }

        inline half4 color() {
            if (sampled.r < 0) {
                sampled = t.sample(s(), tx_coord);
            }
            return sampled;
        }
        inline half4 ambient_color()            { return color(); }
        inline half4 diffuse_color()            { return color(); }
        inline half4 specular_color()           { return color(); }
        inline half  specular_shineness() const { return 50;      }
        inline half  ambient_amount() const     { return 0.15;    }
    };

    return mix(
        shade_phong_blinn(
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
            CachedEnvTextureMaterial(env_texture, float3(ref))
        ),
        // Super arbitrary, "feels right, probably wrong", darkening the mirrored model.
        // Maybe what I really want is ambient occlusion on the mirrored plane close to the model...
        half4(0.2),
        darken
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
            constant ProjectedSpace    & camera      [[buffer(FragBufferIndex::Camera)]],
                     texturecube<half>   env_texture [[texture(FragTextureIndex::EnvTexture)]])
{
    constexpr sampler tx_sampler(mag_filter::linear, address::clamp_to_zero, min_filter::linear);
    const float4 pos   = camera.matrix_screen_to_world * float4(in.position.xy, 1, 1);
    const half4  color = env_texture.sample(tx_sampler, pos.xyz);
    return color;
}
