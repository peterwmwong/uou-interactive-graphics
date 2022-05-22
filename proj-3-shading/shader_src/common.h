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
    VertexBufferIndexMatrixModelToProjection,
    VertexBufferIndexMatrixNormalToWorld,
    VertexBufferIndexLENGTH
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
    FragBufferIndexMatrixProjectionToWorld,
    FragBufferIndexScreenSize,
    FragBufferIndexLightPosition,
    FragBufferIndexCameraPosition,
    FragBufferIndexLENGTH
};

enum LightVertexBufferIndex
{
    LightVertexBufferIndexMatrixWorldToProjection = 0,
    LightVertexBufferIndexLightPosition,
};

#endif