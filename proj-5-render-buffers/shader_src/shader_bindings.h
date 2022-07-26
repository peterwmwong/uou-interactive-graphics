#pragma once

// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

enum struct TextureFilterMode: unsigned char
{
    Nearest = 0,
    Linear,
    Mipmap,
    Anistropic
};
