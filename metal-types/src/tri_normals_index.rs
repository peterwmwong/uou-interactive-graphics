use super::all_metal_types::{packed_half3, TriNormalsIndex};

#[inline]
fn to_packed_half3(v: &[f32], i: usize) -> packed_half3 {
    packed_half3 {
        xyz: [
            half::f16::from_f32(v[i]).to_bits(),
            half::f16::from_f32(v[i + 1]).to_bits(),
            half::f16::from_f32(v[i + 2]).to_bits(),
        ],
    }
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
                to_packed_half3(raw_normals, (raw_indices[start_vertex * 3] * 3) as _),
                to_packed_half3(raw_normals, (raw_indices[start_vertex * 3 + 1] * 3) as _),
                to_packed_half3(raw_normals, (raw_indices[start_vertex * 3 + 2] * 3) as _),
            ],
            index,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{tri_normals_index::to_packed_half3, TriNormalsIndex};

    #[test]
    pub fn test() {
        let index = 77;
        let raw_normals = [
            0.0, 1.0, 2.0, // 0
            3.0, 4.0, 5.0, // 1
            6.0, 7.0, 8.0, // 2
            9.0, 10.0, 11.0, // 3
        ];

        let raw_indices = [
            99, 99, 99, // 0
            0, 1, 3, // 1
            99, 99, 99, // 2
            99, 99, 99, // 3
        ];
        let actual =
            TriNormalsIndex::from_indexed_raw_normals(&raw_normals, &raw_indices, 1, index);

        assert_eq!(actual.normals[0].xyz, to_packed_half3(&raw_normals, 0).xyz);
        assert_eq!(actual.normals[1].xyz, to_packed_half3(&raw_normals, 3).xyz);
        assert_eq!(actual.normals[2].xyz, to_packed_half3(&raw_normals, 9).xyz);
        assert_eq!(actual.index, index);
    }
}
