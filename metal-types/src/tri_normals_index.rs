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
    use crate::packed_half3;
    use metal::*;

    const COS_PI_4: f32 = 0.7071067811865476;
    const SIN_PI_4: f32 = 0.7071067811865475;

    const TEST_COMPUTE_SRC: &'static str = concat!(
        r#"
#include <metal_stdlib>
using namespace metal;
"#,
        include_str!("./tri_normals_index.h"),
        r#"
[[kernel]]
void test(
    constant packed_float3 & input  [[buffer(0)]],
    device   packed_half3  * output [[buffer(1)]]
) {
    *output = decode(input);
}
"#
    );

    #[test]
    pub fn encode_rs_decode_metal() {
        let device = Device::system_default().expect("Failed to access Metal Device");
        let output_buf = device.new_buffer(
            std::mem::size_of::<packed_half3>() as _,
            MTLResourceOptions::StorageModeShared,
        );
        let lib = device
            .new_library_with_source(TEST_COMPUTE_SRC, &CompileOptions::new())
            .expect("Failed to compile test compute kernel source");
        let cmd_queue = device.new_command_queue();
        let test_fn = lib
            .get_function("test", None)
            .expect("Failed to get kernel function");
        let pipeline = device
            .new_compute_pipeline_state_with_function(&test_fn)
            .expect("Failed to get kernel function");

        let t = |input: [f32; 3]| {
            let cmd_buf = cmd_queue.new_command_buffer();
            let e = cmd_buf.new_compute_command_encoder();
            e.set_compute_pipeline_state(&pipeline);
            e.set_bytes(
                0,
                std::mem::size_of::<packed_float3>() as _,
                (&encode(&input, 0) as *const packed_float3) as *const _,
            );
            e.set_buffer(1, Some(&output_buf), 0);
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
            let actual = unsafe { &*(output_buf.contents() as *const packed_half3) };
            let diffs = [
                (input[0] - half::f16::from_bits(actual.xyz[0]).to_f32()).abs(),
                (input[1] - half::f16::from_bits(actual.xyz[1]).to_f32()).abs(),
                (input[2] - half::f16::from_bits(actual.xyz[2]).to_f32()).abs(),
            ];
            const TOLERANCE: f32 = 0.0001;
            assert!(diffs[0] < TOLERANCE);
            assert!(diffs[1] < TOLERANCE);
            assert!(diffs[2] < TOLERANCE);
        };

        t([1., 0., 0.]);
        t([0., 1., 0.]);
        t([0., 0., 1.]);

        t([-1., 0., 0.]);
        t([0., -1., 0.]);
        t([0., 0., -1.]);

        t([COS_PI_4, SIN_PI_4, 0.]);
        t([-COS_PI_4, SIN_PI_4, 0.]);
        t([COS_PI_4, -SIN_PI_4, 0.]);
        t([-COS_PI_4, -SIN_PI_4, 0.]);

        t([COS_PI_4, 0., SIN_PI_4]);
        t([-COS_PI_4, 0., SIN_PI_4]);
        t([COS_PI_4, 0., -SIN_PI_4]);
        t([-COS_PI_4, 0., -SIN_PI_4]);

        t([0., COS_PI_4, SIN_PI_4]);
        t([0., -COS_PI_4, SIN_PI_4]);
        t([0., COS_PI_4, -SIN_PI_4]);
        t([0., -COS_PI_4, -SIN_PI_4]);
    }
}
