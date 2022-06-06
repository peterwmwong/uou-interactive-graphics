#include <metal_stdlib>
#include "./common.h"

using namespace metal;

constant bool  HAS_AMBIENT        [[function_constant(static_cast<uint>(FC::HAS_AMBIENT))]];
constant bool  HAS_DIFFUSE        [[function_constant(static_cast<uint>(FC::HAS_DIFFUSE))]];
constant bool  HAS_NORMAL         [[function_constant(static_cast<uint>(FC::HAS_NORMAL))]];
constant bool  HAS_SPECULAR       [[function_constant(static_cast<uint>(FC::HAS_SPECULAR))]];

struct World {
    float4x4 matrix_model_to_projection [[id(WorldID::matrix_model_to_projection)]];
    float4x4 matrix_world_to_projection [[id(WorldID::matrix_world_to_projection)]];
    float3x3 matrix_normal_to_world     [[id(WorldID::matrix_normal_to_world)]];
    float4x4 matrix_screen_to_world     [[id(WorldID::matrix_screen_to_world)]];

    float4   light_position             [[id(WorldID::light_position)]];
    float4   camera_position            [[id(WorldID::camera_position)]];
};

// TODO: Re-layout memory to be more cache-friendly
// - Currently, with 3 separate arrays for position, normals, and tx_coords, a VS instance must
//   access 3 separate, disjoint memory addresses
// - What if we pack position, normal, and tx_coord as tuple and then have an array of tuples
//   struct VertexData { position; normal; coord; }
//   struct Geometry { const VertexData * vertex_data; }
// - Downside: Data Duplication
//   - Assess size difference
struct Geometry {
    constant uint          * indices   [[id(ObjectGeometryID::indices)]];
    constant packed_float3 * positions [[id(ObjectGeometryID::positions)]];
    constant packed_float3 * normals   [[id(ObjectGeometryID::normals)]];
    constant packed_float2 * tx_coords [[id(ObjectGeometryID::tx_coords)]];
};

struct VertexOut
{
    float4 position [[position]];
    half3  normal;
    half2  tx_coord;
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
        .normal    = half3(normalize(world.matrix_normal_to_world * normal)),
        // TODO: Should flipping-x be determined by some data in the material?
        .tx_coord  = half2(tx_coord.x, 1.0 - tx_coord.y)
    };
}

struct Material {
    float4          diffuse_color      [[id(MaterialID::diffuse_color)]];
    float4          specular_color     [[id(MaterialID::specular_color)]];
    texture2d<half> diffuse_texture    [[id(MaterialID::diffuse_texture)]];
    texture2d<half> specular_texture   [[id(MaterialID::specular_texture)]];
    float           specular_shineness [[id(MaterialID::specular_shineness)]];
};

fragment half4
main_fragment(         VertexOut   in       [[stage_in]],
              constant Material  & material [[buffer(FragBufferIndex::Material)]],
              constant World     & world    [[buffer(FragBufferIndex::World)]])
{
    const half3 n = normalize(in.normal); // Normal - unit vector, world space direction perpendicular to surface
    if (HAS_NORMAL) {
        return half4(n.xy, n.z * -1, 1);
    }

    // Calculate the fragment's World Space position from a Metal Viewport Coordinate.
    const float4 pos_w_persp = world.matrix_screen_to_world * float4(in.position.xyz, 1);
    const half3  pos         = half3(pos_w_persp.xyz / pos_w_persp.w);

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

    // Cosine angle between Light and Normal
    // - max() to remove Diffuse/Specular when the Light is hitting the back of the surface.
    const half ln = max(dot(l, n), 0.h);
    // Cosine angle between Camera and Normal
    // - step() to remove Diffuse/Specular when the Camera is viewing the back of the surface
    // - Using the XCode Shader Profiler, this performed the best compared to...
    //      - ceil(saturate(v))
    //      - trunc(fma(v, .5h, 1.h))
    const half cn = step(0.h, dot(c, n));
    const half Il = 1 * cn; // Diffuse/Specular Light Intensity

    constexpr sampler tx_sampler(mag_filter::linear, address::repeat, min_filter::linear);

    // Ambient/Diffuse Material Color
    const texture2d<half> tx_diffuse     = material.diffuse_texture;
    const bool            has_tx_diffuse = !is_null_texture(tx_diffuse);
    // TODO: Use material.diffuse_color
    const half4  Kd       = (HAS_AMBIENT || HAS_DIFFUSE) && has_tx_diffuse ? tx_diffuse.sample(tx_sampler, float2(in.tx_coord)) : 1.0;

    // Specular Material Color
    const texture2d<half> tx_spec     = material.specular_texture;
    const bool            has_tx_spec = !is_null_texture(tx_spec);
    // TODO: Use material.specular_color
    const half4           Ks          = HAS_SPECULAR && has_tx_spec ? tx_spec.sample(tx_sampler, float2(in.tx_coord)) : 1.0;

    const half4  diffuse  = HAS_DIFFUSE ? Il * ln * Kd : 0;
    const half   s        = material.specular_shineness;
    const half4  specular = HAS_SPECULAR ? (Il * pow(dot(h, n) * Ks, s)) : 0;

    const half   Ia       = 0.1; // Ambient Intensity
    const half4  ambient  = HAS_AMBIENT ? Ia * Kd : 0;

    return half4(ambient + diffuse + specular);
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