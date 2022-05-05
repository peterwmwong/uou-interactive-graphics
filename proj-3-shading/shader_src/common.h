// Header containing types and enum constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

enum VertexBufferIndex
{
    VertexBufferIndexIndices = 0,
    VertexBufferIndexPositions,
    VertexBufferIndexNormals,
    VertexBufferIndexModelViewProjection,
    VertexBufferIndexNormalTransform,
};

enum FragMode
{
    FragModeNormals = 0,
    FragModeAmbient,
    FragModeAmbientDiffuse,
    FragModeSpecular,
    FragModeAmbientDiffuseSpecular,
};

enum FragBufferIndex
{
    FragBufferIndexFragMode = 0,
    FragBufferIndexInverseProjection,
    FragBufferIndexScreenSize,
    FragBufferIndexLightDirection,
};

enum LightVertexBufferIndex
{
    LightVertexBufferIndexViewProjection = 0,
    LightVertexBufferIndexLightPosition,
};

#endif