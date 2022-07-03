# Project 7: Shading Mapping

https://graphics.cs.utah.edu/courses/cs6610/spring2022/?prj=7

![Project 7 Yoda](./p7-yoda.gif)

# Usage

```sh
cargo run --bin proj-7-shadow-mapping [OPTIONAL: Path to Wavefront OBJ file]
```

## Examples

**IMPORTANT: Current working directory is the workspace directory (repository root), not the project directory.**

### Teapot model

```sh
cargo run --bin proj-7-shadow-mapping common-assets/teapot/teapot.obj
```

### Yoda model

```sh
cargo run --bin proj-7-shadow-mapping common-assets/yoda/yoda.obj
```

# Controls

| Mouse                          | Action                                       |
|--------------------------------|----------------------------------------------|
| Right button drag              | Camera distance                              |
| Left button drag               | Camera orbits                                |
| Ctrl + Right button drag       | Light orbits                                 |
| Ctrl + Left button drag        | Light distance                               |

| Key | Action                                 |
|-----|----------------------------------------|
|  0  | Ambient + Diffuse + Specular (default) |
|  1  | Normals                                |
|  2  | Ambient                                |
|  3  | Ambient + Diffuse                      |
|  4  | Specular                               |