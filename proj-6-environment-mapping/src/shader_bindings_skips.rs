use super::shader_bindings::*;
use metal_app::pipeline::{Bind, BindTexture};

// TODO: START HERE
// TODO: START HERE
// TODO: START HERE
// Have the generate_rust_bindings create this.
impl main_fragment_binds<'_> {
    pub fn skip() -> Self {
        Self {
            camera: Bind::Skip,
            light_pos: Bind::Skip,
            matrix_env: Bind::Skip,
            darken: Bind::Skip,
            env_texture: BindTexture::Skip,
        }
    }
}

impl main_vertex_binds<'_> {
    pub fn skip() -> Self {
        Self {
            geometry: Bind::Skip,
            camera: Bind::Skip,
            model: Bind::Skip,
        }
    }
}

impl bg_fragment_binds<'_> {
    pub fn skip() -> Self {
        Self {
            camera: Bind::Skip,
            env_texture: BindTexture::Skip,
        }
    }
}
