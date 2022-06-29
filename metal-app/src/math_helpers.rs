use std::simd::u32x2;

#[inline]
pub fn round_up_pow_of_2(mut v: u32x2) -> u32x2 {
    v -= u32x2::splat(1);
    v |= v >> u32x2::splat(1);
    v |= v >> u32x2::splat(2);
    v |= v >> u32x2::splat(4);
    v |= v >> u32x2::splat(8);
    v |= v >> u32x2::splat(16);
    v + u32x2::splat(1)
}
