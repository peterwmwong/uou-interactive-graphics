# metal-app

# metal-build

- Switching between release/dev build doesn't seem to rebuild shader_bindings.
    - Wish we could add `#[cfg_attr(debug_assertions), derive(Debug)]` to bindgen
- Can bindgen replace vector types (ex. float2) with SIMD<?,?> equivalents?
    - This should remove/reduce the need for vector_type_helpers

# xcode-project

- Look at Apple Developer Sample Code projects on how a scheme's executable is referenced in a non-absolute path way
    - Currently `/Users/pwong` shows for Run and Profile schemes.

# tasks.json

- Could saving a metal file, auto restart a running application (based on the crate directory)?
    - Would make fiddling shaders alot faster, but maybe it's not any faster than doing it in xcode shader debugger?
