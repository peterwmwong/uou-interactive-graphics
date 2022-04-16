use std::{env, path::PathBuf};

// Verifies `vector_type_helpers.rs`.
pub fn main() {
    println!("cargo:rerun-if-changed=src/rust-bindgen-only-vector-types.h");
    let rust_bindgen_only_vector_types_header =
        String::from_utf8_lossy(include_bytes!("src/rust-bindgen-only-vector-types.h"));

    bindgen::Builder::default()
        .header_contents(
            "rust-bindgen-only-vector-types.h",
            &rust_bindgen_only_vector_types_header,
        )
        .clang_arg("-xc++")
        .clang_arg("-std=c++17")
        .derive_eq(true)
        .derive_debug(cfg!(debug_assertions))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(
            PathBuf::from(env::var("OUT_DIR").unwrap())
                .join("rust-bindgen-only-vector-type-bindings.rs"),
        )
        .unwrap();
}
