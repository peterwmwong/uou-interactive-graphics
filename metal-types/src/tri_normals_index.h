#pragma once

#ifdef __METAL_VERSION__

// http://johnwhite3d.blogspot.com/2017/10/signed-octahedron-normal-encoding.html
inline half3 decode(float3 n)
{
    float3 o;
    o.x = n.x - n.y;
    o.y = n.x + n.y - 1.0;
    o.z = fma(n.z, 2.0, -1.0) * (1.0 - abs(o.x) - abs(o.y));
    return half3(normalize(o));
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

// TODO: Use a more compact format for storing normals (Octohedron
// See https://aras-p.info/texts/CompactNormalStorage.html
struct TriNormalsIndex {
    packed_float3  normals[3];
    unsigned short index;

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
