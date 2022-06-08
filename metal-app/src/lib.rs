#![feature(array_zip)]
#![feature(pointer_byte_offsets)]
#![feature(portable_simd)]
#![feature(slice_as_chunks)]

#[macro_use]
pub extern crate objc;
#[macro_use]
pub extern crate cocoa;

mod application;
mod matrix_helpers;
mod metal_helpers;
mod model;
mod objc_helpers;
mod renderer;
mod unwrap_helpers;

pub use application::launch_application;
pub use half;
pub use matrix_helpers::*;
pub use metal;
pub use metal_helpers::*;
pub use model::{MaxBounds, Model};
pub use objc_helpers::*;
pub use renderer::*;
pub use unwrap_helpers::*;
