#pragma once

// TODO: Use a more compact format for storing normals
// See https://aras-p.info/texts/CompactNormalStorage.html#method04spheremap
struct TriNormalsIndex {
    packed_half3   normals[3];
    unsigned short index;

    #ifdef __METAL_VERSION__
    inline half3 normal(const float2 barycentric_coord, const constant MTLPackedFloat4x3 *transforms) const device {
        const auto    m  = transforms[index];
        const half2   b2 = half2(barycentric_coord);
        const half3   b(1.0 - (b2.x + b2.y), b2.x, b2.y);
        const half3x3 n(normals[0], normals[1], normals[2]);
        const half3   normal = n * b;

        // IMPORTANT: Converting to float before normalize may seem redundant, but for models
        // like yoda, small half precision normals seems to cause normalize to go bonkers.
        return half3(normalize(float3(
            half3x3(half3(m[0]), half3(m[1]), half3(m[2])) * normal
        )));
    }
    #endif // __METAL_VERSION__
};

