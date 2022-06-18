// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

enum struct TextureFilterMode: unsigned char
{
    Nearest = 0,
    Linear,
    Mipmap,
    Anistropic
};

enum struct VertexBufferIndex: unsigned char
{
    MatrixModelToProjection = 0,
    LENGTH
};

enum struct FragBufferIndex: unsigned char
{
    Texture = 0,
    TextureFilterMode,
    LENGTH
};

#endif