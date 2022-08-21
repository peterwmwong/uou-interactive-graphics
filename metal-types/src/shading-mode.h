#pragma once

// TODO: Consider moving function constants to a single place so it's easier to spot duplicate indices.
#ifdef __METAL_VERSION__
constant constexpr bool  HasAmbient  [[function_constant(0)]];
constant constexpr bool  HasDiffuse  [[function_constant(1)]];
constant constexpr bool  OnlyNormals [[function_constant(2)]];
constant constexpr bool  HasSpecular [[function_constant(3)]];
#endif