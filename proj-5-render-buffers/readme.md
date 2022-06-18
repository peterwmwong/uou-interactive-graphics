https://graphics.cs.utah.edu/courses/cs6610/spring2022/?prj=5

# Usage

```sh
> cargo run --bin proj-5-render-buffers [Path to Wavefront OBJ file]
```

Examples:

```sh
# Project 4 Teapot model
> cargo run --bin proj-5-render-buffers proj-4-textures/assets/teapot
# Project 4 Yoda model
> cargo run --bin proj-5-render-buffers proj-4-textures/assets/yoda/yoda.obj
```

# Controls

| Mouse             | Action             |
|-------------------|--------------------|
| Right button drag | Camera zoom in/out |
| Left button drag  | Camera orbits      |

| Key | Action                                 |
|-----|----------------------------------------|
|  1  | Texture Filter: Nearest                |
|  2  | Texture Filter: Bilinear               |
|  3  | Texture Filter: Trilinear              |
|  4  | Texture Filter: Anisotropic (max 4)    |
