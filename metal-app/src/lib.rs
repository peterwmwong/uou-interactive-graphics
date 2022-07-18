#![feature(array_zip)]
#![feature(generic_associated_types)]
#![feature(pointer_byte_offsets)]
#![feature(const_pointer_byte_offsets)]
#![feature(portable_simd)]
#![feature(slice_as_chunks)]

#[macro_use]
pub extern crate objc;
#[macro_use]
pub extern crate cocoa;

mod application;
pub mod components;
pub mod image_helpers;
pub mod math_helpers;
mod metal_helpers;
mod model;
mod objc_helpers;
pub mod render_pipeline;
mod renderer;
mod time;
pub mod typed_buffer;

pub use application::launch_application;
pub use metal;
pub use metal_helpers::*;
pub use metal_types;
pub use model::*;
pub use objc_helpers::*;
pub use renderer::*;
pub use time::*;
