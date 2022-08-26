#pragma once

#ifdef __METAL_VERSION__

inline half3x3 decompress(const uint n0, const uint n1) {
    const half3 a0 = unpack_unorm10a2_to_half(n0).xyz;
    const half3 a1 = unpack_unorm10a2_to_half(n1).xyz;
    const auto n12z = n1 >> 30;

    // TODO: START HERE
    // TODO: START HERE
    // TODO: START HERE
    // -What if we encode the xs closer together?
    // - What about heterogeneous encoding a0 is 1010102, but a1 is something else?
    return half3x3(
        half3(a0.x, a1.x, a0.z),
        half3(a0.y, a1.y, a1.z),
        half3(half(n0 >> 30), half(n12z & 1), half(n12z > 1))
    );
}

// http://johnwhite3d.blogspot.com/2017/10/signed-octahedron-normal-encoding.html
inline half3x3 decode_normal(half3 xs, half3 ys, half3 zs) {
    half3 oxs = xs - ys;
    half3 oys = xs + ys - 1.0;
    half3 ozs = (zs * 2.0 - 1.0) * (1.0 - abs(oxs) - abs(oys));
    return half3x3(
        normalize(half3(oxs.x, oys.x, ozs.x)),
        normalize(half3(oxs.y, oys.y, ozs.y)),
        normalize(half3(oxs.z, oys.z, ozs.z))
    );
}

inline half3x3 decode(const uint n0, const uint n1) {
    const auto ns = decompress(n0, n1);
    return decode_normal(ns[0], ns[1], ns[2]);
}
#endif // __METAL_VERSION__

struct TriNormals {
    // 3 Normals packed/encoded into 2 32-bit (10 10 10 2)
    //          | X            | Y             | Z-sign
    // ---------|--------------|---------------|-------------------
    // normal 0 | normals[0].x | normals[0].y  | normals[0].w
    // normal 1 | normals[1].x | normals[1].y  | normals[1].w&1
    // normal 2 | normals[0].z | normals[1].z  | (normals[1].w>>1)&1
    unsigned int normals[2];

    #ifdef __METAL_VERSION__
    inline half3 normal(const float2 barycentric_coord, const constant half3x3 *m) const device {
        const half2   b2 = half2(barycentric_coord);
        const half3   b(1.0 - (b2.x + b2.y), b2.x, b2.y);
        const half3x3 n(decode(normals[0], normals[1]));
        const half3   normal = n * b;

        // IMPORTANT: Converting to float before normalize may seem redundant, but for models
        // like yoda, small half precision normals seems to cause normalize to go bonkers.
        return half3(normalize(float3((*m) * normal)));
    }
    #endif // __METAL_VERSION__
};
