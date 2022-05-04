use std::ops::{Mul, Sub};
use std::simd::f32x4;

#[inline]
fn dot(lhs: f32x4, rhs: f32x4) -> f32 {
    (lhs * rhs).reduce_sum()
}

#[derive(Copy, Clone, PartialEq)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[allow(non_camel_case_types)]
pub struct f32x4x4 {
    columns: [f32x4; 4],
}

impl f32x4x4 {
    #[inline]
    pub const fn new(row1: [f32; 4], row2: [f32; 4], row3: [f32; 4], row4: [f32; 4]) -> Self {
        f32x4x4 {
            columns: [
                // TODO: Compare with using f32x4::gather()
                f32x4::from_array([row1[0], row2[0], row3[0], row4[0]]),
                f32x4::from_array([row1[1], row2[1], row3[1], row4[1]]),
                f32x4::from_array([row1[2], row2[2], row3[2], row4[2]]),
                f32x4::from_array([row1[3], row2[3], row3[3], row4[3]]),
            ],
        }
    }

    pub fn transpose(&self) -> Self {
        let c = self.columns;
        f32x4x4 {
            columns: [
                f32x4::from_array([c[0][0], c[1][0], c[2][0], c[3][0]]),
                f32x4::from_array([c[0][1], c[1][1], c[2][1], c[3][1]]),
                f32x4::from_array([c[0][2], c[1][2], c[2][2], c[3][2]]),
                f32x4::from_array([c[0][3], c[1][3], c[2][3], c[3][3]]),
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
        let a2313 = c[2][1] * c[3][3] - c[3][1] * c[2][3];
        let a1313 = c[1][1] * c[3][3] - c[3][1] * c[1][3];
        let a1213 = c[1][1] * c[2][3] - c[2][1] * c[1][3];
        let a2312 = c[2][1] * c[3][2] - c[3][1] * c[2][2];
        let a1312 = c[1][1] * c[3][2] - c[3][1] * c[1][2];
        let a1212 = c[1][1] * c[2][2] - c[2][1] * c[1][2];
        let a0313 = c[0][1] * c[3][3] - c[3][1] * c[0][3];
        let a0213 = c[0][1] * c[2][3] - c[2][1] * c[0][3];
        let a0312 = c[0][1] * c[3][2] - c[3][1] * c[0][2];
        let a0212 = c[0][1] * c[2][2] - c[2][1] * c[0][2];
        let a0113 = c[0][1] * c[1][3] - c[1][1] * c[0][3];
        let a0112 = c[0][1] * c[1][2] - c[1][1] * c[0][2];
        let det = f32x4::splat(
            c[0][0] * (c[1][1] * a2323 - c[2][1] * a1323 + c[3][1] * a1223)
                - c[1][0] * (c[0][1] * a2323 - c[2][1] * a0323 + c[3][1] * a0223)
                + c[2][0] * (c[0][1] * a1323 - c[1][1] * a0323 + c[3][1] * a0123)
                - c[3][0] * (c[0][1] * a1223 - c[1][1] * a0223 + c[2][1] * a0123),
        );
        return Self {
            columns: [
                f32x4::from_array([
                    (c[1][1] * a2323 - c[2][1] * a1323 + c[3][1] * a1223),
                    -(c[0][1] * a2323 - c[2][1] * a0323 + c[3][1] * a0223),
                    (c[0][1] * a1323 - c[1][1] * a0323 + c[3][1] * a0123),
                    -(c[0][1] * a1223 - c[1][1] * a0223 + c[2][1] * a0123),
                ]) / det,
                f32x4::from_array([
                    -(c[1][0] * a2323 - c[2][0] * a1323 + c[3][0] * a1223),
                    (c[0][0] * a2323 - c[2][0] * a0323 + c[3][0] * a0223),
                    -(c[0][0] * a1323 - c[1][0] * a0323 + c[3][0] * a0123),
                    (c[0][0] * a1223 - c[1][0] * a0223 + c[2][0] * a0123),
                ]) / det,
                f32x4::from_array([
                    (c[1][0] * a2313 - c[2][0] * a1313 + c[3][0] * a1213),
                    -(c[0][0] * a2313 - c[2][0] * a0313 + c[3][0] * a0213),
                    (c[0][0] * a1313 - c[1][0] * a0313 + c[3][0] * a0113),
                    -(c[0][0] * a1213 - c[1][0] * a0213 + c[2][0] * a0113),
                ]) / det,
                f32x4::from_array([
                    -(c[1][0] * a2312 - c[2][0] * a1312 + c[3][0] * a1212),
                    (c[0][0] * a2312 - c[2][0] * a0312 + c[3][0] * a0212),
                    -(c[0][0] * a1312 - c[1][0] * a0312 + c[3][0] * a0112),
                    (c[0][0] * a1212 - c[1][0] * a0212 + c[2][0] * a0112),
                ]) / det,
            ],
        };
    }

    #[inline]
    pub fn zero_translate(&self) -> Self {
        let mut other = self.clone();
        other.columns[3][0] = 0.;
        other.columns[3][1] = 0.;
        other.columns[3][2] = 0.;
        other
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
    pub fn row<const N: usize>(&self) -> f32x4 {
        f32x4::from_array([
            self.columns[0][N],
            self.columns[1][N],
            self.columns[2][N],
            self.columns[3][N],
        ])
    }

    /// Returns 3x3 matrix that is Metal float3x3 memory layout compatible.
    /// According to the specification, a float3x3 is...
    ///     - column major layout
    ///     - 48 bytes (3 columns of *4* floats)
    ///
    /// See https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf
    #[inline]
    pub fn metal_float3x3_upper_left(&self) -> [f32; 12] {
        // TODO: Create a metal_float3x3 type
        [
            self.columns[0][0],
            self.columns[0][1],
            self.columns[0][2],
            0.,
            self.columns[1][0],
            self.columns[1][1],
            self.columns[1][2],
            0.,
            self.columns[2][0],
            self.columns[2][1],
            self.columns[2][2],
            0.,
        ]
    }

    /// Returns 4x4 matrix that is Metal float4x4 memory layout compatible.
    /// According to the specification, a float4x4 is...
    ///     - column major layout
    ///     - 64 bytes: 4 columns of 4 floats
    ///
    /// See https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf
    #[inline]
    pub fn metal_float4x4(&self) -> &[f32x4; 4] {
        &self.columns
    }
}

impl Mul<f32x4> for f32x4x4 {
    type Output = f32x4;

    #[inline]
    fn mul(self, rhs: f32x4) -> Self::Output {
        f32x4::from_array(self.transpose().columns.map(|r| dot(r, rhs)))
    }
}

// TODO: Figure out how to make this a `impl const`.
// - Start by looking at how to make dot() a `const fn`.
impl Mul<f32x4x4> for f32x4x4 {
    type Output = f32x4x4;

    #[inline]
    fn mul(self, rhs: f32x4x4) -> Self::Output {
        Self {
            columns: rhs.columns.map(|col| {
                f32x4::from_array([
                    dot(self.row::<0>(), col),
                    dot(self.row::<1>(), col),
                    dot(self.row::<2>(), col),
                    dot(self.row::<3>(), col),
                ])
            }),
        }
    }
}

impl Sub<f32x4x4> for f32x4x4 {
    type Output = f32x4x4;

    #[inline]
    fn sub(self, rhs: f32x4x4) -> Self::Output {
        let columns = self.columns.zip(rhs.columns).map(|(l, r)| l - r);
        Self { columns }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::simd::f32x4;

    #[test]
    fn test_inverse() {
        let m = f32x4x4::rotate(1., 2., 3.) * f32x4x4::translate(40., 50., 60.);
        let inv_m = m.inverse();

        let expected = f32x4x4::identity();
        let actual = inv_m * m;
        let diff = actual - expected;

        const TOLERANCE: f32x4 = f32x4::splat(3.82e-6);
        for c in diff.columns {
            assert!(c.abs().lanes_lt(TOLERANCE).all());
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
        assert_eq!(
            result,
            f32x4x4::new(
                [
                    (left.row::<0>() * right.columns[0]).reduce_sum(),
                    (left.row::<0>() * right.columns[1]).reduce_sum(),
                    (left.row::<0>() * right.columns[2]).reduce_sum(),
                    (left.row::<0>() * right.columns[3]).reduce_sum(),
                ],
                [
                    (left.row::<1>() * right.columns[0]).reduce_sum(),
                    (left.row::<1>() * right.columns[1]).reduce_sum(),
                    (left.row::<1>() * right.columns[2]).reduce_sum(),
                    (left.row::<1>() * right.columns[3]).reduce_sum(),
                ],
                [
                    (left.row::<2>() * right.columns[0]).reduce_sum(),
                    (left.row::<2>() * right.columns[1]).reduce_sum(),
                    (left.row::<2>() * right.columns[2]).reduce_sum(),
                    (left.row::<2>() * right.columns[3]).reduce_sum(),
                ],
                [
                    (left.row::<3>() * right.columns[0]).reduce_sum(),
                    (left.row::<3>() * right.columns[1]).reduce_sum(),
                    (left.row::<3>() * right.columns[2]).reduce_sum(),
                    (left.row::<3>() * right.columns[3]).reduce_sum(),
                ]
            )
        );
    }
}
