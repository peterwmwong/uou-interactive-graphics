use super::all_metal_types::TriNormals;
use std::{
    ffi::c_uint,
    simd::{f32x4, SimdFloat},
};

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

#[inline]
fn unorm2(b0: bool, b1: bool) -> u32 {
    let b0 = if b0 { 1 } else { 0 };
    let b1 = if b1 { 2 } else { 0 };
    b0 | b1
}

#[inline]
fn unorm10(mut f: f32) -> u32 {
    const MAX_10BIT_VALUE: f32 = ((1u16 << 10) - 1) as f32;
    if f < 0. || f > 1. {
        println!("Expected {f} to be within 0 and 1.");
        f = f.clamp(0., 1.);
    }
    (f * MAX_10BIT_VALUE) as u32
}

#[inline]
fn unorm1010102(r: u32, g: u32, b: u32, a: u32) -> u32 {
    assert_eq!((r >> 10), 0);
    assert_eq!((g >> 10), 0);
    assert_eq!((b >> 10), 0);
    assert_eq!((a >> 2), 0);
    (a << 30) | (b << 20) | (g << 10) | r
}

#[inline]
fn compress(n0: ([f32; 2], bool), n1: ([f32; 2], bool), n2: ([f32; 2], bool)) -> [c_uint; 2] {
    let n0x = unorm10(n0.0[0]);
    let n0y = unorm10(n0.0[1]);
    let n0z = unorm2(n0.1, false);

    let n1x = unorm10(n1.0[0]);
    let n1y = unorm10(n1.0[1]);

    let n2x = unorm10(n2.0[0]);
    let n2y = unorm10(n2.0[1]);

    let n12z = unorm2(n1.1, n2.1);

    [
        unorm1010102(n0x, n0y, n2x, n0z),
        unorm1010102(n1x, n1y, n2y, n12z),
    ]
}

// http://johnwhite3d.blogspot.com/2017/10/signed-octahedron-normal-encoding.html
#[inline]
fn encode_normal(v: &[f32], i: usize) -> ([f32; 2], bool) {
    let mut n: f32x4 = dbg_ensure_unit_vector(f32x4::from_array([v[i], v[i + 1], v[i + 2], 0.]));
    // float3 OutN;

    // n /= ( abs( n.x ) + abs( n.y ) + abs( n.z ) );
    n /= f32x4::splat(n[0].abs() + n[1].abs() + n[2].abs());

    // OutN.y = n.y *  0.5  + 0.5;
    let mut ny = n[1] * 0.5 + 0.5;

    // OutN.x = n.x *  0.5 + OutN.y;
    let nx = n[0] * 0.5 + ny;

    // OutN.y = n.x * -0.5 + OutN.y;
    ny = n[0] * -0.5 + ny;

    // OutN.z = saturate(n.z*FLT_MAX);
    // return OutN;
    ([nx, ny], n[2] >= 0.0)
}

// http://johnwhite3d.blogspot.com/2017/10/signed-octahedron-normal-encoding.html
#[inline]
fn encode(v: &[f32], n0: usize, n1: usize, n2: usize) -> [c_uint; 2] {
    compress(
        encode_normal(v, n0),
        encode_normal(v, n1),
        encode_normal(v, n2),
    )
}

impl TriNormals {
    #[inline]
    pub fn from_indexed_raw_normals(
        raw_normals: &[f32],
        raw_indices: &[u32],
        start_vertex: usize,
    ) -> Self {
        Self {
            normals: encode(
                raw_normals,
                (raw_indices[start_vertex * 3] * 3) as _,
                (raw_indices[start_vertex * 3 + 1] * 3) as _,
                (raw_indices[start_vertex * 3 + 2] * 3) as _,
            ),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::packed_half3;
    use metal::*;
    use std::marker::PhantomData;

    const COS_PI_4: f32 = 0.7071067811865476;
    const SIN_PI_4: f32 = 0.7071067811865475;

    const DECOMPRESS_TOLERANCE: f32 = 0.000928;
    const DECODE_TOLERANCE: f32 = 0.003005207;

    fn assert_eq_with_tolerance(left: f32, right: u16, tolerance: f32, msg: &'static str) {
        let right = half::f16::from_bits(right).to_f32();
        let diff = (left - right).abs();
        if diff > tolerance {
            assert_eq!(left, right, "\n{msg}, diff = {diff}");
        }
    }

    fn assert_equalish(expected: &[f32], actual: &packed_half3, tolerance: f32) {
        assert_eq_with_tolerance(
            expected[0],
            actual.xyz[0],
            tolerance,
            "x component is incorrect",
        );
        assert_eq_with_tolerance(
            expected[1],
            actual.xyz[1],
            tolerance,
            "y component is incorrect",
        );
        assert_eq_with_tolerance(
            expected[2],
            actual.xyz[2],
            tolerance,
            "z component is incorrect",
        );
    }

    struct ComputeExecutor<I: Copy + Clone, O: Copy + Clone> {
        cmd_queue: CommandQueue,
        output_buf: Buffer,
        pipeline: ComputePipelineState,
        _input_type: PhantomData<I>,
        _output_type: PhantomData<O>,
    }

    impl<I: Copy + Clone, O: Copy + Clone> ComputeExecutor<I, O> {
        fn new(src: &'static str) -> Self {
            let device = Device::system_default().expect("Failed to access Metal Device");
            let output_buf = device.new_buffer(
                std::mem::size_of::<O>() as _,
                MTLResourceOptions::StorageModeShared,
            );
            let lib = device
                .new_library_with_source(src, &CompileOptions::new())
                .expect("Failed to compile test compute kernel source");
            let cmd_queue = device.new_command_queue();
            let test_fn = lib
                .get_function("test", None)
                .expect("Failed to get kernel function");
            let pipeline = device
                .new_compute_pipeline_state_with_function(&test_fn)
                .expect("Failed to get kernel function");

            Self {
                cmd_queue,
                output_buf,
                pipeline,
                _input_type: PhantomData,
                _output_type: PhantomData,
            }
        }

        fn run(&self, input: I) -> &O {
            let cmd_buf = self.cmd_queue.new_command_buffer();
            let e = cmd_buf.new_compute_command_encoder();
            e.set_compute_pipeline_state(&self.pipeline);
            e.set_bytes(
                0,
                std::mem::size_of::<I>() as _,
                (&input as *const I) as *const _,
            );
            e.set_buffer(1, Some(&self.output_buf), 0);
            e.dispatch_threads(
                MTLSize {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
                MTLSize {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
            );
            e.end_encoding();
            cmd_buf.commit();
            cmd_buf.wait_until_completed();
            unsafe { &*(self.output_buf.contents() as *const O) }
        }
    }

    #[test]
    pub fn compress_rs_decompress_metal() {
        let c: ComputeExecutor<[c_uint; 2], [packed_half3; 3]> = ComputeExecutor::new(concat!(
            r#"
#include <metal_stdlib>
using namespace metal;
"#,
            include_str!("./tri_normals.h"),
            r#"
[[kernel]]
void test(
    constant uint          * input  [[buffer(0)]],
    device   packed_half3 * output [[buffer(1)]]
) {
    auto v = decompress(input[0], input[1]);
    *(&output[0]) = v[0];
    *(&output[1]) = v[1];
    *(&output[2]) = v[2];
}
"#
        ));
        let t = |input: [([f32; 2], bool); 3], expected: [[f32; 3]; 3]| {
            let actual = c.run(compress(input[0], input[1], input[2]));
            assert_equalish(&expected[0], &actual[0], DECOMPRESS_TOLERANCE);
            assert_equalish(&expected[1], &actual[1], DECOMPRESS_TOLERANCE);
            assert_equalish(&expected[2], &actual[2], DECOMPRESS_TOLERANCE);
        };
        t(
            [([0.1, 0.3], true), ([0.2, 0.4], false), ([0.5, 0.6], true)],
            [[0.1, 0.3, 1.0], [0.2, 0.4, 0.0], [0.5, 0.6, 1.0]],
        );
    }

    #[test]
    pub fn encode_rs_decode_metal() {
        let c: ComputeExecutor<[c_uint; 2], [packed_half3; 3]> = ComputeExecutor::new(concat!(
            r#"
#include <metal_stdlib>
using namespace metal;
"#,
            include_str!("./tri_normals.h"),
            r#"
[[kernel]]
void test(
    constant uint         * input  [[buffer(0)]],
    device   packed_half3 * output [[buffer(1)]]
) {
    auto v = decode(input[0], input[1]);
    *(&output[0]) = v[0];
    *(&output[1]) = v[1];
    *(&output[2]) = v[2];
}
"#
        ));
        let t = |input: [f32; 9]| {
            let actual = c.run(encode(&input, 0, 3, 6));
            assert_equalish(&input[0..3], &actual[0], DECODE_TOLERANCE);
            assert_equalish(&input[3..6], &actual[1], DECODE_TOLERANCE);
            assert_equalish(&input[6..], &actual[2], DECODE_TOLERANCE);
        };
        t([
            1., 0., 0., // 0
            0., 1., 0., // 1
            0., 0., 1., // 2
        ]);
        t([
            -1., 0., 0., // 0
            0., -1., 0., // 1
            0., 0., -1., // 2
        ]);

        t([
            COS_PI_4, SIN_PI_4, 0., // 0
            -COS_PI_4, SIN_PI_4, 0., // 1
            COS_PI_4, -SIN_PI_4, 0., // 2
        ]);

        t([
            -COS_PI_4, -SIN_PI_4, 0., // 0
            COS_PI_4, 0., SIN_PI_4, // 1
            -COS_PI_4, 0., SIN_PI_4, // 2
        ]);

        t([
            COS_PI_4, 0., -SIN_PI_4, // 0
            -COS_PI_4, 0., -SIN_PI_4, // 1
            0., COS_PI_4, SIN_PI_4, // 2
        ]);

        t([
            0., -COS_PI_4, SIN_PI_4, // 0
            0., COS_PI_4, -SIN_PI_4, // 1
            0., -COS_PI_4, -SIN_PI_4, // 2
        ])
    }
}
