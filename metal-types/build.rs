use bindgen::{callbacks::ParseCallbacks, CargoCallbacks};
use std::{
    env,
    fmt::Debug,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
};

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

// TODO: Find a better way to filter out constants
pub fn is_constant_name(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_uppercase() || c == '_')
}

impl ParseCallbacks for CollectItems {
    fn include_file(&self, filename: &str) {
        self.cargo_callbacks.include_file(filename);
    }
    fn item_name(&self, item_name: &str) -> Option<String> {
        if item_name != "root" && !is_constant_name(item_name) {
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

// Verifies `rust_bindgen_only_metal_types_bindings.rs`.
pub fn main() {
    // TODO: Figure out a way to keep this in-sync with lib.rs
    let src_dir = Path::new(METAL_BUILD_MANIFEST_DIR).join("src");
    let header = src_dir.join("all_metal_types.h");
    let mut deps = get_shader_deps(&concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/all_metal_types.metal"
    ));
    deps.push(header.clone());
    let deps_refs: Vec<&dyn AsRef<Path>> = deps.iter().map(|a| a as _).collect();

    build_hash::build_hash(
        src_dir.join("all_metal_types_h_hash"),
        &deps_refs[..],
        || {
            let mut all_metal_types_file = fs::File::options()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&src_dir.join("all_metal_types.rs"))
            .expect("Could not create all_metal_types.rs containing Rust bindings for types in src/all_metal_types.h");

            all_metal_types_file
                .write_all(
                    r#"#![allow(non_snake_case)]
"#
                    .as_bytes(),
                )
                .expect(
                    "Failed to generate tests verifying all generated Rust types implement Copy",
                );

            bindgen::Builder::default()
                .header(header.to_string_lossy())
                .clang_arg("-xc++")
                .clang_arg("-std=c++17")
                .default_enum_style(bindgen::EnumVariation::Rust {
                    non_exhaustive: false,
                })
                .derive_copy(true)
                .derive_debug(false)
                .derive_default(true)
                .derive_eq(true)
                .no_debug("*")
                .parse_callbacks(Box::new(CollectItems::new()))
                .generate()
                .expect("Unable to generate bindings")
                .write(Box::new(&all_metal_types_file))
                .expect("Failed to generate all_metal_types.rs");
            unsafe {
                ITEMS.sort();
            }

            let mut w = |s: &str| {
                all_metal_types_file.write_all(s.as_bytes()).expect(
                    "Failed to generate tests verifying all generated Rust types implement Copy",
                );
            };

            // Generate tests to verify all Metal Types derive Copy/Clone.
            w("
#[test]
fn test_metal_types_derive_copy() {
    use std::marker::PhantomData;
    struct HasCopyClone<T: Sized + Copy + Clone>(PhantomData<T>);");
            for item in unsafe { &ITEMS } {
                w(&format!(
                    r"
    HasCopyClone(PhantomData::<{item}>);"
                ));
            }
            w("
}");
            std::fs::write(
    src_dir
        .join("all_metal_types_list.rs"),
    unsafe {
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
        },
    );
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
