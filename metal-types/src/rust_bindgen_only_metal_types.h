#pragma once

// Rust Bindgen Workaround: Polyfill Vector Types
#ifndef __METAL_VERSION__

// Definitions are according to Metal Shading Language Specification (Version 2.4)
// https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf

// Spec: 2.2 Vector Data Types / Table 2.3. Size and alignment of vector data types

struct alignas(8) float2 {
    float xy[2];
};

struct alignas(16) float4 {
    float xyzw[4];
};

struct alignas(4) ushort2 {
    unsigned short xy[2];
};

// Spec: 2.2.3 Packed Vector Types / Table 2.4. Size and alignment of packed vector data types

struct alignas(4) packed_float2 {
    float xy[2];
};

struct alignas(4) packed_float4 {
    float xyzw[4];
};

// Spec: 2.3 Matrix Data Types / Table 2.5. Size and alignment of matrix data types

struct alignas(16) float3x3 {
    float columns[3][4];
};

struct alignas(16) float4x4 {
    float columns[4][4];
};

#endif //__METAL_VERSION__
