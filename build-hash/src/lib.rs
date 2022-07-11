use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::Path,
};

fn hash_assets<P: AsRef<Path>>(paths_to_hash: &[P]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for path in paths_to_hash {
        std::fs::read(path).unwrap().hash(&mut hasher);
        println!("cargo:rerun-if-changed={}", path.as_ref().to_string_lossy());
    }
    hasher.finish()
}

fn read_cached_assets_hash<P: AsRef<Path>>(cached_hash_path: P) -> Option<u64> {
    println!(
        "cargo:rerun-if-changed={}",
        cached_hash_path.as_ref().to_string_lossy()
    );
    if let Ok(hash) = std::fs::read(cached_hash_path) {
        return Some(u64::from_ne_bytes(hash.try_into().unwrap()));
    }
    None
}

fn save_assets_hash<P: AsRef<Path>>(hash: u64, cached_hash_path: P) {
    std::fs::write(cached_hash_path.as_ref(), hash.to_ne_bytes()).unwrap();
}

pub fn build_hash<P: AsRef<Path>, P2: AsRef<Path>, F: FnOnce()>(
    cached_hash_path: P,
    paths_to_hash: &[P2],
    f: F,
) {
    let current_hash = hash_assets(paths_to_hash);
    if let Some(old_hash) = read_cached_assets_hash(&cached_hash_path) {
        if old_hash == current_hash {
            return;
        }
    }
    f();
    save_assets_hash(current_hash, &cached_hash_path);
}
