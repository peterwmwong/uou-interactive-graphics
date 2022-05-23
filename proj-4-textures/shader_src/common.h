// Header containing types and enum constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

enum FC
{
    FC_HAS_AMBIENT = 0,
    FC_HAS_DIFFUSE,
    FC_HAS_NORMAL,
    FC_HAS_SPECULAR,
    FC_SPECULAR_SHINENESS,
};

enum VertexBufferIndex
{
    VertexBufferIndex_Indices = 0,
    VertexBufferIndex_Positions,
    VertexBufferIndex_Normals,
    VertexBufferIndex_Texcoords,
    VertexBufferIndex_MatrixModelToProjection,
    VertexBufferIndex_MatrixNormalToWorld,
    VertexBufferIndex_LENGTH
};

enum FragBufferIndex
{
    FragBufferIndex_MatrixProjectionToWorld = 0,
    FragBufferIndex_ScreenSize,
    FragBufferIndex_LightPosition,
    FragBufferIndex_CameraPosition,
    FragBufferIndex_AmbientTexture,
    FragBufferIndex_Specular,
    FragBufferIndex_LENGTH
};

enum LightVertexBufferIndex
{
    LightVertexBufferIndex_MatrixWorldToProjection = 0,
    LightVertexBufferIndex_LightPosition,
    LightVertexBufferIndex_LENGTH,
};

#endif