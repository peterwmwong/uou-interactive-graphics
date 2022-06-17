# proj-3-fresnel

- Add Fresnel Effect

# metal-shader-app

- Consider creating a ShaderToy-esque crate that allows you to quickly create a metal-app with just
  a **Fragment** Shader
- As such, assumes the following configuration...
    - Application/Window setup
    - Window Resize maintenance
        - Resize depth attachment texture
    - Single Render Pipeline
        - Color Attachment
        - Depth Attachment
        - Fragment shader
            - Function name of `main()`
            - Bound buffers
                - Mouse
                    - Coordinate
                    - Button states
                - Time
                - Viewport/Screen Size (pixels)
    - Render
        - One draw call (triangle strip that's a quad fits the screen exactly)
- Question: Would it be better to allow specifying more than just a Fragment shader?
    - Geometry (vertices)
    - Vertex shader
        - This should simplify...
            - Applying Perspective Projection
            - Remove the need to Ray Marching
                - Which one is worse/harder?
                    - Ray Marching and SDF
                    - Vertices, Vertex Shader, Draw call
                        - Maybe there's a Vertices/Draw call simplification? Just provide vertices and assume a draw call with triangle primitive and vertex count.

# metal-app

- Extract Camera/Light user interaction and position (rotation/distance) maintenance
    ```rs
    // API
    struct UserInteractableRay { // jeeeeeeeeeessus, think of a better name mate
        distance_from_origin: f32,
        rotation_xy: f32x2,
        modifier_keys: ModifierKeys,
    }

    impl UserInteractableRay {
        // Returns `true` if event matches and caused a change in distance or rotation
        pub fn on_event(&mut self, event: UserEvent) -> bool { ... }
    }

    // Usage
    Delegate {
        light: UserInteractableRay::new(INITIAL_LIGHT_DISTANCE, INITIAL_LIGHT_ROTATION, ModifierKeys::CONTROL),
        camera: UserInteractableRay::new(INITIAL_CAMERA_DISTANCE, INITIAL_CAMERA_ROTATION, ModifierKeys::empty())
    }

    impl RenderDelegate for Delegate {
        fn on_event(&mut self, event: UserEvent) {
            for el in &[&self.light, &self.camera] {
                if el.on_event(event) { // `true` if event matches and caused a change
                    self.needs_render = true;
                    return;
                }
            }
        }
    }
    ```
- Metal 3
    - Use new gpuAddress/gpuHandle, and remove calls to argument encoder
        - https://developer.apple.com/videos/play/wwdc2022/10101/
- Write tests for Materials
- Write tests for Geometry
- Write tests for Model

# metal-build

- Move the generated shader_bindings.rs back into OUT_DIR
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
