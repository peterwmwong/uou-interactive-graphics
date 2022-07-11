#![feature(fs_try_exists)]
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
};

const CUBE_TEXTURE_FILENAMES: [&'static str; 6] = [
    "cubemap_posx.png",
    "cubemap_negx.png",
    "cubemap_posy.png",
    "cubemap_negy.png",
    "cubemap_posz.png",
    "cubemap_negz.png",
];

fn hash_assets<const N: usize>(paths_to_hash: [PathBuf; N]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for path in paths_to_hash {
        std::fs::read(&path).unwrap().hash(&mut hasher);
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
    }
    hasher.finish()
}

fn read_cached_assets_hash(cached_hash_path: &PathBuf) -> Option<u64> {
    println!(
        "cargo:rerun-if-changed={}",
        cached_hash_path.to_string_lossy()
    );
    if let Ok(hash) = std::fs::read(cached_hash_path) {
        return Some(u64::from_ne_bytes(hash.try_into().unwrap()));
    }
    None
}

fn save_assets_hash(hash: u64, cached_hash_path: &PathBuf) {
    std::fs::write(cached_hash_path, hash.to_ne_bytes()).unwrap();
}

fn create_cubemap_asset() {
    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let cube_source_textures_dir = assets_dir.join("cubemap");
    let cached_hash_path = assets_dir.join("assets_hash");
    let current_hash =
        hash_assets(CUBE_TEXTURE_FILENAMES.map(|a| cube_source_textures_dir.join(a)));
    if let Some(old_hash) = read_cached_assets_hash(&cached_hash_path) {
        if old_hash == current_hash {
            return;
        }
    }

    let cubemap_asset_dir = assets_dir.join("cubemap.asset");
    if std::fs::try_exists(&cubemap_asset_dir)
        .expect("Could not determine whether destination exists or not")
    {
        if cubemap_asset_dir.is_dir() {
            std::fs::remove_dir_all(&cubemap_asset_dir)
                .expect("Unable to remove existing asset directory");
        } else {
            std::fs::remove_file(&cubemap_asset_dir).expect("Unable to remove existing asset file");
        }
    }
    std::fs::create_dir(&cubemap_asset_dir).expect("Failed to create temp asset directory");
    let test_cube_texture_files = CUBE_TEXTURE_FILENAMES.map(|f| cube_source_textures_dir.join(f));
    asset_compiler::cube_texture::create_cube_texture_asset_dir(
        &cubemap_asset_dir,
        &test_cube_texture_files,
    );

    save_assets_hash(current_hash, &cached_hash_path);
}

fn main() {
    metal_build::build();
    create_cubemap_asset();
}
