#pragma once

#ifdef __METAL_VERSION__

// TODO: Can this be removed
#include <metal_stdlib>
using namespace metal;

#define ARG_CONSTANT_PTR(x) constant x*
#define ARG_TEXTURE(x) x
#define DEF_CONSTANT constant

#else

#define ARG_CONSTANT_PTR(x) unsigned long
#define ARG_TEXTURE(x) unsigned long
#define DEF_CONSTANT

#endif