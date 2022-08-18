/* automatically generated by rust-bindgen 0.60.1 */

#[repr(C)]
#[repr(align(8))]
#[derive(Copy, Clone, PartialEq)]
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
#[derive(Copy, Clone, PartialEq)]
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
#[derive(Copy, Clone, PartialEq, Eq)]
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
#[derive(Copy, Clone, PartialEq)]
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
#[derive(Copy, Clone, PartialEq)]
pub struct packed_float3 {
    pub xyzw: [f32; 3usize],
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
    fn test_field_xyzw() {
        assert_eq!(
            unsafe {
                let uninit = ::std::mem::MaybeUninit::<packed_float3>::uninit();
                let ptr = uninit.as_ptr();
                ::std::ptr::addr_of!((*ptr).xyzw) as usize - ptr as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(packed_float3),
                "::",
                stringify!(xyzw)
            )
        );
    }
    test_field_xyzw();
}
#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
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
#[repr(align(16))]
#[derive(Copy, Clone, PartialEq)]
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
#[derive(Copy, Clone, PartialEq)]
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
#[derive(Copy, Clone, PartialEq)]
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

#[test]
fn test_metal_types_derive_copy() {
    use std::marker::PhantomData;
    struct HasCopyClone<T: Sized + Copy + Clone>(PhantomData<T>);
    HasCopyClone(PhantomData::<float2>);
    HasCopyClone(PhantomData::<float3x3>);
    HasCopyClone(PhantomData::<float4>);
    HasCopyClone(PhantomData::<float4x3>);
    HasCopyClone(PhantomData::<float4x4>);
    HasCopyClone(PhantomData::<packed_float2>);
    HasCopyClone(PhantomData::<packed_float3>);
    HasCopyClone(PhantomData::<packed_float4>);
    HasCopyClone(PhantomData::<ushort2>);
}