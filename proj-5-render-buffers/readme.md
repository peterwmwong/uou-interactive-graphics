https://graphics.cs.utah.edu/courses/cs6610/spring2022/?prj=5

![Project 5 Checkerboard](./p5-checkerboard.gif)
![Project 5 Yoda](./p5-yoda.gif)

# Usage

```sh
cargo run --bin proj-4-textures [OPTIONAL: Path to Wavefront OBJ file]
```

## Examples

**IMPORTANT: Current working directory is the workspace directory (repository root), not the project directory.**

### Checkerboard

Good for comparing different texture filtering (see [Controls](#controls) for changing Texture Filter modes)

```sh
cargo run --bin proj-5-render-buffers
```

### Teapot model

```sh
cargo run --bin proj-4-textures proj-4-textures/assets/teapot.obj
```

### Yoda model

```sh
cargo run --bin proj-4-textures proj-4-textures proj-4-textures/assets/yoda/yoda.obj
```

# Controls

| Mouse                          | Action                                       |
|--------------------------------|----------------------------------------------|
| Right button drag              | Camera zoom in/out                           |
| Left button drag               | Camera orbits                                |
| Alt/Option + Right button drag | Camera zoom in/out model rendered to texture |
| Alt/Option + Left button drag  | Camera orbits model rendered to texture      |

|   Key   | Action                                 |
|---------|----------------------------------------|
|    1    | Texture Filter: Nearest                |
|    2    | Texture Filter: Bilinear               |
|    3    | Texture Filter: Trilinear              |
|    4    | Texture Filter: Anisotropic (max 4)    |
| Alt + 0 | Ambient + Diffuse + Specular (default) |
| Alt + 1 | Normals                                |
| Alt + 2 | Ambient                                |
| Alt + 3 | Ambient + Diffuse                      |
| Alt + 4 | Specular                               |
