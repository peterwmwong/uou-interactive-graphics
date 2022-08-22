use super::all_metal_types::TriNormalsIndex;
use crate::packed_float2;
use std::simd::{f32x2, f32x4, SimdFloat, SimdPartialOrd, StdFloat};

#[inline]
fn oct_wrap(v: f32x2) -> f32x2 {
    // (1.0 - abs(v.yx)) *
    (f32x2::splat(1.) - f32x2::from_array([v[1], v[0]]).abs())
        // * (v.xy >= 0.0 ? 1.0 : -1.0)
        * f32x2::splat(if v.simd_ge(f32x2::splat(0.)).any() { 1. } else { -1. })
}

#[inline]
fn encode(v: &[f32], i: usize) -> packed_float2 {
    let mut n = f32x4::from_array([v[i], v[i + 1], v[i + 2], 0.]);
    // n /= (abs( n.x ) + abs( n.y ) + abs( n.z ));
    n /= f32x4::splat(n[0].abs() + n[1].abs() + n[2].abs());
    // n.xy = n.z >= 0.0 ? n.xy : OctWrap( n.xy );
    let mut r = f32x2::from_array([n[0], n[1]]);
    if n[2] < 0. {
        r = oct_wrap(r);
    }
    // n.xy = n.xy * 0.5 + 0.5;
    const P5: f32x2 = f32x2::from_array([0.5, 0.5]);
    r = r.mul_add(P5, P5);
    assert!(r.simd_ge(f32x2::splat(0.0)).all() && r.simd_le(f32x2::splat(1.0)).all());
    packed_float2 { xy: [r[0], r[1]] }
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
            _padding: 0,
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     pub fn test() {
//         let index = 77;
//         let raw_normals = [
//             0.0, 1.0, 2.0, // 0
//             3.0, 4.0, 5.0, // 1
//             6.0, 7.0, 8.0, // 2
//             9.0, 10.0, 11.0, // 3
//         ];

//         let raw_indices = [
//             99, 99, 99, // 0
//             0, 1, 3, // 1
//             99, 99, 99, // 2
//             99, 99, 99, // 3
//         ];
//         let actual =
//             TriNormalsIndex::from_indexed_raw_normals(&raw_normals, &raw_indices, 1, index);

//         assert_eq!(actual.normals[0].xyz, to_packed_half2(&raw_normals, 0).xyz);
//         assert_eq!(actual.normals[1].xyz, to_packed_half2(&raw_normals, 3).xyz);
//         assert_eq!(actual.normals[2].xyz, to_packed_half2(&raw_normals, 9).xyz);
//         assert_eq!(actual.index, index);
//     }
// }
