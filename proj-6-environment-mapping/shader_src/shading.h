#ifndef shading_h
#define shading_h

#include <metal_stdlib>

using namespace metal;

// TODO: Extract this out into a metal-shader-common crate or something.
// Instead of just a folder at the project, a crate opens the door for automated testing and
// benchmarking. The idea is the crate (rust) could provide all the boilerplate (Metal setup,
// pipeline, etc.) to test/benchmark shared shader functions.
template<typename T>
inline half4 shade_phong_blinn(const half3 frag_pos, const half3 light_pos, const half3 camera_pos, const half3 normal, T material) {
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
    const half3 l  = normalize(light_pos - frag_pos);  // Light  - world space direction from fragment to light
    const half3 c  = normalize(camera_pos - frag_pos); // Camera - world space direction from fragment to camera
    const half3 h  = normalize(l + c);                 // Half   - half-way vector between Light and Camera
    const half3 n  = normal;                           // Normal - unit vector, world space direction perpendicular to surface
    const half  hn = dot(h, n);
    // Cosine angle between Light and Normal
    // - max() to remove Diffuse/Specular when the Light is hitting the back of the surface.
    const half ln = max(dot(l, n), 0.h);

    // Diffuse/Specular Light Intensity of 1.0 for camera facing surfaces, otherwise 0.0.
    // - Use Cosine angle between Camera and Normal (positive <90d, negative >90d)
    // - Using the XCode Shader Profiler, this performed the best compared to...
    //      - ceil(saturate(v))
    //      - trunc(fma(v, .5h, 1.h))
    // TODO: Parameterize ambient light intensity.
    const half Ia = 0.6;
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

inline half4 shade_mirror(        float4     screen_pos,
                                  float4     camera_pos_f,
                                  float3     normal_f,
                                  float4x4   matrix_screen_to_world,
                          texturecube<half>  bg_texture) {
    // Calculate the fragment's World Space position from a Metal Viewport Coordinate.
    const float4 pos_w      = matrix_screen_to_world * float4(screen_pos.xyz, 1);
    const half3  pos        = half3(pos_w.xyz / pos_w.w);
    const half3  camera_pos = half3(camera_pos_f.xyz);
    const half3  camera_dir = normalize(pos - camera_pos.xyz);
    const half3  normal     = half3(normalize(normal_f));
    const half3  ref        = reflect(camera_dir, normal);

    constexpr sampler tx_sampler(mag_filter::linear, address::clamp_to_zero, min_filter::linear);

    struct Material {
        const half4 color;
        inline Material(half4 c): color(c) {}
        inline half4 ambient_color() { return color; }
        inline half4 diffuse_color() { return color; }
        inline half4 specular_color() { return color; }
        inline half specular_shineness() { return 50; }
    };
    // TODO: Bring back the Light component (moveable, rendered light) to proj-6.
    constexpr half3 light_position = half3(0, 1, -1);
    const     half4 bg_color       = bg_texture.sample(tx_sampler, float3(ref));
    return shade_phong_blinn(pos, light_position, camera_pos, normal, Material(bg_color));
}

#endif