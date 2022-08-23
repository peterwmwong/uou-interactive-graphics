#pragma once

#include "./macros.h"

DEF_CONSTANT constexpr unsigned int DEBUG_PATH_MAX_NUM_POINTS = 8;

#ifdef __METAL_VERSION__

constant constexpr bool UpdateDebugPath [[function_constant(4)]];

struct DebugPath;
struct DebugPathHelper {
    device DebugPath * dbg_ray [[function_constant(UpdateDebugPath)]];
    bool active                [[function_constant(UpdateDebugPath)]];

    inline void add_point(const float3 p);
    inline void add_point(const half3 p);
    inline void add_relative_point(const float3 dir_from_previous);
    inline void add_relative_point(const half3 dir_from_previous);
    template<typename T>
    inline void add_intersection(const raytracing::ray r, const T intersection);
};
#endif

struct DebugPath {
    packed_float3 points[DEBUG_PATH_MAX_NUM_POINTS];
    float2 screen_pos;
    unsigned char num_points;

#ifdef __METAL_VERSION__
    inline DebugPathHelper activate_if_screen_pos(const float2 pos) device {
        if (UpdateDebugPath) {
            const bool active = all(abs(screen_pos - pos) < float2(0.5));
            if (active) num_points = 0;
            return DebugPathHelper { .dbg_ray = this, .active = active };
        } else {
            return DebugPathHelper {};
        }
    }
#endif
};

#ifdef __METAL_VERSION__
inline void DebugPathHelper::add_point(const float3 p) {
    if (UpdateDebugPath) {
        if (active) {
            dbg_ray->points[dbg_ray->num_points] = p;
            dbg_ray->num_points++;
        }
    }
}
inline void DebugPathHelper::add_point(const half3 p) {
    if (UpdateDebugPath) {
        add_point(float3(p));
    }
}

inline void DebugPathHelper::add_relative_point(const float3 dir_from_previous) {
    if (UpdateDebugPath) {
        add_point(dbg_ray->points[dbg_ray->num_points - 1] + dir_from_previous);
    }
}

inline void DebugPathHelper::add_relative_point(const half3 dir_from_previous) {
    if (UpdateDebugPath) {
        add_relative_point(float3(dir_from_previous));
    }
}

template<typename T>
inline void DebugPathHelper::add_intersection(const raytracing::ray r, const T intersection) {
    if (UpdateDebugPath) {
        add_point(r.origin + (r.direction * intersection.distance));
    }
}
#endif
