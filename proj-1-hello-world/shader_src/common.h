// Header containing types and enum constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.

#ifndef __METAL_VERSION__
#include "./rust-bindgen-only-vector-types.h"
#endif

#ifndef common_h
#define common_h

enum VertexFuncBufferIndex
{
    VertexFuncBufferIndexScreenSize = 0,
    VertexFuncBufferIndexRects
};

enum FragmentFuncBufferIndex
{
    FragmentFuncBufferIndexRects = 0
};

enum AttachmentIndex
{
    AttachmentIndexColor = 0,
    AttachmentIndexComponentId
};

#endif