// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

enum struct FC
{
    HasAmbient = 0,
    HasDiffuse,
    HasNormal,
    HasSpecular,
};

enum struct GeometryID
{
    Indices = 0,
    Positions,
    Normals,
    TXCoords,
};

enum struct WorldID
{
    MatrixModelToProjection = 0,
    MatrixNormalToWorld,
    MatrixWorldToProjection,
    MatrixScreenToWorld,
    LightPosition,
    CameraPosition,
};

enum struct MaterialID
{
    AmbientTexture = 0,
    DiffuseTexture,
    SpecularTexture,
    SpecularShineness,
};

enum struct VertexBufferIndex
{
    Geometry = 0,
    World,
    LENGTH
};

enum struct FragBufferIndex
{
    Material = 0,
    World,
    LENGTH
};

enum struct LightVertexBufferIndex
{
    World = 0,
    LENGTH,
};

#endif