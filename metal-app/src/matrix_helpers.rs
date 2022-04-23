use std::ops::Mul;
use std::simd::f32x4;

fn dot_product(lhs: f32x4, rhs: f32x4) -> f32 {
    lhs.mul(rhs).reduce_sum()
}

#[derive(Copy, Clone, PartialEq)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[allow(non_camel_case_types)]
pub struct f32x4x4 {
    rows: [f32x4; 4],
}

impl f32x4x4 {
    #[inline]
    pub fn new(row1: [f32; 4], row2: [f32; 4], row3: [f32; 4], row4: [f32; 4]) -> Self {
        f32x4x4 {
            rows: [
                f32x4::from_array(row1),
                f32x4::from_array(row2),
                f32x4::from_array(row3),
                f32x4::from_array(row4),
            ],
        }
    }

    #[inline]
    pub fn scale(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self::new(
            [x, 0., 0., 0.],
            [0., y, 0., 0.],
            [0., 0., z, 0.],
            [0., 0., 0., w],
        )
    }

    #[inline]
    pub fn identity() -> Self {
        Self::scale(1., 1., 1., 1.)
    }

    #[inline]
    pub fn column<const N: usize>(&self) -> f32x4 {
        f32x4::from_array([
            self.rows[0][N],
            self.rows[0][N],
            self.rows[0][N],
            self.rows[0][N],
        ])
    }
}

impl Mul<f32x4> for f32x4x4 {
    type Output = f32x4;

    #[inline]
    fn mul(self, rhs: f32x4) -> Self::Output {
        f32x4::from_array(self.rows.map(|r| dot_product(r, rhs)))
    }
}

impl Mul<f32x4x4> for f32x4x4 {
    type Output = f32x4x4;

    #[inline]
    fn mul(self, rhs: f32x4x4) -> Self::Output {
        Self {
            rows: self.rows.map(|r| {
                f32x4::from_array([
                    dot_product(r, rhs.column::<0>()),
                    dot_product(r, rhs.column::<1>()),
                    dot_product(r, rhs.column::<2>()),
                    dot_product(r, rhs.column::<3>()),
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
                    (left.rows[0] * right.column::<0>()).reduce_sum(),
                    (left.rows[0] * right.column::<1>()).reduce_sum(),
                    (left.rows[0] * right.column::<2>()).reduce_sum(),
                    (left.rows[0] * right.column::<3>()).reduce_sum(),
                ],
                [
                    (left.rows[1] * right.column::<0>()).reduce_sum(),
                    (left.rows[1] * right.column::<1>()).reduce_sum(),
                    (left.rows[1] * right.column::<2>()).reduce_sum(),
                    (left.rows[1] * right.column::<3>()).reduce_sum(),
                ],
                [
                    (left.rows[2] * right.column::<0>()).reduce_sum(),
                    (left.rows[2] * right.column::<1>()).reduce_sum(),
                    (left.rows[2] * right.column::<2>()).reduce_sum(),
                    (left.rows[2] * right.column::<3>()).reduce_sum(),
                ],
                [
                    (left.rows[3] * right.column::<0>()).reduce_sum(),
                    (left.rows[3] * right.column::<1>()).reduce_sum(),
                    (left.rows[3] * right.column::<2>()).reduce_sum(),
                    (left.rows[3] * right.column::<3>()).reduce_sum(),
                ]
            )
        );
    }
}
