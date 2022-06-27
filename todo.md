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

- Camera
    - Implement Reverse-Z + Infinity Z for better depth precision (ex. fix Z-fighting)
        - https://dev.theomader.com/depth-precision/
        - https://developer.nvidia.com/content/depth-precision-visualized
        - Currently, I think I'm seeing Z-fighting on the Yoda model, zooming out slightly, and noting the flicker in Yoda's eye
        - Setup a z-fighting example rendering to verify the benefit
            - Ex. https://austin-eng.com/webgpu-samples/samples/reversedZ
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

# metal-types

# xcode-project

- Look at Apple Developer Sample Code projects on how a scheme's executable is referenced in a non-absolute path way
    - Currently `/Users/pwong` shows for Run and Profile schemes.

# Projects

- Move common assets to a root directory
    - Too many Teapots and Yodas

## proj-3

- Add Fresnel Effect

## proj-6

- Remove is_mirrored, include mirroring into matrix transformations for mirrored rendering.
- Optimize
    - Reduce environment cube texture loading the Metal 3 MTLIO
    - Reduce render passes
        - Can everything be done in a single render pass?
            - Raster Order Groups
            - Amplification / Viewports
- Allow Camera to freely move and rotate
    - I suspect many calculations were simplified knowing the camera is *ALWAYS* looking the world coordinate origin
        - Ex. mirror transformation: calculating the vector to reflect
    - What is affected?
    - How much more complicated do to they become?
    - How well do I maths?
    - Let's find out.
- Bring back a moveable light?

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