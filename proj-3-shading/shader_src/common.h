// Header containing types and enum constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

#ifdef __METAL_VERSION__
#define DEF_CONSTANT constant
#else
#define DEF_CONSTANT
#endif

DEF_CONSTANT const float INITIAL_CAMERA_DISTANCE = 50.0;

enum VertexBufferIndex
{
    VertexBufferIndexIndices = 0,
    VertexBufferIndexPositions,
    VertexBufferIndexModelViewProjection,
    VertexBufferIndexScreenSize,
    VertexBufferIndexCameraRotation,
    VertexBufferIndexCameraDistance,
};

#endif