// Header containing types and enum constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

enum VertexBufferIndex
{
    VertexBufferIndexMaxPositionValue = 0,
    VertexBufferIndexPositions,
    VertexBufferIndexAspectRatio,
    VertexBufferIndexTime
};

#endif