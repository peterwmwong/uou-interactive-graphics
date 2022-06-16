// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

enum struct VertexBufferIndex
{
    MatrixModelToProjection = 0,
    LENGTH
};

enum struct FragBufferIndex
{
    Texture = 0,
    LENGTH
};

#endif