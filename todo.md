# X-Projects (Extra Projects)

Projects not part of the official CS 5610/6610 coursework, but inspired by topics covered in the
lectures.

**TODO: Thoroughly look back at lectures before lecture 25 (Volume Rendering) and find techniques worth of an X-Project**

## Lecture 15 (Lights & Shadows)

- Virtual Shadow Maps (VSM)
    1. What are VSMs?
    2. How does it relate to texture streaming and spare textures?
        - https://developer.apple.com/videos/play/tech-talks/10876/?time=620
        - Are these implementation details for VSMs?
    3. Given answering/understanding #1/#2, implement VSMs
        - [Apple Metal's tech talk](https://developer.apple.com/videos/play/tech-talks/10876/?time=620)
          has a highl-level overall of the "Sparse Tiled Shadow Maps" technique

## Lecture 24 (Refractions, Transparency, Blending, & Alpha Testing)

- Refractions (https://youtu.be/LTzhxLEgldA?t=179)
    1. Find a glass model used by a paper or tutorial so our rendering can be easily
      compared/verified for "correctness"
    2. Render some model and apply front/back refractions with an environment map
        - Use Chris Wyman (2015) Front/Back Technique: https://youtu.be/LTzhxLEgldA?t=706
    3. Additional #1: Render another model behind glass model
- Refractions Ray Traced
- Order Independent Transparency
    1. Re-implement/understand Apple Metal's Sample code for Order Independent Transparency
        - https://developer.apple.com/documentation/metal/metal_sample_code_library/implementing_order-independent_transparency_with_image_blocks
    2. Additional #1: Look at Moment Based Order Independent Transparency
- Alpha Testing (https://youtu.be/LTzhxLEgldA?t=3064)
    - https://bgolus.medium.com/anti-aliased-alpha-test-the-esoteric-alpha-to-coverage-8b177335ae4f
    1. Find some foliage texture (bush, tree leaves, grass?) used by a paper tutorial so our
       rendering can be easily compared/verified for "correctness".
        - Try a simple single branch/leaf? (see ~/Downloads/Bush_Mediteranean)
    2. Render w and w/o alphaToCoverage and compare quality
        - https://developer.apple.com/documentation/metal/mtlrenderpipelinedescriptor/1514624-alphatocoverageenabled
    3. Additional #1: Apply Alpha Distribution
        - http://www.cemyuksel.com/research/alphadistribution/

# asset-compiler

- Model asset
    - Geometry
        - Look at https://github.com/zeux/meshoptimizer
            - mesh shader optimizations!
            - vertex quantization! (normalized integers)
                - Metal 3 Ray Tracing Acceleration Structures support this now too
        - Use MTLIO too?
    - Materials

# metal-shaders

- Testing

# metal-app

- Geometry should optionally load/contain tx_coords
    - Look at Model's MaterialKind (Material and NoMaterial), very similar in nature
- Delete everything RenderPipeline replaced :)
- UIRay
    - Orbiting drag doesn't quite feel right
        - It doesn't seem to scale exactly to the amount dragged.
- Move `shaders.metallib` generation and access into metal-app
    - Currently...
        1. `metal-build` generates the `metallib` into `OUT_DIR`
        2. Project includes the bytes...
            ```rs
           const LIBRARY_BYTES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"))
            ```
        3. Project passes it to the `metal` to create a `metal::Library`
        4. Project passes the library to RenderPipeline
    - What if it were treated like everything else that `metal-build` generates (bindings and bindings hash)?
    - Instead...
        1. Project uses `get_metal_library() -> metal::Library` from shader_bindings.
        2. Project passes the library to RenderPipeline
    - Unlocks...
        - Remove release build checks `library.get_function()` result
        - If we wrap `metal::Library` with our own type, we could prevent shader functions from
          other libraries being mixed up.
- Encapsulate Shadow Mapping... somehow
    - Parts
        - Render Pipeline
            - Depth Only
            - Param: Label
            - Param: Vertex Function
        - Depth State
            - Param: Label
        - Encoding
            - Setup Render Pass Descriptor / Render Command Encoder
            - Set Label, Render Pipeline, Depth State
            - Hand it back caller to do pre-draw and draw...
                - Set Buffers, Textures, any other pre-draw commands
                - Draw
- Implement Triple Buffering
    - Currently we're committing a Metal command buffer and **waiting** for completion.
    - This preventing the main thread from doing other work or queueing another frame to render
    - Most Apple Metal samples (from Apple) do triple buffering
        - All Metal resources needed for each frame are duplicated/isolated
        - Semaphore caps the number of simultaneous render encodings
- Rethink Memory/Heap Allocation/Layout Strategy
    - Most projects allocate the following resources:
        - Model Resources
            - m_model_to_world (normal_to_world can be derived from this)
            - Geometry Buffers
                - indices
                - positions
                - normals
                - texture coords
            - Material
                - Textures
                    - diffuse
                    - specular
                    - ambient
                - Constants
                    - specular shineness
        - Camera Space (previously known as World) Argument Buffer
            - m_world_to_projection
            - m_screen_to_world
            - position_world
        - Depth Texture
        - Light/Shadow Caster Resources
            - Shadow Map Depth Texture
            - Light Space Argument Buffer
                - m_world_to_projection
    - What belongs where?
        - Usage
            - Vertex Shader needs
                - Model
                    - Geometry Buffers
                    - m_model_to_world
            - Fragment Shader needs
                - Model
                    - Material
                - Camera Space
                    - m_screen_to_world
                    - position_in_world
                - Light Resources
                    - Shadow Map Depth Texture
                    - m_world_to_projection
                    - position_in_world
        - How much does colocation affect performance?
            - ex. All Fragment Shader needed resource are in one part a heap vs different parts of the heaps vs different heaps vs some in heaps some in non-heaps
- Rethink RenderDelegate API
    - proj-5 exposed how awkward it is for one Delegate to use another
        - It's not easy to initialize
            - Pass ownership of Device
        - It's not easy to reuse/share...
            - CommandQueue
            - CommandBuffer
            - Device
- Objective-C Exception information lost during Application initialization
    - Currently, if an Objective-C Exception occurs within launchApplication, the actual exception seems to be obscured.
    - Thankfully, running Xcode, will automatically debugger breakpoint at the offending location, it just lacks why.
        - I've hit this a few times by making an erroneous request of a Metal API during initialization (Bad texture allocation)
    - Is there something to be done?
        - Should certain initialization be deferred?
- Performance: Reduce/Batch Obj-C implicit `sel!` calls
    - Part of the overhead with calling into Obj-C is registering an Obj-C Selector (`sel!`) before using it.
    - Find other places where we make repeated Obj-C calls, and cache the Obj-C Selector and reuse it.
    - See example `get_gpu_addresses()`
    - Watch for `objc2` updates: https://github.com/madsmtm/objc2/pull/104
- Write tests for Materials
- Write tests for Geometry
- Write tests for Model

# metal-build

- Delete hash and bindings when shader compilation (step before bindings) or bindings fails
    - Currently, the following sequence causes `shader_bindings.rs` not to be re-created
        1. Successfully generate bindings
        2. Update bindings `.h` with a bindings-only error
            - Ex. Reference something only only defined in `#ifdef __METAL_VERSION__`
        3. Bindings will be messed up (only preamble)
        4. hash will be outdated, still the same hash after #1
        5. Fix bindings `.h` by undo-ing changes of #2
        6. Bindings don't get regenerated because the hash is the same #1!

# metal-types

# xcode-project

- Look at Apple Developer Sample Code projects on how a scheme's executable is referenced in a non-absolute path way
    - Currently `/Users/pwong` shows for Run and Profile schemes.

# Projects

- Implement Reverse-Z + Infinity Z for better depth precision (ex. fix Z-fighting)
    - https://dev.theomader.com/depth-precision/
    - https://developer.nvidia.com/content/depth-precision-visualized
    - Currently, I think I'm seeing Z-fighting on the Yoda model, zooming out slightly, and noting the flicker in Yoda's eye
    - Setup a z-fighting example rendering to verify the benefit
        - Ex. https://austin-eng.com/webgpu-samples/samples/reversedZ

## proj-3

- Add Fresnel Effect

## proj-6

- Allow Camera to freely move and rotate
    - I suspect many calculations were simplified knowing the camera is *ALWAYS* looking the world coordinate origin
        - Ex. mirror transformation: calculating the vector to reflect
    - What is affected?
    - How much more complicated do to they become?
    - How well do I maths?
    - Let's find out.
- Bring back a moveable light?

## proj-7

- Look into methods of more realistically blurring the shadows based on distance
- Review [Real-Time Polygonal-Light Shading with Linearly Transformed Cosines](https://eheitzresearch.wordpress.com/415-2/)
- Review [Combining Analytic Direct Illumination and Stochastic Shadows](https://eheitzresearch.wordpress.com/705-2/)
- Review [GPU Gems 2, Chapter 14. Perspective Shadow Maps: Care and Feeding](https://developer.nvidia.com/gpugems/gpugems/part-ii-lighting-and-shadows/chapter-14-perspective-shadow-maps-care-and-feeding)

## proj-8

- Implement Parallax Occlusion Mapping
- Implement using Mesh Shaders

# tasks.json

- Could saving a metal file, auto restart a running application (based on the crate directory)?
    - Would make fiddling shaders alot faster, but maybe it's not any faster than doing it in xcode shader debugger?

# scripts

- Make a script for generating the project gifs
    ```sh
    for i in $(ls *.mov); do ffmpeg -i $i -filter_complex "[0:v] fps=30,scale=512:-1,split [a][b];[a] palettegen [p];[b][p] paletteuse" $i.gif; done
    ```

## asm-gen

- Generate instruction type counts (ex. how many branch instructions?)

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
