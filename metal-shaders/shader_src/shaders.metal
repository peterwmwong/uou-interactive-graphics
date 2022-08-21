#include "./shading.h"
#include "../../metal-types/src/shading-mode.h"

[[fragment]]
half4 test_shade_phong_blinn_with_constant_material() {
    return shade_phong_blinn<ConstantMaterial>(
        {
            .frag_pos = half3(0),
            .light_pos = half3(0),
            .camera_pos = half3(0),
            .normal = half3(0),
            .has_ambient = HasAmbient,
            .has_diffuse = HasDiffuse,
            .has_specular = HasSpecular,
            .only_normals = OnlyNormals
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
            .has_ambient = HasAmbient,
            .has_diffuse = HasDiffuse,
            .has_specular = HasSpecular,
            .only_normals = OnlyNormals
        },
        TexturedMaterial(material, float2(0))
    );
}