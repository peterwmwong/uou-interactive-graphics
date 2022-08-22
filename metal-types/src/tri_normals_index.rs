use super::all_metal_types::TriNormalsIndex;
use crate::packed_float3;
use std::simd::{f32x4, SimdFloat};

#[inline(always)]
fn dbg_ensure_unit_vector(n: f32x4) -> f32x4 {
    #[cfg(debug_assertions)]
    {
        let len = (n * n).reduce_sum().sqrt();
        if (1.0 - len).abs() >= 0.0001 {
            println!("Input is not unit vector (needs normalize)");
        }
        return n / f32x4::splat(len);
    }
    #[cfg(not(debug_assertions))]
    {
        return n;
    }
}

// http://johnwhite3d.blogspot.com/2017/10/signed-octahedron-normal-encoding.html
#[inline]
fn encode(v: &[f32], i: usize) -> packed_float3 {
    let mut n: f32x4 = dbg_ensure_unit_vector(f32x4::from_array([v[i], v[i + 1], v[i + 2], 0.]));
    // float3 OutN;
    let mut out_n = f32x4::splat(0.);

    // n /= ( abs( n.x ) + abs( n.y ) + abs( n.z ) );
    n /= f32x4::splat(n[0].abs() + n[1].abs() + n[2].abs());

    // OutN.y = n.y *  0.5  + 0.5;
    out_n[1] = n[1] * 0.5 + 0.5;

    // OutN.x = n.x *  0.5 + OutN.y;
    out_n[0] = n[0] * 0.5 + out_n[1];

    // OutN.y = n.x * -0.5 + OutN.y;
    out_n[1] = n[0] * -0.5 + out_n[1];

    // OutN.z = saturate(n.z*FLT_MAX);
    out_n[2] = (n[2] * f32::MAX).clamp(0., 1.);
    // out_n[2] = if n[2] >= 0.0 { 1.0 } else { 0.0 };

    // return OutN;
    packed_float3 {
        xyz: [out_n[0], out_n[1], out_n[2]],
    }
}

#[inline]
fn decode(n: packed_float3) -> [f32; 3] {
    let n = f32x4::from_array([n.xyz[0], n.xyz[1], n.xyz[2], 0.0]);
    // float3 OutN;
    let mut out_n = f32x4::splat(0.);

    // OutN.x = (n.x - n.y);
    out_n[0] = n[0] - n[1];

    // OutN.y = (n.x + n.y) - 1.0;
    out_n[1] = (n[0] + n[1]) - 1.0;

    // OutN.z = n.z * 2.0 - 1.0;
    // OutN.z = OutN.z * ( 1.0 - abs(OutN.x) - abs(OutN.y));
    out_n[2] = (n[2] * 2.0 - 1.0) * (1.0 - out_n[0].abs() - out_n[1].abs());

    // OutN = normalize( OutN );
    out_n = out_n / f32x4::splat((out_n * out_n).reduce_sum().sqrt());

    // return OutN;
    [out_n[0], out_n[1], out_n[2]]
}

impl TriNormalsIndex {
    #[inline]
    pub fn from_indexed_raw_normals(
        raw_normals: &[f32],
        raw_indices: &[u32],
        start_vertex: usize,
        index: u16,
    ) -> Self {
        Self {
            normals: [
                encode(raw_normals, (raw_indices[start_vertex * 3] * 3) as _),
                encode(raw_normals, (raw_indices[start_vertex * 3 + 1] * 3) as _),
                encode(raw_normals, (raw_indices[start_vertex * 3 + 2] * 3) as _),
            ],
            index,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn t(input: [f32; 3]) {
        let e = encode(&input, 0);
        let d = decode(e);
        assert_eq!(input, d);
    }

    // TODO: Add tests that uses a Metal Compute Pipeline to verify the shader `decode()`.

    #[test]
    pub fn test() {
        t([1., 0., 0.]);
        t([0., 1., 0.]);
        t([0., 0., 1.]);

        t([-1., 0., 0.]);
        t([0., -1., 0.]);
        t([0., 0., -1.]);

        t([0.7071067811865476, 0.7071067811865475, 0.]);
        t([-0.7071067811865476, 0.7071067811865475, 0.]);
        t([0.7071067811865476, -0.7071067811865475, 0.]);
        t([-0.7071067811865476, -0.7071067811865475, 0.]);

        t([0.7071067811865476, 0., 0.7071067811865475]);
        t([-0.7071067811865476, 0., 0.7071067811865475]);
        t([0.7071067811865476, 0., -0.7071067811865475]);
        t([-0.7071067811865476, 0., -0.7071067811865475]);

        t([0., 0.7071067811865476, 0.7071067811865475]);
        t([0., -0.7071067811865476, 0.7071067811865475]);
        t([0., 0.7071067811865476, -0.7071067811865475]);
        t([0., -0.7071067811865476, -0.7071067811865475]);
    }
}
