#include <metal_stdlib>
#include "../../metal-shaders/shader_src/shading.h"
#include "./shader_bindings.h"

using namespace metal;

inline half4 shade_mirror(const float4            screen_pos,
                          const float4            camera_pos_f,
                          const float4            light_pos_f,
                          const float3            normal_f,
                          const float4x4          matrix_screen_to_world,
                          // The goal is to transform the environment. When rendering the mirrored
                          // world, we need to transformed all the objects of the world, including
                          // the environment (flip the environment texture). Instead of creating a
                          // separate "mirrored" environment texture, we change the sampling
                          // direction achieving the same result.
                          const float3x3          matrix_env,
                          const texturecube<half> env_texture) {
    // Calculate the fragment's World Space position from a Metal Viewport Coordinate (screen).
    const float4 pos_w      = matrix_screen_to_world * float4(screen_pos.xyz, 1);
    const half3  pos        = half3(pos_w.xyz / pos_w.w);
    const half3  camera_pos = half3(camera_pos_f.xyz);
    const half3  camera_dir = normalize(pos - camera_pos.xyz);
    const half3  normal     = half3(normalize(normal_f));
    const half3  ref        = half3x3(matrix_env) * reflect(camera_dir, normal);

    constexpr sampler tx_sampler(mag_filter::linear, address::clamp_to_zero, min_filter::linear);
    const half4 env_color = env_texture.sample(tx_sampler, float3(ref));
    return shade_phong_blinn(
        {
            .frag_pos     = pos,
            .light_pos    = half3(light_pos_f.xyz),
            .camera_pos   = camera_pos,
            .normal       = normal,
            .has_ambient  = HasAmbient,
            .has_diffuse  = HasDiffuse,
            .has_specular = HasSpecular,
            .only_normals = OnlyNormals,
        },
        ConstantMaterial(1, env_color, env_color, 50, 0.15)
    );
}

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
              constant float3x3          & matrix_env  [[buffer(FragBufferIndex::MatrixEnvironment)]],
                       texturecube<half>   env_texture [[texture(FragTextureIndex::EnvTexture)]])
{
    return shade_mirror(in.position,
                        camera.position_world,
                        light_pos,
                        in.normal,
                        camera.matrix_screen_to_world,
                        matrix_env,
                        env_texture);
};

struct PlaneVertexOut
{
    float4 position [[position]];
    float3 normal;
};

vertex PlaneVertexOut
plane_vertex(         uint             vertex_id [[vertex_id]],
             constant ProjectedSpace & camera    [[buffer(VertexBufferIndex::Camera)]],
             constant float          & plane_y   [[buffer(VertexBufferIndex::PlaneY)]])
{
    // Vertices of Plane laying flat on the ground, along the x/z axis.
    constexpr const float plane_size = 0.9;
    constexpr const float2 verts_xz[4] = {
        {-1, -1}, // Bottom Left
        {-1,  1}, // Top    Left
        { 1, -1}, // Bottom Rigt
        { 1,  1}, // Top    Right
    };
    const float2 v = verts_xz[vertex_id] * plane_size;
    return {
        .position = camera.matrix_world_to_projection * float4(v[0], plane_y, v[1], 1.0),
        .normal   = float3(0, 1, 0),
    };
}

fragment half4
plane_fragment(        PlaneVertexOut            in                    [[stage_in]],
              constant ProjectedSpace          & camera                [[buffer(FragBufferIndex::Camera)]],
              constant float4                  & light_pos             [[buffer(FragBufferIndex::LightPosition)]],
                       texturecube<half>         env_texture           [[texture(FragTextureIndex::EnvTexture)]],
                       texture2d<half,
                                 access::read>  mirrored_model_texture [[texture(FragTextureIndex::ModelTexture)]])
{
    // To render the mirrored model (ex. teapot, sphere, yoda) on the plane, the
    // `mirrored_model_texture` is the model-only contents of the mirror. If the texel doesn't have
    // anything (empty/clear color), render the environment.
    //
    // Alternatively, when rendering the mirrored world (`mirrored_model_texture`), we could also
    // render the environment/skybox and this fragment shader would ONLY read from
    // `mirrored_model_texture`. I suspect in most cases (and this case) this would be slower and
    // alot more work for the GPU. With the mirrored texture only containing the model, all the
    // pixels outside of the model are saved from executing a fragment shader.
    const half4 mirror_color = mirrored_model_texture.read(uint2(in.position.xy), 0);
    if (mirror_color.a > 0.h) {
        // Super arbitrary, "feels right, probably wrong", darkening the mirrored contents.
        // Maybe what I really want is ambient occlusion on the mirrored plane close to the model...
        return mix(mirror_color, half4(0.2), 0.5);
    } else {
        const float3x3 identity = float3x3(1);
        return shade_mirror(in.position, camera.position_world, light_pos, in.normal, camera.matrix_screen_to_world, identity, env_texture);
    }
};


