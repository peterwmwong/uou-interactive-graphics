#![allow(non_snake_case)]
/* automatically generated by rust-bindgen 0.60.1 */

#[repr(C)]
#[repr(align(8))]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct float2 {
    pub xy: [f32; 2usize],
}
#[test]
fn bindgen_test_layout_float2() {
    assert_eq!(
        ::std::mem::size_of::<float2>(),
        8usize,
        concat!("Size of: ", stringify!(float2))
    );
    assert_eq!(
        ::std::mem::align_of::<float2>(),
        8usize,
        concat!("Alignment of ", stringify!(float2))
    );
    fn test_field_xy() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<float2>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xy) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(float2),
                "::",
                stringify!(xy)
            )
        );
    }
    test_field_xy();
}
#[repr(C)]
#[repr(align(16))]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct float4 {
    pub xyzw: [f32; 4usize],
}
#[test]
fn bindgen_test_layout_float4() {
    assert_eq!(
        ::std::mem::size_of::<float4>(),
        16usize,
        concat!("Size of: ", stringify!(float4))
    );
    assert_eq!(
        ::std::mem::align_of::<float4>(),
        16usize,
        concat!("Alignment of ", stringify!(float4))
    );
    fn test_field_xyzw() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<float4>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xyzw) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(float4),
                "::",
                stringify!(xyzw)
            )
        );
    }
    test_field_xyzw();
}
#[repr(C)]
#[repr(align(4))]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct half2 {
    pub xy: [::std::os::raw::c_ushort; 2usize],
}
#[test]
fn bindgen_test_layout_half2() {
    assert_eq!(
        ::std::mem::size_of::<half2>(),
        4usize,
        concat!("Size of: ", stringify!(half2))
    );
    assert_eq!(
        ::std::mem::align_of::<half2>(),
        4usize,
        concat!("Alignment of ", stringify!(half2))
    );
    fn test_field_xy() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<half2>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xy) as usize - ptr as usize
            },
            0usize,
            concat!("Offset of field: ", stringify!(half2), "::", stringify!(xy))
        );
    }
    test_field_xy();
}
#[repr(C)]
#[repr(align(8))]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct half3 {
    pub xyz: [::std::os::raw::c_ushort; 3usize],
}
#[test]
fn bindgen_test_layout_half3() {
    assert_eq!(
        ::std::mem::size_of::<half3>(),
        8usize,
        concat!("Size of: ", stringify!(half3))
    );
    assert_eq!(
        ::std::mem::align_of::<half3>(),
        8usize,
        concat!("Alignment of ", stringify!(half3))
    );
    fn test_field_xyz() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<half3>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xyz) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(half3),
                "::",
                stringify!(xyz)
            )
        );
    }
    test_field_xyz();
}
#[repr(C)]
#[repr(align(8))]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct half4 {
    pub xyzw: [::std::os::raw::c_ushort; 4usize],
}
#[test]
fn bindgen_test_layout_half4() {
    assert_eq!(
        ::std::mem::size_of::<half4>(),
        8usize,
        concat!("Size of: ", stringify!(half4))
    );
    assert_eq!(
        ::std::mem::align_of::<half4>(),
        8usize,
        concat!("Alignment of ", stringify!(half4))
    );
    fn test_field_xyzw() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<half4>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xyzw) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(half4),
                "::",
                stringify!(xyzw)
            )
        );
    }
    test_field_xyzw();
}
#[repr(C)]
#[repr(align(4))]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct ushort2 {
    pub xy: [::std::os::raw::c_ushort; 2usize],
}
#[test]
fn bindgen_test_layout_ushort2() {
    assert_eq!(
        ::std::mem::size_of::<ushort2>(),
        4usize,
        concat!("Size of: ", stringify!(ushort2))
    );
    assert_eq!(
        ::std::mem::align_of::<ushort2>(),
        4usize,
        concat!("Alignment of ", stringify!(ushort2))
    );
    fn test_field_xy() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ushort2>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xy) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(ushort2),
                "::",
                stringify!(xy)
            )
        );
    }
    test_field_xy();
}
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct packed_float2 {
    pub xy: [f32; 2usize],
}
#[test]
fn bindgen_test_layout_packed_float2() {
    assert_eq!(
        ::std::mem::size_of::<packed_float2>(),
        8usize,
        concat!("Size of: ", stringify!(packed_float2))
    );
    assert_eq!(
        ::std::mem::align_of::<packed_float2>(),
        4usize,
        concat!("Alignment of ", stringify!(packed_float2))
    );
    fn test_field_xy() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<packed_float2>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xy) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(packed_float2),
                "::",
                stringify!(xy)
            )
        );
    }
    test_field_xy();
}
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct packed_float3 {
    pub xyz: [f32; 3usize],
}
#[test]
fn bindgen_test_layout_packed_float3() {
    assert_eq!(
        ::std::mem::size_of::<packed_float3>(),
        12usize,
        concat!("Size of: ", stringify!(packed_float3))
    );
    assert_eq!(
        ::std::mem::align_of::<packed_float3>(),
        4usize,
        concat!("Alignment of ", stringify!(packed_float3))
    );
    fn test_field_xyz() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<packed_float3>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xyz) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(packed_float3),
                "::",
                stringify!(xyz)
            )
        );
    }
    test_field_xyz();
}
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct packed_float4 {
    pub xyzw: [f32; 4usize],
}
#[test]
fn bindgen_test_layout_packed_float4() {
    assert_eq!(
        ::std::mem::size_of::<packed_float4>(),
        16usize,
        concat!("Size of: ", stringify!(packed_float4))
    );
    assert_eq!(
        ::std::mem::align_of::<packed_float4>(),
        4usize,
        concat!("Alignment of ", stringify!(packed_float4))
    );
    fn test_field_xyzw() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<packed_float4>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xyzw) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(packed_float4),
                "::",
                stringify!(xyzw)
            )
        );
    }
    test_field_xyzw();
}
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct packed_half2 {
    pub xy: [::std::os::raw::c_ushort; 2usize],
}
#[test]
fn bindgen_test_layout_packed_half2() {
    assert_eq!(
        ::std::mem::size_of::<packed_half2>(),
        4usize,
        concat!("Size of: ", stringify!(packed_half2))
    );
    assert_eq!(
        ::std::mem::align_of::<packed_half2>(),
        2usize,
        concat!("Alignment of ", stringify!(packed_half2))
    );
    fn test_field_xy() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<packed_half2>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xy) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(packed_half2),
                "::",
                stringify!(xy)
            )
        );
    }
    test_field_xy();
}
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct packed_half3 {
    pub xyz: [::std::os::raw::c_ushort; 3usize],
}
#[test]
fn bindgen_test_layout_packed_half3() {
    assert_eq!(
        ::std::mem::size_of::<packed_half3>(),
        6usize,
        concat!("Size of: ", stringify!(packed_half3))
    );
    assert_eq!(
        ::std::mem::align_of::<packed_half3>(),
        2usize,
        concat!("Alignment of ", stringify!(packed_half3))
    );
    fn test_field_xyz() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<packed_half3>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xyz) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(packed_half3),
                "::",
                stringify!(xyz)
            )
        );
    }
    test_field_xyz();
}
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct packed_half4 {
    pub xyzw: [::std::os::raw::c_ushort; 4usize],
}
#[test]
fn bindgen_test_layout_packed_half4() {
    assert_eq!(
        ::std::mem::size_of::<packed_half4>(),
        8usize,
        concat!("Size of: ", stringify!(packed_half4))
    );
    assert_eq!(
        ::std::mem::align_of::<packed_half4>(),
        2usize,
        concat!("Alignment of ", stringify!(packed_half4))
    );
    fn test_field_xyzw() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<packed_half4>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xyzw) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(packed_half4),
                "::",
                stringify!(xyzw)
            )
        );
    }
    test_field_xyzw();
}
#[repr(C)]
#[repr(align(16))]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct float3x3 {
    pub columns: [[f32; 4usize]; 3usize],
}
#[test]
fn bindgen_test_layout_float3x3() {
    assert_eq!(
        ::std::mem::size_of::<float3x3>(),
        48usize,
        concat!("Size of: ", stringify!(float3x3))
    );
    assert_eq!(
        ::std::mem::align_of::<float3x3>(),
        16usize,
        concat!("Alignment of ", stringify!(float3x3))
    );
    fn test_field_columns() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<float3x3>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).columns) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(float3x3),
                "::",
                stringify!(columns)
            )
        );
    }
    test_field_columns();
}
#[repr(C)]
#[repr(align(16))]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct float4x3 {
    pub columns: [[f32; 4usize]; 4usize],
}
#[test]
fn bindgen_test_layout_float4x3() {
    assert_eq!(
        ::std::mem::size_of::<float4x3>(),
        64usize,
        concat!("Size of: ", stringify!(float4x3))
    );
    assert_eq!(
        ::std::mem::align_of::<float4x3>(),
        16usize,
        concat!("Alignment of ", stringify!(float4x3))
    );
    fn test_field_columns() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<float4x3>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).columns) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(float4x3),
                "::",
                stringify!(columns)
            )
        );
    }
    test_field_columns();
}
#[repr(C)]
#[repr(align(16))]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct float4x4 {
    pub columns: [[f32; 4usize]; 4usize],
}
#[test]
fn bindgen_test_layout_float4x4() {
    assert_eq!(
        ::std::mem::size_of::<float4x4>(),
        64usize,
        concat!("Size of: ", stringify!(float4x4))
    );
    assert_eq!(
        ::std::mem::align_of::<float4x4>(),
        16usize,
        concat!("Alignment of ", stringify!(float4x4))
    );
    fn test_field_columns() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<float4x4>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).columns) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(float4x4),
                "::",
                stringify!(columns)
            )
        );
    }
    test_field_columns();
}
#[repr(C)]
#[repr(align(8))]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct half3x3 {
    pub columns: [[::std::os::raw::c_ushort; 4usize]; 3usize],
}
#[test]
fn bindgen_test_layout_half3x3() {
    assert_eq!(
        ::std::mem::size_of::<half3x3>(),
        24usize,
        concat!("Size of: ", stringify!(half3x3))
    );
    assert_eq!(
        ::std::mem::align_of::<half3x3>(),
        8usize,
        concat!("Alignment of ", stringify!(half3x3))
    );
    fn test_field_columns() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<half3x3>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).columns) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(half3x3),
                "::",
                stringify!(columns)
            )
        );
    }
    test_field_columns();
}
pub const DEBUG_PATH_MAX_NUM_POINTS: ::std::os::raw::c_uint = 8;
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct DebugPath {
    pub points: [packed_float3; 8usize],
    pub screen_pos: float2,
    pub num_points: ::std::os::raw::c_uchar,
}
#[test]
fn bindgen_test_layout_DebugPath() {
    assert_eq!(
        ::std::mem::size_of::<DebugPath>(),
        112usize,
        concat!("Size of: ", stringify!(DebugPath))
    );
    assert_eq!(
        ::std::mem::align_of::<DebugPath>(),
        8usize,
        concat!("Alignment of ", stringify!(DebugPath))
    );
    fn test_field_points() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<DebugPath>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).points) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(DebugPath),
                "::",
                stringify!(points)
            )
        );
    }
    test_field_points();
    fn test_field_screen_pos() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<DebugPath>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).screen_pos) as usize - ptr as usize
            },
            96usize,
            concat!(
                "Offset of field: ",
                stringify!(DebugPath),
                "::",
                stringify!(screen_pos)
            )
        );
    }
    test_field_screen_pos();
    fn test_field_num_points() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<DebugPath>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).num_points) as usize - ptr as usize
            },
            104usize,
            concat!(
                "Offset of field: ",
                stringify!(DebugPath),
                "::",
                stringify!(num_points)
            )
        );
    }
    test_field_num_points();
}
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct GeometryNoTxCoords {
    pub indices: ::std::os::raw::c_ulong,
    pub positions: ::std::os::raw::c_ulong,
    pub normals: ::std::os::raw::c_ulong,
}
#[test]
fn bindgen_test_layout_GeometryNoTxCoords() {
    assert_eq!(
        ::std::mem::size_of::<GeometryNoTxCoords>(),
        24usize,
        concat!("Size of: ", stringify!(GeometryNoTxCoords))
    );
    assert_eq!(
        ::std::mem::align_of::<GeometryNoTxCoords>(),
        8usize,
        concat!("Alignment of ", stringify!(GeometryNoTxCoords))
    );
    fn test_field_indices() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<GeometryNoTxCoords>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).indices) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(GeometryNoTxCoords),
                "::",
                stringify!(indices)
            )
        );
    }
    test_field_indices();
    fn test_field_positions() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<GeometryNoTxCoords>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).positions) as usize - ptr as usize
            },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(GeometryNoTxCoords),
                "::",
                stringify!(positions)
            )
        );
    }
    test_field_positions();
    fn test_field_normals() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<GeometryNoTxCoords>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).normals) as usize - ptr as usize
            },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(GeometryNoTxCoords),
                "::",
                stringify!(normals)
            )
        );
    }
    test_field_normals();
}
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct Geometry {
    pub indices: ::std::os::raw::c_ulong,
    pub positions: ::std::os::raw::c_ulong,
    pub normals: ::std::os::raw::c_ulong,
    pub tx_coords: ::std::os::raw::c_ulong,
}
#[test]
fn bindgen_test_layout_Geometry() {
    assert_eq!(
        ::std::mem::size_of::<Geometry>(),
        32usize,
        concat!("Size of: ", stringify!(Geometry))
    );
    assert_eq!(
        ::std::mem::align_of::<Geometry>(),
        8usize,
        concat!("Alignment of ", stringify!(Geometry))
    );
    fn test_field_indices() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Geometry>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).indices) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(Geometry),
                "::",
                stringify!(indices)
            )
        );
    }
    test_field_indices();
    fn test_field_positions() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Geometry>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).positions) as usize - ptr as usize
            },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(Geometry),
                "::",
                stringify!(positions)
            )
        );
    }
    test_field_positions();
    fn test_field_normals() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Geometry>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).normals) as usize - ptr as usize
            },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(Geometry),
                "::",
                stringify!(normals)
            )
        );
    }
    test_field_normals();
    fn test_field_tx_coords() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Geometry>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).tx_coords) as usize - ptr as usize
            },
            24usize,
            concat!(
                "Offset of field: ",
                stringify!(Geometry),
                "::",
                stringify!(tx_coords)
            )
        );
    }
    test_field_tx_coords();
}
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct Material {
    pub ambient_texture: ::std::os::raw::c_ulong,
    pub diffuse_texture: ::std::os::raw::c_ulong,
    pub specular_texture: ::std::os::raw::c_ulong,
    pub specular_shineness: f32,
    pub ambient_amount: f32,
}
#[test]
fn bindgen_test_layout_Material() {
    assert_eq!(
        ::std::mem::size_of::<Material>(),
        32usize,
        concat!("Size of: ", stringify!(Material))
    );
    assert_eq!(
        ::std::mem::align_of::<Material>(),
        8usize,
        concat!("Alignment of ", stringify!(Material))
    );
    fn test_field_ambient_texture() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Material>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).ambient_texture) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(Material),
                "::",
                stringify!(ambient_texture)
            )
        );
    }
    test_field_ambient_texture();
    fn test_field_diffuse_texture() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Material>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).diffuse_texture) as usize - ptr as usize
            },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(Material),
                "::",
                stringify!(diffuse_texture)
            )
        );
    }
    test_field_diffuse_texture();
    fn test_field_specular_texture() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Material>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).specular_texture) as usize - ptr as usize
            },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(Material),
                "::",
                stringify!(specular_texture)
            )
        );
    }
    test_field_specular_texture();
    fn test_field_specular_shineness() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Material>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).specular_shineness) as usize - ptr as usize
            },
            24usize,
            concat!(
                "Offset of field: ",
                stringify!(Material),
                "::",
                stringify!(specular_shineness)
            )
        );
    }
    test_field_specular_shineness();
    fn test_field_ambient_amount() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<Material>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).ambient_amount) as usize - ptr as usize
            },
            28usize,
            concat!(
                "Offset of field: ",
                stringify!(Material),
                "::",
                stringify!(ambient_amount)
            )
        );
    }
    test_field_ambient_amount();
}
#[repr(C)]
#[repr(align(16))]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct ModelSpace {
    pub m_model_to_projection: float4x4,
    pub m_normal_to_world: float3x3,
}
#[test]
fn bindgen_test_layout_ModelSpace() {
    assert_eq!(
        ::std::mem::size_of::<ModelSpace>(),
        112usize,
        concat!("Size of: ", stringify!(ModelSpace))
    );
    assert_eq!(
        ::std::mem::align_of::<ModelSpace>(),
        16usize,
        concat!("Alignment of ", stringify!(ModelSpace))
    );
    fn test_field_m_model_to_projection() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ModelSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).m_model_to_projection) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(ModelSpace),
                "::",
                stringify!(m_model_to_projection)
            )
        );
    }
    test_field_m_model_to_projection();
    fn test_field_m_normal_to_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ModelSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).m_normal_to_world) as usize - ptr as usize
            },
            64usize,
            concat!(
                "Offset of field: ",
                stringify!(ModelSpace),
                "::",
                stringify!(m_normal_to_world)
            )
        );
    }
    test_field_m_normal_to_world();
}
#[repr(C)]
#[repr(align(16))]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct ProjectedSpace {
    pub m_world_to_projection: float4x4,
    pub m_screen_to_world: float4x4,
    pub position_world: float4,
}
#[test]
fn bindgen_test_layout_ProjectedSpace() {
    assert_eq!(
        ::std::mem::size_of::<ProjectedSpace>(),
        144usize,
        concat!("Size of: ", stringify!(ProjectedSpace))
    );
    assert_eq!(
        ::std::mem::align_of::<ProjectedSpace>(),
        16usize,
        concat!("Alignment of ", stringify!(ProjectedSpace))
    );
    fn test_field_m_world_to_projection() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ProjectedSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).m_world_to_projection) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(ProjectedSpace),
                "::",
                stringify!(m_world_to_projection)
            )
        );
    }
    test_field_m_world_to_projection();
    fn test_field_m_screen_to_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ProjectedSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).m_screen_to_world) as usize - ptr as usize
            },
            64usize,
            concat!(
                "Offset of field: ",
                stringify!(ProjectedSpace),
                "::",
                stringify!(m_screen_to_world)
            )
        );
    }
    test_field_m_screen_to_world();
    fn test_field_position_world() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<ProjectedSpace>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).position_world) as usize - ptr as usize
            },
            128usize,
            concat!(
                "Offset of field: ",
                stringify!(ProjectedSpace),
                "::",
                stringify!(position_world)
            )
        );
    }
    test_field_position_world();
}
#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct TriNormals {
    pub normals: [::std::os::raw::c_uint; 2usize],
}
#[test]
fn bindgen_test_layout_TriNormals() {
    assert_eq!(
        ::std::mem::size_of::<TriNormals>(),
        8usize,
        concat!("Size of: ", stringify!(TriNormals))
    );
    assert_eq!(
        ::std::mem::align_of::<TriNormals>(),
        4usize,
        concat!("Alignment of ", stringify!(TriNormals))
    );
    fn test_field_normals() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<TriNormals>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).normals) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(TriNormals),
                "::",
                stringify!(normals)
            )
        );
    }
    test_field_normals();
}

#[test]
fn test_metal_types_derive_copy() {
    use std::marker::PhantomData;
    struct HasCopyClone<T: Sized + Copy + Clone>(PhantomData<T>);
    HasCopyClone(PhantomData::<DebugPath>);
    HasCopyClone(PhantomData::<Geometry>);
    HasCopyClone(PhantomData::<GeometryNoTxCoords>);
    HasCopyClone(PhantomData::<Material>);
    HasCopyClone(PhantomData::<ModelSpace>);
    HasCopyClone(PhantomData::<ProjectedSpace>);
    HasCopyClone(PhantomData::<TriNormals>);
    HasCopyClone(PhantomData::<float2>);
    HasCopyClone(PhantomData::<float3x3>);
    HasCopyClone(PhantomData::<float4>);
    HasCopyClone(PhantomData::<float4x3>);
    HasCopyClone(PhantomData::<float4x4>);
    HasCopyClone(PhantomData::<half2>);
    HasCopyClone(PhantomData::<half3>);
    HasCopyClone(PhantomData::<half3x3>);
    HasCopyClone(PhantomData::<half4>);
    HasCopyClone(PhantomData::<packed_float2>);
    HasCopyClone(PhantomData::<packed_float3>);
    HasCopyClone(PhantomData::<packed_float4>);
    HasCopyClone(PhantomData::<packed_half2>);
    HasCopyClone(PhantomData::<packed_half3>);
    HasCopyClone(PhantomData::<packed_half4>);
    HasCopyClone(PhantomData::<ushort2>);
}