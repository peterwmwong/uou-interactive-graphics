DEF_CONSTANT constexpr unsigned int DEBUG_PATH_MAX_NUM_POINTS = 8;

#ifdef __METAL_VERSION__

constant constexpr bool HasDebugPath [[function_constant(4)]];

struct DebugPath;
struct DebugPathHelper {
    device DebugPath * dbg_ray [[function_constant(HasDebugPath)]];
    bool active                [[function_constant(HasDebugPath)]];

    inline void add_point(const float3 p);
    inline void add_relative_point(const float3 dir_from_previous);
    template<typename T>
    inline void add_intersection(const raytracing::ray r, const T intersection);
};
#endif

struct DebugPath {
    packed_float3 points[DEBUG_PATH_MAX_NUM_POINTS];
    float2 screen_pos;
    bool update_disabled;
    unsigned char num_points;

#ifdef __METAL_VERSION__
    inline DebugPathHelper activate_if_screen_pos(const float2 pos) device {
        if (HasDebugPath) {
            const bool active = !update_disabled && all(abs(screen_pos - pos) <= float2(0.5));
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
    if (HasDebugPath) {
        if (active) {
            dbg_ray->points[dbg_ray->num_points] = p;
            dbg_ray->num_points++;
        }
    }
}

inline void DebugPathHelper::add_relative_point(const float3 dir_from_previous) {
    if (HasDebugPath) {
        add_point(dbg_ray->points[dbg_ray->num_points - 1] + dir_from_previous);
    }
}

template<typename T>
inline void DebugPathHelper::add_intersection(const raytracing::ray r, const T intersection) {
    if (HasDebugPath) {
        add_point(r.origin + (r.direction * intersection.distance));
    }
}
#endif
