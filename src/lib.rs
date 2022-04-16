#![feature(array_methods)]
#![feature(array_zip)]
#![feature(core_intrinsics)]
#![feature(let_else)]
#![feature(portable_simd)]

#[macro_use]
extern crate objc;
#[macro_use]
extern crate cocoa;

pub mod application;
pub mod metal_helpers;
pub mod objc_helpers;
pub mod renderer;
pub mod shader_bindings;
pub mod unwrap_helpers;
