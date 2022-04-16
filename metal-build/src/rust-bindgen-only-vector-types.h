// Rust Bindgen Workaround: Polyfill Vector Types
#ifndef __METAL_VERSION__
#ifndef rust_bindgen_only_vector_types_h
#define rust_bindgen_only_vector_types_h

// Definitions are according to Metal Shading Language Specification (Version 2.4)
// https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf

// Spec: 2.2 Vector Data Types / Table 2.3. Size and alignment of vector data types
typedef struct alignas(4) half2
{
    __fp16 x;
    __fp16 y;
} half2;

typedef struct alignas(2) packed_half2
{
    __fp16 x;
    __fp16 y;
} packed_half2;

typedef struct alignas(8) half4
{
    __fp16 x;
    __fp16 y;
    __fp16 z;
    __fp16 w;
} half4;

typedef struct alignas(2) packed_half4
{
    __fp16 x;
    __fp16 y;
    __fp16 z;
    __fp16 w;
} packed_half4;

typedef struct alignas(8) float2
{
    float x;
    float y;
} float2;

typedef struct alignas(16) float4
{
    float x;
    float y;
    float z;
    float w;
} float4;

// Spec: 2.2 Vector Data Types / Table 2.3. Size and alignment of vector data types
typedef struct alignas(4) ushort2
{
    unsigned short x;
    unsigned short y;
} ushort2;

// Spec: 2.2.3 Packed Vector Types / Table 2.4. Size and alignment of packed vector data types
typedef struct alignas(4) packed_float2
{
    float x;
    float y;
} packed_float2;

typedef struct alignas(4) packed_float4
{
    float x;
    float y;
    float z;
    float w;
} packed_float4;

#endif // rust_bindgen_only_vector_types_h
#endif //__METAL_VERSION__