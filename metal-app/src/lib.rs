#![feature(array_zip)]
#![feature(pointer_byte_offsets)]
#![feature(portable_simd)]
#![feature(slice_as_chunks)]

#[macro_use]
pub extern crate objc;
#[macro_use]
pub extern crate cocoa;

mod application;
pub mod components;
mod metal_helpers;
mod model;
mod objc_helpers;
mod renderer;
mod unwrap_helpers;

pub use application::launch_application;
pub use metal;
pub use metal_helpers::*;
pub use metal_types;
pub use model::{GeometryToEncode, MaterialToEncode, MaxBounds, Model};
pub use objc_helpers::*;
pub use renderer::*;
pub use unwrap_helpers::*;
