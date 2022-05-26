// Header containing types and enum constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

enum struct VertexBufferIndex
{
    Indices = 0,
    Positions,
    Normals,
    MatrixModelToProjection,
    MatrixNormalToWorld,
    LENGTH
};

enum struct FragMode
{
    Normals = 0,
    Ambient,
    AmbientDiffuse,
    Specular,
    AmbientDiffuseSpecular,
};

enum struct FragBufferIndex
{
    FragMode = 0,
    MatrixProjectionToWorld,
    ScreenSize,
    LightPosition,
    CameraPosition,
    LENGTH,
};

enum struct LightVertexBufferIndex
{
    MatrixWorldToProjection = 0,
    LightPosition,
    LENGTH,
};

#endif