use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

fn main() {
    generate_rust_types_from_shader_types();
    compile_shaders();
}

fn generate_rust_types_from_shader_types() {
    println!("cargo:rerun-if-changed=shader_src/common.h");
    println!("cargo:rerun-if-changed=shader_src/rust-bindgen-only-vector-types.h");

    let cached_hash_path =
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("shader_src_common_h_hash");
    let current_hash = hash_shader_src([
        PathBuf::from("build.rs"),
        PathBuf::from("shader_src").join("common.h"),
        PathBuf::from("shader_src").join("rust-bindgen-only-vector-types.h"),
    ]);
    if let Some(old_hash) = read_cached_shader_src_hash(&cached_hash_path) {
        if old_hash == current_hash {
            return;
        }
    }

    bindgen::Builder::default()
        .header("shader_src/common.h")
        .clang_arg("-xc++")
        .clang_arg("-std=c++17")
        .derive_eq(true)
        .derive_debug(cfg!(debug_assertions))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("shader_bindings.rs"))
        .unwrap();

    save_shader_src_hash(current_hash, &cached_hash_path);
}

fn hash_shader_src<const N: usize>(paths_to_hash: [PathBuf; N]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for path in paths_to_hash {
        fs::read(path).unwrap().hash(&mut hasher);
    }
    hasher.finish()
}

fn read_cached_shader_src_hash(cached_hash_path: &PathBuf) -> Option<u64> {
    if let Ok(hash) = fs::read(cached_hash_path) {
        return Some(u64::from_ne_bytes(hash.try_into().unwrap()));
    }
    None
}

fn save_shader_src_hash(hash: u64, cached_hash_path: &PathBuf) {
    fs::write(cached_hash_path, hash.to_ne_bytes()).unwrap();
}

fn compile_shaders() {
    println!("cargo:rerun-if-changed=shader_src");

    // Compile Metal Shaders into the following:
    // - shaders.air         Metal IR (AIR) used to create Metal binary (metallib)
    // - shaders.metallib    Metal binary loaded by application containing compiled shaders
    // - shaders.metallibsym Debugging information (release build only). In non-release builds,
    //                       debugging information is embedded in shaders.metallib.
    // - shaders.dia         Metal Diagnostics? Important for XCode Frame Capture debugging/profiling (prevents XCode crash)
    // - shaders.dat         Metal Dependencies? Important for XCode Frame Capture debugging/profiling (prevents XCode crash)
    //
    // See: https://developer.apple.com/documentation/metal/libraries/generating_and_loading_a_metal_library_symbol_file
    let out_dir = env::var("OUT_DIR").unwrap();
    let tmp_diag_path = format!("{out_dir}/shaders.dia");
    let tmp_deps_path = format!("{out_dir}/shaders.dat");
    let shaders_air_path = format!("{out_dir}/shaders.air");
    let shaders_metallib_path = format!("{out_dir}/shaders.metallib");
    run_command(Command::new("xcrun").args(&[
        "-sdk",
        "macosx",
        "metal",
        "-c",
        "-gline-tables-only",
        "-frecord-sources",
        "-ffast-math",
        // Options copied from XCode build logs of a working Apple Metal sample project.
        // vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv
        "-serialize-diagnostics",
        &tmp_diag_path,
        "-index-store-path",
        &out_dir,
        "-MMD",
        "-MT",
        "dependencies",
        "-MF",
        &tmp_deps_path,
        // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        "./shader_src/shaders.metal",
        "-o",
        &shaders_air_path,
    ]));
    run_command(Command::new("xcrun").args(&[
        "-sdk",
        "macosx",
        "metal",
        "-frecord-sources",
        "-o",
        &shaders_metallib_path,
        &shaders_air_path,
    ]));

    // Extracts debugging information from .metallib file and into a new .metallibsym file (release
    // build only).
    // See: https://developer.apple.com/documentation/metal/libraries/generating_and_loading_a_metal_library_symbol_file
    #[cfg(not(debug_assertions))]
    run_command(Command::new("xcrun").args(&[
        "-sdk",
        "macosx",
        "metal-dsymutil",
        "-flat",
        "-remove-source",
        &shaders_metallib_path,
    ]));
}

fn run_command(command: &mut Command) {
    let output = command.spawn().unwrap().wait_with_output().unwrap();
    if !output.status.success() {
        panic!(
            r#"
stdout: {}
stderr: {}
"#,
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap()
        );
    }
}
