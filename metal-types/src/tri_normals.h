#pragma once

#ifdef __METAL_VERSION__

inline half3x3 decompress(const uint n0, const uint n1) {
    const half3 a0 = unpack_unorm10a2_to_half(n0).xyz;
    const half3 a1 = unpack_unorm10a2_to_half(n1).xyz;
    const auto n12z = n1 >> 30;
    return half3x3(
        half3(a0.x, a0.y, half(n0 >> 30)),
        half3(a1.x, a1.y, half(n12z & 1)),
        half3(a0.z, a1.z, half(n12z > 1))
    );
}

// http://johnwhite3d.blogspot.com/2017/10/signed-octahedron-normal-encoding.html
inline half3 decode_normal(half3 n) {
    half3 o;
    o.x = n.x - n.y;
    o.y = n.x + n.y - 1.0;
    o.z = (n.z * 2.0 - 1.0) * (1.0 - abs(o.x) - abs(o.y));
    return normalize(o);
}

inline half3x3 decode(const uint n0, const uint n1) {
    const auto ns = decompress(n0, n1);
    return half3x3(
        decode_normal(ns[0]),
        decode_normal(ns[1]),
        decode_normal(ns[2])
    );
}

// TODO: Can we vectorize this be decode 3 normals at once?
// Idea: Instead of encoding 3 encoded normals as ([xy], [xy], [xy])... but ([xxx], [yyy])
// inline half3x3 decode_2(float3 xs, float3 ys, float3 zs)
// {
//     const float3 nx = xs - ys;
//     const float3 ny = xs + ys - 1.0;
//     const float3 nz = (zs * 2.0 - 1.0) * (1.0 - abs(nx) - abs(ny));
//     return half3x3(
//         half3(normalize(float3(nx.x, ny.x, nz.x))),
//         half3(normalize(float3(nx.y, ny.y, nz.y))),
//         half3(normalize(float3(nx.z, ny.z, nz.z)))
//     );
// }
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
