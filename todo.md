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

- Rethink RenderDelegate API
    - proj-5 exposed how awkward it is for one Delegate to use another
        - It's not easy to initialize
            - Pass ownership of Device
        - It's not easy to reuse/share...
            - CommandQueue
            - CommandBuffer
            - Device
- Performance: Reduce/Batch Obj-C implicit `sel!` calls
    - Part of the overhead with calling into Obj-C is registering an Obj-C Selector (`sel!`) before using it.
    - Find other places where we make repeated Obj-C calls, and cache the Obj-C Selector and reuse it.
    - See example `get_gpu_addresses()`
    - Watch for `objc2` updates: https://github.com/madsmtm/objc2/pull/104
- Write tests for Materials
- Write tests for Geometry
- Write tests for Model

# metal-build

- Consider moving rust polyfill metal_types (codegen and code) into it's own crate.
    - Move going further... why even codegen metal types?
    - Couldn't we simply define these types in rust with the correct size/alignment?
    - Other than size/alignment what could get out-of-sync with metal compiler version?
- Make first build faster
    - Currently, the first build will run bindgen because there is no cached hash file (hash value based on `src/rust_bindgen_only_metal_types.h`)...
        - ... which means we need to generate `$OUT_DIR/rust_bindgen_only_metal_type_bindings.rs` and `rust_bindgen_only_metal_types_list.rs`
    - What we could do....
        - Write `rust_bindgen_only_metal_type_bindings.rs` to `src/` (instead of `$OUT_DIR`)
        - Write the cached hash file to `src/`
        - Change the `build.rs` to compare the hash file with current has of `src/rust_bindgen_only_metal_types.h`, before running bindgen.

# xcode-project

- Look at Apple Developer Sample Code projects on how a scheme's executable is referenced in a non-absolute path way
    - Currently `/Users/pwong` shows for Run and Profile schemes.

# All Projects

- Add gif/picture to readmes

# tasks.json

- Could saving a metal file, auto restart a running application (based on the crate directory)?
    - Would make fiddling shaders alot faster, but maybe it's not any faster than doing it in xcode shader debugger?
