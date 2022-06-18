https://graphics.cs.utah.edu/courses/cs6610/spring2022/?prj=5

# Usage

```sh
> cargo run --bin proj-5-render-buffers [OPTIONAL: Path to Wavefront OBJ file]
```

Examples:

```sh
# Checkerboard pattern
# - Good for comparing different texture filtering (see below for switching between Texture Filter modes)
> cargo run --bin proj-5-render-buffers

# Project 4 Teapot model
> cargo run --bin proj-5-render-buffers proj-4-textures/assets/teapot

# Project 4 Yoda model
> cargo run --bin proj-5-render-buffers proj-4-textures/assets/yoda/yoda.obj
```

# Controls

| Mouse                          | Action                                       |
|--------------------------------|----------------------------------------------|
| Right button drag              | Camera zoom in/out                           |
| Left button drag               | Camera orbits                                |
| Alt/Option + Right button drag | Camera zoom in/out model rendered to texture |
| Alt/Option + Left button drag  | Camera orbits model rendered to texture      |

| Key | Action                                 |
|-----|----------------------------------------|
|  1  | Texture Filter: Nearest                |
|  2  | Texture Filter: Bilinear               |
|  3  | Texture Filter: Trilinear              |
|  4  | Texture Filter: Anisotropic (max 4)    |
