#![feature(portable_simd)]
mod vector_type_helpers;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

const METAL_BUILD_MANIFEST_DIR: &'static str = env!("CARGO_MANIFEST_DIR");

pub fn build() {
    generate_rust_shader_bindings();
    compile_shaders();
}

fn generate_rust_shader_bindings() {
    let shader_common_header_file = PathBuf::from("shader_src").join("common.h");
    let rust_bindgen_only_vector_types_header_file = Path::new(METAL_BUILD_MANIFEST_DIR)
        .join("src")
        .join("rust-bindgen-only-vector-types.h");
    let vector_type_helpers_file = Path::new(METAL_BUILD_MANIFEST_DIR)
        .join("src")
        .join("vector_type_helpers.rs");

    let cached_hash_path =
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("shader_src_common_h_hash");
    let current_hash = hash_shader_src([
        &PathBuf::from("build.rs"),
        &shader_common_header_file,
        &rust_bindgen_only_vector_types_header_file,
        &vector_type_helpers_file,
    ]);
    if let Some(old_hash) = read_cached_shader_src_hash(&cached_hash_path) {
        if old_hash == current_hash {
            return;
        }
    }

    let shader_bindings_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("src")
        .join("shader_bindings.rs");
    {
        let mut shader_bindings_file = fs::File::options()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&shader_bindings_path)
        .expect("Could not create shader_bindings.rs containing Rust bindings for types in shader_src/common.h");

        shader_bindings_file
            .write(
                r#"#![allow(deref_nullptr, non_upper_case_globals, non_snake_case)]
"#
                .as_bytes(),
            )
            .unwrap();

        bindgen::Builder::default()
            .header(rust_bindgen_only_vector_types_header_file.to_string_lossy())
            .header(shader_common_header_file.to_string_lossy())
            .clang_arg("-xc++")
            .clang_arg("-std=c++17")
            .derive_eq(true)
            .derive_debug(cfg!(debug_assertions))
            .parse_callbacks(Box::new(bindgen::CargoCallbacks))
            .generate()
            .expect("Unable to generate bindings")
            .write(Box::new(&shader_bindings_file))
            .unwrap();

        let mut found_start_here = false;
        for line in fs::read_to_string(vector_type_helpers_file)
            .unwrap()
            .lines()
        {
            if !found_start_here {
                found_start_here = line == "// APPEND THE FOLLOWING TO `shader_bindings.rs`";
            } else {
                shader_bindings_file
                    .write_fmt(format_args!("{line}\n"))
                    .expect("Could not add vector type helpers to shader_bindings.rs");
            }
        }
    }

    save_shader_src_hash(current_hash, &cached_hash_path);
}

fn hash_shader_src<const N: usize>(paths_to_hash: [&PathBuf; N]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for path in paths_to_hash {
        fs::read(path).unwrap().hash(&mut hasher);
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
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
    let shader_src_dir = PathBuf::from("shader_src");
    println!(
        "cargo:rerun-if-changed={}",
        shader_src_dir.to_string_lossy()
    );

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
