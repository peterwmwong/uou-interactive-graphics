#pragma once

#include <metal_stdlib>
#include "../../metal-types/src/material.h"

using namespace metal;

struct ConstantMaterial {
    const half4 ambient;
    const half4 diffuse;
    const half4 specular;
    const half  shineness;
    const half  ambient_amt;
    inline ConstantMaterial(half4 ambient, half4 diffuse, half4 specular, half shineness, half ambient_amount):
        ambient(ambient),
        diffuse(diffuse),
        specular(specular),
        shineness(shineness),
        ambient_amt(ambient_amount) {}
    inline half4 ambient_color() const           { return ambient; }
    inline half4 diffuse_color() const           { return diffuse; }
    inline half4 specular_color() const          { return specular; }
    inline half  specular_shineness() const      { return shineness; }
    inline constexpr half ambient_amount() const { return ambient_amt; }
};

struct TexturedMaterial {
    const constant Material & m;
    const          float2     tx_coord;
    const          bool       is_shadow;

    inline TexturedMaterial(constant Material & material,
                                     float2     tx_coord,
                                     bool       is_shadow = false):
        m(material),
        tx_coord(tx_coord),
        is_shadow(is_shadow) {}

    inline constexpr const struct sampler s() const {
        constexpr struct sampler tx_sampler(mag_filter::linear, address::repeat, min_filter::linear);
        return tx_sampler;
    }

    inline half4 ambient_color() const      { return m.ambient_texture.sample(s(), tx_coord); }
    inline half4 diffuse_color() const      { return is_shadow ? 0 : m.diffuse_texture.sample(s(), tx_coord); }
    inline half4 specular_color() const     { return is_shadow ? 0 : m.specular_texture.sample(s(), tx_coord); }
    inline half  specular_shineness() const { return m.specular_shineness; }
    inline half  ambient_amount() const     { return m.ambient_amount; }
};

struct ShadePhongBlinParams {
    const half3 frag_pos;
    const half3 light_pos;
    const half3 camera_pos;
    const half3 normal;
    const bool has_ambient = true;
    const bool has_diffuse = true;
    const bool has_specular = true;
    const bool only_normals = false;
};

template<typename T>
inline half4 shade_phong_blinn(const ShadePhongBlinParams p,
                                     T                    material) {
    /*
    ================================================================
    Rendering Equation: Ambient + Geometry Term (Diffuse + Specular)
    ================================================================

    F(l, c) = Bidirectional Reflectance Distribution Function

    Ambient + Geometry Term (Diffuse    + Specular)
    -------   ------------- ----------   -------------------------------
    Ia Kd   + Il cos(a)     (Kd F(l, c) + Ks (cos(t) F(l, c))^s / cos(a))
    Ia Kd   + Il cos(a)     (Kd         + Ks cos(t)^s           / cos(a))
    Ia Kd   + Il (l·n)      (Kd         + Ks (h·n)^s            / (l·n))

    ...distribute the Geometry Term...

    Ambient + Diffuse     + Specular
    -------   -----------   -------------
    Ia Kd   + Il (l·n) Kd + Il Ks (h·n)^s
    */

    // TODO: START HERE
    // TODO: START HERE
    // TODO: START HERE
    // Switch to putting everything in View Space
    // - Seems more common when I read examples online
    // - Should be a performance improvement
    //      1. Memory reduction: no need to store world.camera_position
    //      2. Faster: calculating `c` (direction/unit vector towards camera) is simply `-pos` (assuming pos is now in view space)
    const half3 l = normalize(p.light_pos - p.frag_pos);  // Light  - world space direction from fragment to light
    const half3 c = normalize(p.camera_pos - p.frag_pos); // Camera - world space direction from fragment to camera
    const half3 h = normalize(l + c);                     // Half   - half-way vector between Light and Camera
    const half3 n = p.normal;                             // Normal - unit vector, world space direction perpendicular to surface
    if (p.only_normals) {
        return half4(n.xy, n.z * -1, 1);
    }

    const half hn = max(dot(h, n), 0.h);
    // Cosine angle between Light and Normal
    // - max() to remove Diffuse/Specular when the Light is hitting the back of the surface.
    const half ln = max(dot(l, n), 0.h);

    const half Ia = material.ambient_amount();
    const half Il = 1. - Ia;
    const half Id = Il * ln;

    half4 color = 0;
    if (p.has_specular) {
        const half4 Ks = material.specular_color();
        color += Il * Ks * powr(hn, material.specular_shineness());
    }
    if (p.has_diffuse) {
        const half4 Kd = material.diffuse_color();
        color += Id * Kd;
    }
    if (p.has_ambient) {
        const half4 Ka = material.ambient_color();
        color += Ia * Ka;
    }
    return color;
}
