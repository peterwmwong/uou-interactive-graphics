// Header containing types and enum constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

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

enum FragMode
{
    FragMode_Normals = 0,
    FragMode_Ambient,
    FragMode_AmbientDiffuse,
    FragMode_Specular,
    FragMode_AmbientDiffuseSpecular,
};

enum FragBufferIndex
{
    FragBufferIndex_FragMode = 0,
    FragBufferIndex_MatrixProjectionToWorld,
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