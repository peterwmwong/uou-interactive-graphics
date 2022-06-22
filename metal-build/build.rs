use std::{
    env,
    fmt::Debug,
    path::{Path, PathBuf},
};

use bindgen::{callbacks::ParseCallbacks, CargoCallbacks};

const METAL_BUILD_MANIFEST_DIR: &'static str = env!("CARGO_MANIFEST_DIR");

static mut ITEMS: Vec<String> = Vec::new();

#[derive(Debug)]
struct CollectItems {
    cargo_callbacks: CargoCallbacks,
}

impl CollectItems {
    fn new() -> Self {
        Self {
            cargo_callbacks: CargoCallbacks,
        }
    }
}

impl ParseCallbacks for CollectItems {
    fn include_file(&self, filename: &str) {
        self.cargo_callbacks.include_file(filename);
    }
    fn item_name(&self, item_name: &str) -> Option<String> {
        if item_name != "root" {
            unsafe {
                let item_name = item_name.to_owned();
                if !ITEMS.contains(&item_name) {
                    ITEMS.push(item_name);
                }
            }
        }
        None
    }
}

// Verifies `vector_type_helpers.rs`.
pub fn main() {
    // TODO: Figure out a way to keep this in-sync with lib.rs
    let header = Path::new(METAL_BUILD_MANIFEST_DIR)
        .join("src")
        .join("rust_bindgen_only_metal_types.h");

    bindgen::Builder::default()
        .header(header.to_string_lossy())
        .clang_arg("-xc++")
        .clang_arg("-std=c++17")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .derive_eq(true)
        .derive_debug(false)
        .no_debug("*")
        .parse_callbacks(Box::new(CollectItems::new()))
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(
            PathBuf::from(env::var("OUT_DIR").unwrap())
                .join("rust_bindgen_only_metal_type_bindings.rs"),
        )
        .expect("Failed to generate rust_bindgen_only_metal_type_bindings.rs");
    std::fs::write(
        Path::new(METAL_BUILD_MANIFEST_DIR)
            .join("src")
            .join("rust_bindgen_only_metal_types_list.rs"),
        unsafe {
            ITEMS.sort();
            let joined_items = ITEMS
                .iter()
                .map(|a| format!("\t\"{a}\",\n"))
                .collect::<String>();
            let num_items = ITEMS.len();
            format!(r#"/**************************************************************************************************
GENERATED FILE. DO NOT MODIFY.

This file is generated by the `build.rs`.
***************************************************************************************************/
pub const TYPES: [&'static str; {num_items}] = [
{joined_items}];
"#)
        },
    )
    .expect("Failed to write tmp.txt");
}
