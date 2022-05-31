// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

#ifdef __METAL_VERSION__
#define DEF_CONSTANT constant
#else
#define DEF_CONSTANT
#endif

enum struct FC
{
    HAS_AMBIENT = 0,
    HAS_DIFFUSE,
    HAS_NORMAL,
    HAS_SPECULAR,
};

enum struct ObjectGeometryID
{
    indices = 0,
    positions,
    normals,
    tx_coords,
};

enum struct MaterialID
{
    diffuse_color = 0,
    specular_color,
    diffuse_texture,
    specular_texture,
    specular_shineness,
};

enum struct VertexBufferIndex
{
    ObjectGeometry = 0,
    MatrixModelToProjection,
    MatrixNormalToWorld,
    LENGTH
};

enum struct FragBufferIndex
{
    MatrixProjectionToWorld = 0,
    ScreenSize,
    LightPosition,
    CameraPosition,
    Material,
    LENGTH
};

enum struct LightVertexBufferIndex
{
    MatrixWorldToProjection = 0,
    LightPosition,
    LENGTH,
};

#endif