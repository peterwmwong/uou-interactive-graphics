use std::ops::Mul;
use std::simd::f32x4;

#[inline]
fn dot_product(lhs: f32x4, rhs: f32x4) -> f32 {
    lhs.mul(rhs).reduce_sum()
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
}

impl Mul<f32x4> for f32x4x4 {
    type Output = f32x4;

    #[inline]
    fn mul(self, rhs: f32x4) -> Self::Output {
        f32x4::from_array(self.transpose().columns.map(|r| dot_product(r, rhs)))
    }
}

// TODO: Figure out how to make this a `impl const`.
// - Start by looking at how to make dot_product() a `const fn`.
impl Mul<f32x4x4> for f32x4x4 {
    type Output = f32x4x4;

    #[inline]
    fn mul(self, rhs: f32x4x4) -> Self::Output {
        Self {
            columns: rhs.columns.map(|col| {
                f32x4::from_array([
                    dot_product(self.row::<0>(), col),
                    dot_product(self.row::<1>(), col),
                    dot_product(self.row::<2>(), col),
                    dot_product(self.row::<3>(), col),
                ])
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use std::simd::f32x4;

    use super::*;

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
