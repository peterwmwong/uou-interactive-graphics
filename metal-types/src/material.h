#pragma once

#include "./macros.h"

// A Model object's material. Commonly used with `metal_app::model::Model` to load and help
// encode textures to be used by a Fragment Shader.
struct Material {
    ARG_TEXTURE(texture2d<half>) ambient_texture;
    ARG_TEXTURE(texture2d<half>) diffuse_texture;
    ARG_TEXTURE(texture2d<half>) specular_texture;
    float                        specular_shineness;
    float                        ambient_amount;
};
