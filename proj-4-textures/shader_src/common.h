// Header containing types and enum struct constants shared between Metal shaders and Rust source code
//
// These are used to generate Rust types in the `build.rs` build script.
#ifndef common_h
#define common_h

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// CamelCase enum variants to appease clippy
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

enum struct WorldID
{
    matrix_model_to_projection = 0,
    matrix_normal_to_world,
    matrix_world_to_projection,
    matrix_screen_to_world,
    light_position,
    camera_position,
};

enum struct MaterialID
{
    ambient_texture = 0,
    diffuse_texture,
    specular_texture,
    specular_shineness,
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