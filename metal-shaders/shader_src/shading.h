#pragma once

#include <metal_stdlib>

using namespace metal;

struct ConstantMaterial {
    const half4 ambient;
    const half4 diffuse;
    const half4 specular;
    const half shineness;
    inline ConstantMaterial(half4 ambient, half4 diffuse, half4 specular, half shineness):
        ambient(ambient),
        diffuse(diffuse),
        specular(specular),
        shineness(shineness) {}
    inline half4 ambient_color() const { return ambient; }
    inline half4 diffuse_color() const { return diffuse; }
    inline half4 specular_color() const { return specular; }
    inline half  specular_shineness() const { return shineness; }
    inline constexpr half ambient_amount() const { return 0.15; }
};

template<typename T>
struct TexturedMaterial {
    const    float2     tx_coord;
    const    bool       is_shadow;
    const constant T & m;

    inline TexturedMaterial<T>(          float2    tx_coord,
                                         bool      is_shadow,
                                constant T       & m):
        tx_coord(tx_coord),
        is_shadow(is_shadow),
        m(m) {}

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
};

template<typename T>
inline half4 shade_phong_blinn(const ShadePhongBlinParams p, const T material) {
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

    // TODO: START HERE
    // TODO: START HERE
    // TODO: START HERE
    // Switch to putting everything in View Space
    // - Seems more common when I read examples online
    // - Should be a performance improvement
    //      1. Memory reduction: no need to store world.camera_position
    //      2. Faster: calculating `c` (direction/unit vector towards camera) is simply `-pos` (assuming pos is now in view space)
    const half3 l  = normalize(p.light_pos - p.frag_pos);  // Light  - world space direction from fragment to light
    const half3 c  = normalize(p.camera_pos - p.frag_pos); // Camera - world space direction from fragment to camera
    const half3 h  = normalize(l + c);                 // Half   - half-way vector between Light and Camera
    const half3 n  = p.normal;                           // Normal - unit vector, world space direction perpendicular to surface
    const half  hn = dot(h, n);
    // Cosine angle between Light and Normal
    // - max() to remove Diffuse/Specular when the Light is hitting the back of the surface.
    const half ln = max(dot(l, n), 0.h);

    const half Ia = material.ambient_amount();
    // Diffuse/Specular Light Intensity of 1.0 for camera facing surfaces, otherwise 0.0.
    // - Use Cosine angle between Camera and Normal (positive <90d, negative >90d)
    // - Using the XCode Shader Profiler, this performed the best compared to...
    //      - ceil(saturate(v))
    //      - trunc(fma(v, .5h, 1.h))
    const half Il = step(0.h, dot(c, n)) * (1. - Ia);

    half4 color = 0;

    // TODO: Bring back Function Constant capable specialization (see proj-4), to allow for
    // debug views (ex. normal, ambient only, diffues only, specular only)
    // - Can it be C++ template parameter?
    // - If it's just a function parameter, will the Metal compiler be smart enough to inline and
    //   do constant propagation?
    //   - Should be easy enough with XCode (after GPU profiling) to see the number of instructions
    //     matches that of proj-4 (uses Function Constants, recompiles pipeline after switching
    //     modes).
    const half4 Ks = material.specular_color();
    color += Il * pow(hn * Ks, material.specular_shineness());

    const half4 Ka = material.ambient_color();
    color += Ia * Ka;

    const half4 Kd = material.diffuse_color();
    color += Il * ln * Kd;
    return color;
}
