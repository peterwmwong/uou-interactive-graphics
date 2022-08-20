#![feature(array_zip)]
#![feature(portable_simd)]
mod all_metal_types;
mod all_metal_types_list;
mod tri_normals_index;

pub use all_metal_types::*;
pub use all_metal_types_list::*;
use metal::{MTLPackedFloat3, MTLPackedFloat4x3};
pub use tri_normals_index::*;

/**************************************************************************************************
 Helper methods and trait implementations make it easier to write and read Metal types.
 See `metal-types/src/rust_bindgen_only_metal_types.rs`.
***************************************************************************************************/
use std::{
    ffi::c_ushort,
    fmt::Debug,
    ops::{Mul, Sub},
    simd::{f32x2, f32x4, u16x2, SimdFloat},
};

#[allow(non_camel_case_types)]
pub type float = f32;
#[allow(non_camel_case_types)]
pub type uint = u32;
#[allow(non_camel_case_types)]
pub type int = i32;
#[allow(non_camel_case_types)]
pub type ushort = u16;
#[allow(non_camel_case_types)]
pub type short = i16;

// TODO: Add some tests to verify this actually correct for whatever platfrom this is
// running on.
macro_rules! transmute_from_to {
    ($from_ident:ident, $to_ident:ident) => {
        impl From<$from_ident> for $to_ident {
            #[inline(always)]
            fn from(simd: $from_ident) -> Self {
                unsafe { std::mem::transmute(simd) }
            }
        }
        impl From<$to_ident> for $from_ident {
            #[inline(always)]
            fn from(simd: $to_ident) -> Self {
                unsafe { std::mem::transmute(simd) }
            }
        }
    };
}

transmute_from_to!(f32x2, float2);
transmute_from_to!(f32x2, packed_float2);
transmute_from_to!(f32x4, float4);
transmute_from_to!(f32x4, packed_float4);
transmute_from_to!(float2, packed_float2);
transmute_from_to!(float4, packed_float4);
transmute_from_to!(u16x2, ushort2);

#[allow(non_camel_case_types)]
pub trait f32x4_extras {
    fn length(&self) -> f32;
    fn normalize(&self) -> f32x4;
    fn reflect(&self, incident: f32x4) -> f32x4;
}

impl f32x4_extras for f32x4 {
    fn length(&self) -> f32 {
        (self * self).reduce_sum().sqrt()
    }

    fn normalize(&self) -> f32x4 {
        self * f32x4::splat(1. / self.length())
    }

    fn reflect(&self, incident: f32x4) -> f32x4 {
        let self_norm = self.normalize();
        incident - (f32x4::splat(2. * dot(self_norm, incident)) * self_norm)
    }
}

#[inline]
fn dot(lhs: f32x4, rhs: f32x4) -> f32 {
    (lhs * rhs).reduce_sum()
}

impl From<MTLPackedFloat4x3> for float4x3 {
    #[inline]
    fn from(MTLPackedFloat4x3 { columns: c }: MTLPackedFloat4x3) -> Self {
        Self {
            columns: [
                [c[0].0, c[0].1, c[0].2, 0.],
                [c[1].0, c[1].1, c[1].2, 0.],
                [c[2].0, c[2].1, c[2].2, 0.],
                [c[3].0, c[3].1, c[3].2, 0.],
            ],
        }
    }
}

impl From<float4x3> for MTLPackedFloat4x3 {
    fn from(float4x3 { columns: c }: float4x3) -> Self {
        Self {
            columns: [
                MTLPackedFloat3(c[0][0], c[0][1], c[0][2]),
                MTLPackedFloat3(c[1][0], c[1][1], c[1][2]),
                MTLPackedFloat3(c[2][0], c[2][1], c[2][2]),
                MTLPackedFloat3(c[3][0], c[3][1], c[3][2]),
            ],
        }
    }
}

#[allow(non_camel_case_types)]
pub type f32x4x4 = float4x4;

impl f32x4x4 {
    #[inline]
    pub const fn new(row1: [f32; 4], row2: [f32; 4], row3: [f32; 4], row4: [f32; 4]) -> Self {
        f32x4x4 {
            columns: [
                [row1[0], row2[0], row3[0], row4[0]],
                [row1[1], row2[1], row3[1], row4[1]],
                [row1[2], row2[2], row3[2], row4[2]],
                [row1[3], row2[3], row3[3], row4[3]],
            ],
        }
    }

    #[inline]
    pub const fn transpose(&self) -> Self {
        let c = self.columns;
        f32x4x4 {
            columns: [
                [c[0][0], c[1][0], c[2][0], c[3][0]],
                [c[0][1], c[1][1], c[2][1], c[3][1]],
                [c[0][2], c[1][2], c[2][2], c[3][2]],
                [c[0][3], c[1][3], c[2][3], c[3][3]],
            ],
        }
    }

    #[inline]
    pub fn inverse(&self) -> Self {
        // Based on https://stackoverflow.com/questions/1148309/inverting-a-4x4-matrix/44446912#44446912
        let c = self.columns;
        let a2323 = c[2][2] * c[3][3] - c[3][2] * c[2][3];
        let a1323 = c[1][2] * c[3][3] - c[3][2] * c[1][3];
        let a1223 = c[1][2] * c[2][3] - c[2][2] * c[1][3];
        let a0323 = c[0][2] * c[3][3] - c[3][2] * c[0][3];
        let a0223 = c[0][2] * c[2][3] - c[2][2] * c[0][3];
        let a0123 = c[0][2] * c[1][3] - c[1][2] * c[0][3];

        let x1 = c[1][1] * a2323 - c[2][1] * a1323 + c[3][1] * a1223;
        let x2 = c[2][1] * a0323 - c[3][1] * a0223 - c[0][1] * a2323;
        let x3 = c[0][1] * a1323 - c[1][1] * a0323 + c[3][1] * a0123;
        let x4 = c[1][1] * a0223 - c[2][1] * a0123 - c[0][1] * a1223;
        let inv_det =
            f32x4::splat(1. / (c[0][0] * x1 + c[1][0] * x2 + c[2][0] * x3 + c[3][0] * x4));
        return Self {
            columns: [
                (f32x4::from_array([x1, x2, x3, x4]) * inv_det).to_array(),
                {
                    (f32x4::from_array([
                        -(c[1][0] * a2323 - c[2][0] * a1323 + c[3][0] * a1223),
                        (c[0][0] * a2323 - c[2][0] * a0323 + c[3][0] * a0223),
                        -(c[0][0] * a1323 - c[1][0] * a0323 + c[3][0] * a0123),
                        (c[0][0] * a1223 - c[1][0] * a0223 + c[2][0] * a0123),
                    ]) * inv_det)
                        .to_array()
                },
                {
                    let a1313 = c[1][1] * c[3][3] - c[3][1] * c[1][3];
                    let a2313 = c[2][1] * c[3][3] - c[3][1] * c[2][3];
                    let a1213 = c[1][1] * c[2][3] - c[2][1] * c[1][3];
                    let a0313 = c[0][1] * c[3][3] - c[3][1] * c[0][3];
                    let a0213 = c[0][1] * c[2][3] - c[2][1] * c[0][3];
                    let a0113 = c[0][1] * c[1][3] - c[1][1] * c[0][3];
                    (f32x4::from_array([
                        (c[1][0] * a2313 - c[2][0] * a1313 + c[3][0] * a1213),
                        -(c[0][0] * a2313 - c[2][0] * a0313 + c[3][0] * a0213),
                        (c[0][0] * a1313 - c[1][0] * a0313 + c[3][0] * a0113),
                        -(c[0][0] * a1213 - c[1][0] * a0213 + c[2][0] * a0113),
                    ]) * inv_det)
                        .to_array()
                },
                {
                    let a2312 = c[2][1] * c[3][2] - c[3][1] * c[2][2];
                    let a1312 = c[1][1] * c[3][2] - c[3][1] * c[1][2];
                    let a1212 = c[1][1] * c[2][2] - c[2][1] * c[1][2];
                    let a0312 = c[0][1] * c[3][2] - c[3][1] * c[0][2];
                    let a0212 = c[0][1] * c[2][2] - c[2][1] * c[0][2];
                    let a0112 = c[0][1] * c[1][2] - c[1][1] * c[0][2];
                    (f32x4::from_array([
                        -(c[1][0] * a2312 - c[2][0] * a1312 + c[3][0] * a1212),
                        (c[0][0] * a2312 - c[2][0] * a0312 + c[3][0] * a0212),
                        -(c[0][0] * a1312 - c[1][0] * a0312 + c[3][0] * a0112),
                        (c[0][0] * a1212 - c[1][0] * a0212 + c[2][0] * a0112),
                    ]) * inv_det)
                        .to_array()
                },
            ],
        };
    }

    #[inline]
    pub const fn zero_translate(&self) -> Self {
        Self {
            columns: [
                self.columns[0],
                self.columns[1],
                self.columns[2],
                [0., 0., 0., 1.],
            ],
        }
    }

    // TODO: Add translate_scale_rotate()
    // - Many projects need this for m_model_to_world and instead perform the heavy 3 matrix
    //   multiplications.
    // - Generates an immense amount of instructions, because floating point operations cannot be
    //   used in const evaluation.

    // TODO: test
    #[inline]
    pub const fn scale_translate(sx: f32, sy: f32, sz: f32, tx: f32, ty: f32, tz: f32) -> Self {
        Self::new(
            [sx, 0., 0., tx],
            [0., sy, 0., ty],
            [0., 0., sz, tz],
            [0., 0., 0., 1.],
        )
    }

    #[inline]
    pub const fn translate(x: f32, y: f32, z: f32) -> Self {
        Self::new(
            [1., 0., 0., x],
            [0., 1., 0., y],
            [0., 0., 1., z],
            [0., 0., 0., 1.],
        )
    }

    #[inline]
    pub const fn scale(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self::new(
            [x, 0., 0., 0.],
            [0., y, 0., 0.],
            [0., 0., z, 0.],
            [0., 0., 0., w],
        )
    }

    #[inline]
    pub fn z_rotate(zrot: f32) -> Self {
        Self::new(
            [zrot.cos(), zrot.sin(), 0., 0.],
            [-zrot.sin(), zrot.cos(), 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        )
    }

    #[inline]
    pub fn y_rotate(yrot: f32) -> Self {
        Self::new(
            [yrot.cos(), 0., -yrot.sin(), 0.],
            [0., 1., 0., 0.],
            [yrot.sin(), 0., yrot.cos(), 0.],
            [0., 0., 0., 1.],
        )
    }

    #[inline]
    pub fn x_rotate(xrot: f32) -> Self {
        Self::new(
            [1., 0., 0., 0.],
            [0., xrot.cos(), xrot.sin(), 0.],
            [0., -xrot.sin(), xrot.cos(), 0.],
            [0., 0., 0., 1.],
        )
    }

    #[inline]
    pub fn rotate(xrot: f32, yrot: f32, zrot: f32) -> Self {
        Self::x_rotate(xrot) * Self::y_rotate(yrot) * Self::z_rotate(zrot)
    }

    #[inline]
    pub const fn identity() -> Self {
        Self::scale(1., 1., 1., 1.)
    }

    #[inline]
    pub const fn row<const N: usize>(&self) -> f32x4 {
        let c = self.columns;
        f32x4::from_array([c[0][N], c[1][N], c[2][N], c[3][N]])
    }
}

impl Mul<f32x4> for f32x4x4 {
    type Output = f32x4;

    #[inline]
    fn mul(self, rhs: f32x4) -> Self::Output {
        f32x4::from_array(self.transpose().columns.map(|r| dot(r.into(), rhs)))
    }
}

impl Mul<f32x4x4> for f32x4x4 {
    type Output = f32x4x4;

    #[inline]
    fn mul(self, rhs: f32x4x4) -> Self::Output {
        let rows = self.transpose().columns;
        Self {
            columns: rhs.columns.map(|col| {
                f32x4::from_array([
                    dot(rows[0].into(), col.into()),
                    dot(rows[1].into(), col.into()),
                    dot(rows[2].into(), col.into()),
                    dot(rows[3].into(), col.into()),
                ])
                .into()
            }),
        }
    }
}

impl Debug for f32x4x4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str_row = |row: f32x4| {
            let s = row
                .as_array()
                .map(|e| format!("{e:>+10.5}").replace("+", " "))
                .join(",");
            format!("[{s}]")
        };
        f.debug_tuple("f32x4x4")
            .field(&str_row(self.row::<0>()))
            .field(&str_row(self.row::<1>()))
            .field(&str_row(self.row::<2>()))
            .field(&str_row(self.row::<3>()))
            .finish()
    }
}

impl Sub<f32x4x4> for f32x4x4 {
    type Output = f32x4x4;

    #[inline]
    fn sub(self, rhs: f32x4x4) -> Self::Output {
        let columns = self
            .columns
            .zip(rhs.columns)
            .map(|(l, r)| (f32x4::from_array(l) - f32x4::from_array(r)).to_array());
        Self { columns }
    }
}

impl From<f32x4x4> for float4x3 {
    #[inline(always)]
    fn from(f32x4x4 { columns: c }: f32x4x4) -> Self {
        Self {
            columns: [
                [c[0][0], c[0][1], c[0][2], 0.],
                [c[1][0], c[1][1], c[1][2], 0.],
                [c[2][0], c[2][1], c[2][2], 0.],
                [c[3][0], c[3][1], c[3][2], 0.],
            ],
        }
    }
}

impl From<float4x3> for f32x4x4 {
    #[inline(always)]
    fn from(float4x3 { columns: c }: float4x3) -> Self {
        Self {
            columns: [
                [c[0][0], c[0][1], c[0][2], 0.0],
                [c[1][0], c[1][1], c[1][2], 0.0],
                [c[2][0], c[2][1], c[2][2], 0.0],
                [c[3][0], c[3][1], c[3][2], 1.0],
            ],
        }
    }
}

impl From<f32x4x4> for float3x3 {
    #[inline(always)]
    fn from(f32x4x4 { columns: c }: f32x4x4) -> Self {
        float3x3 {
            columns: [c[0], c[1], c[2]],
        }
    }
}

impl From<f32x4x4> for MTLPackedFloat4x3 {
    #[inline(always)]
    fn from(m: f32x4x4) -> Self {
        Self {
            columns: [
                MTLPackedFloat3(m.columns[0][0], m.columns[0][1], m.columns[0][2]),
                MTLPackedFloat3(m.columns[1][0], m.columns[1][1], m.columns[1][2]),
                MTLPackedFloat3(m.columns[2][0], m.columns[2][1], m.columns[2][2]),
                MTLPackedFloat3(m.columns[3][0], m.columns[3][1], m.columns[3][2]),
            ],
        }
    }
}
impl From<MTLPackedFloat4x3> for f32x4x4 {
    #[inline(always)]
    fn from(m: MTLPackedFloat4x3) -> Self {
        f32x4x4 {
            columns: [
                [m.columns[0].0, m.columns[0].1, m.columns[0].2, 0.],
                [m.columns[1].0, m.columns[1].1, m.columns[1].2, 0.],
                [m.columns[2].0, m.columns[2].1, m.columns[2].2, 0.],
                [m.columns[3].0, m.columns[3].1, m.columns[3].2, 1.],
            ],
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::simd::SimdPartialOrd;

    mod test_f32x4_extras {
        use super::*;

        fn assert_eq_f32x4(actual: f32x4, expected: f32x4) {
            const TOLERANCE: f32x4 = f32x4::from_array([1e-6; 4]);
            let pass = (actual - expected).abs().simd_lt(TOLERANCE).all();
            if !pass {
                dbg!(expected, actual);
            }
            assert!(pass);
        }

        #[test]
        fn test_length() {
            let actual = f32x4::from_array([1., 2., 3., 4.]).length();
            let expected =
                (1_f32.powf(2.) + 2_f32.powf(2.) + 3_f32.powf(2.) + 4_f32.powf(2.)).sqrt();
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_normalize() {
            let v = f32x4::from_array([1., 2., 3., 4.]);
            let actual = v.normalize();
            let expected = v / f32x4::splat(v.length());
            assert_eq_f32x4(actual, expected);
        }

        #[test]
        fn test_reflect() {
            fn t(v: [f32; 4], i: [f32; 4], expected: [f32; 4]) {
                let v: f32x4 = v.into();
                let i: f32x4 = i.into();
                let expected: f32x4 = expected.into();
                let actual = v.reflect(i);

                assert_eq_f32x4(actual, expected);
            }
            t([1., 0., 0., 0.], [1., 1., 0., 0.], [-1., 1., 0., 0.]);
            t([1., 1., 0., 0.], [1., 0., 0., 0.], [0., -1., 0., 0.]);
            t([1., 1., 0., 0.], [0., 1., 0., 0.], [-1., 0., 0., 0.]);
        }
    }

    mod test_f32x4x4 {
        use super::*;

        #[test]
        fn test_inverse() {
            let m = f32x4x4::rotate(1., 2., 3.) * f32x4x4::translate(40., 50., 60.);
            let inv_m = m.inverse();

            let expected = f32x4x4::identity();
            let actual = inv_m * m;
            let diff = actual - expected;

            const TOLERANCE: f32x4 = f32x4::from_array([3.82e-6; 4]);
            for c in diff.columns {
                let c: f32x4 = c.into();
                assert!(c.abs().simd_lt(TOLERANCE).all());
            }
        }

        #[test]
        fn test_translate() {
            let t = f32x4x4::translate(40., 50., 60.);
            let p = f32x4::from_array([1., 2., 3., 1.]);

            let result = t * p;
            assert_eq!(result, f32x4::from_array([41., 52., 63., 1.]));
        }

        #[test]
        fn test_zero_translate() {
            let r = f32x4x4::rotate(1., 2., 3.);
            let m = r * f32x4x4::translate(40., 50., 60.);

            let result = m.zero_translate();
            assert_eq!(result, r);
        }

        #[test]
        fn test_row() {
            let m = f32x4x4::new(
                [5., 6., 7., 8.],
                [9., 10., 11., 12.],
                [13., 14., 15., 16.],
                [17., 18., 19., 20.],
            );

            assert_eq!(m.row::<0>(), f32x4::from_array([5., 6., 7., 8.]));
            assert_eq!(m.row::<1>(), f32x4::from_array([9., 10., 11., 12.]));
            assert_eq!(m.row::<2>(), f32x4::from_array([13., 14., 15., 16.]));
            assert_eq!(m.row::<3>(), f32x4::from_array([17., 18., 19., 20.]));
        }

        #[test]
        fn test_mul_with_f32x4() {
            let r = f32x4::from_array([1., 2., 3., 4.]);
            let m = f32x4x4::new(
                [5., 6., 7., 8.],
                [9., 10., 11., 12.],
                [13., 14., 15., 16.],
                [17., 18., 19., 20.],
            );

            let result = m * r;
            assert_eq!(
                result,
                f32x4::from_array([
                    5. * 1. + 6. * 2. + 7. * 3. + 8. * 4.,
                    9. * 1. + 10. * 2. + 11. * 3. + 12. * 4.,
                    13. * 1. + 14. * 2. + 15. * 3. + 16. * 4.,
                    17. * 1. + 18. * 2. + 19. * 3. + 20. * 4.,
                ])
            )
        }

        #[test]
        fn test_mul_with_f32x4x4() {
            let left = f32x4x4::new(
                [1., 2., 3., 4.],
                [5., 6., 7., 8.],
                [9., 10., 11., 12.],
                [13., 14., 15., 16.],
            );
            let right = f32x4x4::new(
                [17., 18., 19., 20.],
                [21., 22., 23., 24.],
                [25., 26., 27., 28.],
                [29., 30., 31., 32.],
            );

            let result = left * right;
            let columns = right.columns.map(|a| f32x4::from_array(a));
            assert_eq!(
                result,
                f32x4x4::new(
                    [
                        (left.row::<0>() * columns[0]).reduce_sum(),
                        (left.row::<0>() * columns[1]).reduce_sum(),
                        (left.row::<0>() * columns[2]).reduce_sum(),
                        (left.row::<0>() * columns[3]).reduce_sum(),
                    ],
                    [
                        (left.row::<1>() * columns[0]).reduce_sum(),
                        (left.row::<1>() * columns[1]).reduce_sum(),
                        (left.row::<1>() * columns[2]).reduce_sum(),
                        (left.row::<1>() * columns[3]).reduce_sum(),
                    ],
                    [
                        (left.row::<2>() * columns[0]).reduce_sum(),
                        (left.row::<2>() * columns[1]).reduce_sum(),
                        (left.row::<2>() * columns[2]).reduce_sum(),
                        (left.row::<2>() * columns[3]).reduce_sum(),
                    ],
                    [
                        (left.row::<3>() * columns[0]).reduce_sum(),
                        (left.row::<3>() * columns[1]).reduce_sum(),
                        (left.row::<3>() * columns[2]).reduce_sum(),
                        (left.row::<3>() * columns[3]).reduce_sum(),
                    ]
                )
            );
        }
    }
}

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct MTLQuadTessellationFactorsHalf {
    pub edgeTessellationFactor: [c_ushort; 4usize],
    pub insideTessellationFactor: [c_ushort; 2usize],
}

impl MTLQuadTessellationFactorsHalf {
    pub fn new(v: u16) -> Self {
        let v = half::f16::from_f32(v as _).to_bits();
        Self {
            edgeTessellationFactor: [v; 4],
            insideTessellationFactor: [v; 2],
        }
    }
}
