#include "shading.h"

[[fragment]]
half4 test_shade_phong_blinn_with_constant_material() {
    return shade_phong_blinn<ConstantMaterial>(
        {
            .frag_pos = half3(0),
            .light_pos = half3(0),
            .camera_pos = half3(0),
            .normal = half3(0),
            .has_ambient = true,
            .has_diffuse = true,
            .has_specular = true,
        },
        ConstantMaterial(0, 0, 0, 0, 0)
    );
}

[[fragment]]
half4 test_shade_phong_blinn_with_textured_material(constant Material & material [[buffer(0)]]) {
    return shade_phong_blinn(
        {
            .frag_pos = half3(0),
            .light_pos = half3(0),
            .camera_pos = half3(0),
            .normal = half3(0),
            .has_ambient = true,
            .has_diffuse = true,
            .has_specular = true,
        },
        TexturedMaterial(material, float2(0))
    );
}