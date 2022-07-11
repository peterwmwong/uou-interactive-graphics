#![feature(fs_try_exists)]
use std::path::PathBuf;

const CUBE_TEXTURE_FILENAMES: [&'static str; 6] = [
    "cubemap_posx.png",
    "cubemap_negx.png",
    "cubemap_posy.png",
    "cubemap_negy.png",
    "cubemap_posz.png",
    "cubemap_negz.png",
];

fn create_cubemap_asset() {
    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let cube_source_textures_dir = assets_dir.join("cubemap");
    build_hash::build_hash(
        &assets_dir.join("cubemap_asset_hash"),
        &CUBE_TEXTURE_FILENAMES.map(|a| cube_source_textures_dir.join(a)),
        || {
            let cubemap_asset_dir = assets_dir.join("cubemap.asset");
            if std::fs::try_exists(&cubemap_asset_dir)
                .expect("Could not determine whether destination exists or not")
            {
                if cubemap_asset_dir.is_dir() {
                    std::fs::remove_dir_all(&cubemap_asset_dir)
                        .expect("Unable to remove existing asset directory");
                } else {
                    std::fs::remove_file(&cubemap_asset_dir)
                        .expect("Unable to remove existing asset file");
                }
            }
            std::fs::create_dir(&cubemap_asset_dir).expect("Failed to create temp asset directory");
            let test_cube_texture_files =
                CUBE_TEXTURE_FILENAMES.map(|f| cube_source_textures_dir.join(f));
            asset_compiler::cube_texture::create_cube_texture_asset_dir(
                &cubemap_asset_dir,
                &test_cube_texture_files,
            );
        },
    );
}

fn main() {
    metal_build::build();
    create_cubemap_asset();
}
