use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{env, fs};

const METAL_BUILD_MANIFEST_DIR: &'static str = env!("CARGO_MANIFEST_DIR");

pub fn build() {
    generate_rust_shader_bindings();
    compile_shaders();
}

fn generate_rust_shader_bindings() {
    let shader_src_dir = PathBuf::from("shader_src");
    let shader_bindings_header_file = shader_src_dir.join("shader_bindings.h");
    let rust_bindgen_only_metal_types_header_file = Path::new(METAL_BUILD_MANIFEST_DIR)
        .join("..")
        .join("metal-types")
        .join("src")
        .join("rust_bindgen_only_metal_types.h");

    build_hash::build_hash(
        shader_src_dir.join("shader_bindings_h_hash"),
        &[
            &shader_bindings_header_file,
            &rust_bindgen_only_metal_types_header_file,
        ],
        || {
            let shader_bindings_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("src")
                .join("shader_bindings.rs");
            let mut shader_bindings_file = fs::File::options()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&shader_bindings_path)
                .expect("Could not create shader_bindings.rs containing Rust bindings for types in shader_src/shader_bindings.h");

            shader_bindings_file
                .write(
                    r#"#![allow(deref_nullptr, non_upper_case_globals, non_snake_case)]
/**************************************************************************************************
 GENERATED FILE. DO NOT MODIFY.

 This file is generated by the `metal_build` crate, check you're `build.rs` for
 `metal_build::build()`.
 Structs and Enums are generated based on `shader_src/shader_bindings.h`.
***************************************************************************************************/
#[allow(unused_imports)]
use metal_app::metal_types::*;
"#
                    .as_bytes(),
                )
                .unwrap();

            let mut builder = bindgen::Builder::default()
                .header(rust_bindgen_only_metal_types_header_file.to_string_lossy())
                .header(shader_bindings_header_file.to_string_lossy())
                .clang_arg("-xc++")
                .clang_arg("-std=c++17")
                .derive_eq(true)
                .default_enum_style(bindgen::EnumVariation::Rust {
                    non_exhaustive: false,
                })
                .derive_debug(false)
                .no_debug("*")
                .parse_callbacks(Box::new(bindgen::CargoCallbacks));
            for block_item in metal_types::TYPES {
                builder = builder.blocklist_type(block_item);
            }
            builder
                .generate()
                .expect("Unable to generate bindings")
                .write(Box::new(&shader_bindings_file))
                .expect("Unable to write shader_bindings.rs file");
        },
    );
}

fn get_shader_deps(shader_path: &str) -> Vec<PathBuf> {
    let Output { stdout, .. } = run_command(
        Command::new("xcrun")
            .args(&[
                "-sdk",
                "macosx",
                "metal",
                "-std=metal3.0",
                shader_path,
                "-MM",
            ])
            .env_clear(),
    );
    let mut deps = vec![];
    for l in std::str::from_utf8(&stdout)
        .expect("Failed to read dependencies output")
        .lines()
        .filter(|l| l.starts_with("  ") && !l.contains("include/metal/module.modulemap"))
    {
        let dep = l.trim_end_matches(" \\").trim_start_matches(' ');
        let dep = PathBuf::from(dep).canonicalize().expect(&format!(
            "Failed to canonicalize path to dependency {dep:?}"
        ));
        if !deps.contains(&dep) {
            deps.push(dep);
        }
    }
    deps
}

fn compile_shaders() {
    let metal_shaders_file = PathBuf::from("shader_src")
        .join("shaders.metal")
        .canonicalize()
        .expect("Failed to canonicalize path to shaders.metal")
        .to_string_lossy()
        .to_string();
    println!("{metal_shaders_file}");
    for dep in get_shader_deps(&metal_shaders_file) {
        println!("cargo:rerun-if-changed={}", dep.to_string_lossy());
    }

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
        "-std=metal3.0",
        &metal_shaders_file,
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

fn run_command(command: &mut Command) -> Output {
    let out = command
        .output()
        .expect(&format!("Failed to run command {command:?}"));
    if !out.status.success() {
        panic!(
            r#"
    stdout: {}
    stderr: {}
    "#,
            String::from_utf8(out.stdout).unwrap(),
            String::from_utf8(out.stderr).unwrap()
        );
    }
    out
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let shader_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_shader_src")
            .canonicalize()
            .expect("Failed to canonicalize path to test_shader_src directory");
        let shader_file = shader_dir.join("shaders.metal");
        let mut deps: Vec<PathBuf> = get_shader_deps(&shader_file.to_string_lossy());
        deps.sort();
        let deps: Vec<String> = deps
            .into_iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        let mut expected = [
            "../../metal-shaders/shader_src/bindings/material.h",
            "../../metal-shaders/shader_src/bindings/macros.h",
            "../../metal-shaders/shader_src/bindings/model-space.h",
            "../../metal-shaders/shader_src/bindings/projected-space.h",
            "../../metal-shaders/shader_src/bindings/shading-mode.h",
            "../../metal-shaders/shader_src/bindings/geometry.h",
            "../../metal-shaders/shader_src/shading.h",
            "shaders.metal",
            "shader_bindings.h",
        ]
        .map(|s| {
            shader_dir
                .join(s)
                .canonicalize()
                .expect("Failed to canonicalize expected dependency")
                .to_string_lossy()
                .to_string()
        });
        expected.sort();

        assert_eq!(&deps, &expected);
    }
}
