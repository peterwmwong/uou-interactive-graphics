#include <metal_stdlib>
#include "./shading.h"
#include "./common.h"

using namespace metal;

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
bg_fragment(         BGVertexOut         in       [[stage_in]],
            constant World             & world    [[buffer(BGFragBufferIndex::World)]],
                     texturecube<half>   texture  [[texture(BGFragTextureIndex::CubeMapTexture)]])
{
    constexpr sampler tx_sampler(mag_filter::linear, address::clamp_to_zero, min_filter::linear);
    const float4 pos   = world.matrix_screen_to_world * float4(in.position.xy, 1, 1);
    const half4  color = texture.sample(tx_sampler, pos.xyz);
    return color;
}

struct VertexOut
{
    float4 position [[position]];
    float3 normal;
    bool   is_mirrored;
};

vertex VertexOut
main_vertex(         uint       vertex_id [[vertex_id]],
                     uint       inst_id   [[instance_id]],
            constant World    & world     [[buffer(VertexBufferIndex::World)]],
            constant Geometry & geometry  [[buffer(VertexBufferIndex::Geometry)]])
{
    const uint idx = geometry.indices[vertex_id];
    const bool is_mirrored = inst_id == MIRRORED_INSTANCE_ID;
    float4 pos = float4(geometry.positions[idx], 1.0);
    float3 normal = world.matrix_normal_to_world * float3(geometry.normals[idx]);
    if (is_mirrored) {
        const float3 pos_world = (world.matrix_model_to_world * pos).xyz + float3(0, -(2.0 * world.plane_y), 0);
        const float3 refl = normalize(float3(pos_world.x, 0.0, pos_world.z));
        pos = world.matrix_world_to_projection * float4(reflect(-pos_world.xyz, refl), 1.0);
        normal = reflect(-normal, refl);
    } else {
        pos = world.matrix_model_to_projection * pos;
    }
    return { .position = pos, .normal = normal, is_mirrored };
}

fragment half4
main_fragment(         VertexOut           in         [[stage_in]],
              constant World             & world      [[buffer(FragBufferIndex::World)]],
                       texturecube<half>   bg_texture [[texture(FragTextureIndex::CubeMapTexture)]])
{
    const half4 color = shade_mirror(in.position, world.camera_position, in.normal, world.matrix_screen_to_world, bg_texture, in.is_mirrored);
    return color;
};

struct PlaneVertexOut
{
    float4 position [[position]];
    float3 normal;
};

vertex PlaneVertexOut
plane_vertex(         uint      vertex_id [[vertex_id]],
                     uint       inst_id   [[instance_id]],
            constant World    & world     [[buffer(VertexBufferIndex::World)]])
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
        .position = world.matrix_world_to_projection * float4(v[0], world.plane_y, v[1], 1.0),
        .normal   = float3(0, 1, 0),
    };
}

fragment half4
plane_fragment(         PlaneVertexOut     in                     [[stage_in]],
              constant World             & world                  [[buffer(FragBufferIndex::World)]],
                       texturecube<half>   bg_texture             [[texture(FragTextureIndex::CubeMapTexture)]],
                       texture2d<half>     mirrored_model_texture [[texture(FragTextureIndex::ModelTexture)]])
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
    half4 mirror_color = mirrored_model_texture.read(uint2(in.position.xy), 0);
    if (mirror_color.a > 0.h) {
        // Super arbitrary, "feels right, probably wrong", darkening the mirrored contents.
        // Maybe what I really want is ambient occlusion on the mirrored plane close to the model...
        const constexpr half4 darken = half4(half3(0.8), 1);
        return mirror_color * darken;
    } else {
        return shade_mirror(in.position, world.camera_position, in.normal, world.matrix_screen_to_world, bg_texture, false);
    }
};


