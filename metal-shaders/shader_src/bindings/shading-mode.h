#pragma once

enum struct ShadingMode: unsigned char
{
    HasAmbient = 0,
    HasDiffuse,
    OnlyNormals,
    HasSpecular
};

#ifdef __METAL_VERSION__
constant constexpr bool  HasAmbient  [[function_constant(static_cast<uint>(ShadingMode::HasAmbient))]];
constant constexpr bool  HasDiffuse  [[function_constant(static_cast<uint>(ShadingMode::HasDiffuse))]];
constant constexpr bool  OnlyNormals   [[function_constant(static_cast<uint>(ShadingMode::OnlyNormals))]];
constant constexpr bool  HasSpecular [[function_constant(static_cast<uint>(ShadingMode::HasSpecular))]];
#endif