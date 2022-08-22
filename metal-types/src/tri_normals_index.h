#pragma once

#ifdef __METAL_VERSION__
inline half3 decode(float2 f) {
    // float2 f = float2(f_half);

    // f = f * 2.0 - 1.0;
    f = f * 2.0 - 1.0;

    // https://twitter.com/Stubbesaurus/status/937994790553227264
    // float3 n = float3( f.x, f.y, 1.0 - abs( f.x ) - abs( f.y ) );
    float3 n = float3(f.x, f.y, 1.0 - abs(f.x) - abs(f.y));

    // float t = saturate( -n.z );
    float t = saturate(-n.z);

    // n.xy += n.xy >= 0.0 ? -t : t;
    n.xy += any(n.xy >= 0.0) ? -t : t;

    // return normalize( n );
    return half3(normalize(n));
}

#endif // __METAL_VERSION__

// TODO: Use a more compact format for storing normals (Octohedron
// See https://aras-p.info/texts/CompactNormalStorage.html
struct TriNormalsIndex {
    packed_float2  normals[3];
    unsigned short index;
    unsigned short _padding;

    #ifdef __METAL_VERSION__
    inline half3 normal(const float2 barycentric_coord, const constant MTLPackedFloat4x3 *transforms) const device {
        const auto    m  = transforms[index];
        const half2   b2 = half2(barycentric_coord);
        const half3   b(1.0 - (b2.x + b2.y), b2.x, b2.y);
        const half3x3 n(decode(normals[0]), decode(normals[1]), decode(normals[2]));
        const half3   normal = n * b;

        // IMPORTANT: Converting to float before normalize may seem redundant, but for models
        // like yoda, small half precision normals seems to cause normalize to go bonkers.
        return half3(normalize(float3(
            half3x3(half3(m[0]), half3(m[1]), half3(m[2])) * normal
        )));
    }
    #endif // __METAL_VERSION__
};

