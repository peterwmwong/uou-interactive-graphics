#include "shading.h"

[[fragment]]
half4 test_shade_phong_blinn_with_constant_material() {
    return shade_phong_blinn<ConstantMaterial>(
        {
            .frag_pos = half3(0),
            .light_pos = half3(0),
            .camera_pos = half3(0),
            .normal = half3(0),
        },
        ConstantMaterial(0, 0, 0, 0, 0)
    );
}

struct Material {
    const texture2d<half> ambient_texture;
    const texture2d<half> diffuse_texture;
    const texture2d<half> specular_texture;
    const float           specular_shineness;
    const float           ambient_amount;
};

[[fragment]]
half4 test_shade_phong_blinn_with_textured_material(constant Material & material [[buffer(0)]]) {
    return shade_phong_blinn(
        {
            .frag_pos = half3(0),
            .light_pos = half3(0),
            .camera_pos = half3(0),
            .normal = half3(0),
        },
        TexturedMaterial<Material>(float2(0), false, material)
    );
}