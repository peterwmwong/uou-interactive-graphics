#include <metal_stdlib>
#include "./common.h"

using namespace metal;

constant bool  HAS_AMBIENT        [[function_constant(static_cast<uint>(FC::HAS_AMBIENT))]];
constant bool  HAS_DIFFUSE        [[function_constant(static_cast<uint>(FC::HAS_DIFFUSE))]];
constant bool  HAS_NORMAL         [[function_constant(static_cast<uint>(FC::HAS_NORMAL))]];
constant bool  HAS_SPECULAR       [[function_constant(static_cast<uint>(FC::HAS_SPECULAR))]];

// TODO: Re-layout memory to be more cache-friendly
// - Currently, with 3 separate arrays for position, normals, and tx_coords, a VS instance must
//   access 3 separate, disjoint memory addresses
// - What if we pack position, normal, and tx_coord as tuple and then have an array of tuples
//   struct VertexData { position; normal; coord; }
//   struct ObjectGeometry { const VertexData * vertex_data; }
// - Downside: Data Duplication
//   - Assess size difference
struct ObjectGeometry {
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
main_vertex(         uint             inst_id         [[instance_id]],
                     uint             vertex_id       [[vertex_id]],
            constant ObjectGeometry & obj_geo         [[buffer(VertexBufferIndex::ObjectGeometry)]],
            constant float4x4       & model_to_proj   [[buffer(VertexBufferIndex::MatrixModelToProjection)]],
            constant float3x3       & normal_to_world [[buffer(VertexBufferIndex::MatrixNormalToWorld)]])
{
    const uint   idx      = obj_geo.indices[inst_id * 3 + vertex_id];
    const float4 position = float4(obj_geo.positions[idx], 1.0);
    const float3 normal   = obj_geo.normals[idx];
    const float2 tx_coord = obj_geo.tx_coords[idx];
    return {
        .position  = model_to_proj * position,
        .normal    = half3(normal_to_world * normal),
        .tx_coord  = half2(tx_coord)
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
main_fragment(         VertexOut   in            [[stage_in]],
              constant float4x4  & proj_to_world [[buffer(FragBufferIndex::MatrixProjectionToWorld)]],
              constant float2    & screen_size   [[buffer(FragBufferIndex::ScreenSize)]],
              constant float3    & light_pos     [[buffer(FragBufferIndex::LightPosition)]],
              constant float3    & cam_pos       [[buffer(FragBufferIndex::CameraPosition)]],
              constant Material  & material      [[buffer(FragBufferIndex::Material)]])
{
    const half3 n = normalize(in.normal); // Normal - unit vector, world space direction perpendicular to surface
    if (HAS_NORMAL) {
        return half4(n.xy, n.z * -1, 1);
    }

    // Calculate the fragment's World Space position from a Metal Viewport Coordinate.
    // 1. Viewport Coordinate -> Normalized Device Coordinate (aka Projected w/Perspective)
    const half2  screen_pos   = half2(in.position.xy);
    const half2  proj_pos_xy  = fma(half2(2, -2), (screen_pos / half2(screen_size)), half2(-1, 1));
    // 2. Projected Coordinate -> World Space position
    const float4 proj_pos     = float4(float2(proj_pos_xy), in.position.z, 1);
    const float4 pos_w_persp  = proj_to_world * proj_pos;
    const half3  pos          = half3(pos_w_persp.xyz / pos_w_persp.w);

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
    const half3 l  = normalize(half3(light_pos.xyz) - pos); // Light  - world space direction from fragment to light
    const half3 c  = normalize(half3(cam_pos.xyz) - pos);   // Camera - world space direction from fragment to camera
    const half3 h  = normalize(l + c);                      // Half   - half-way vector between Light and Camera

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
    const float2 tx_coord = float2(in.tx_coord);
    // TODO: Use material.diffuse_color
    const half4  Kd       = HAS_AMBIENT || HAS_DIFFUSE ? material.diffuse_texture.sample(tx_sampler, tx_coord) : 0;

    // Specular Material Color
    const texture2d<half> tx_spec     = material.specular_texture;
    const bool            has_tx_spec = !is_null_texture(tx_spec);
    // TODO: Use material.specular_color
    const half4           Ks          = HAS_SPECULAR && has_tx_spec ? tx_spec.sample(tx_sampler, tx_coord) : 0;

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
light_vertex(constant float4x4 & model_to_proj [[buffer(LightVertexBufferIndex::MatrixWorldToProjection)]],
             constant float4   & light_pos     [[buffer(LightVertexBufferIndex::LightPosition)]])
{
    return {
        .position = model_to_proj * light_pos,
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