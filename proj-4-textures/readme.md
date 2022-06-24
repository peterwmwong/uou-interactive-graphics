https://graphics.cs.utah.edu/courses/cs6610/spring2022/?prj=4

![Project 4 Teapot](./p4.gif)
![Project 4 Yoda](./p4-yoda.gif)

# Usage

```sh
cargo run --bin proj-4-textures [OPTIONAL: Path to Wavefront OBJ file]
```

## Examples

**IMPORTANT: Current working directory is the workspace directory (repository root), not the project directory.**

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
| Ctrl + Right button drag       | Light orbits                                 |

| Key | Action                                 |
|-----|----------------------------------------|
|  0  | Ambient + Diffuse + Specular (default) |
|  1  | Normals                                |
|  2  | Ambient                                |
|  3  | Ambient + Diffuse                      |
|  4  | Specular                               |
