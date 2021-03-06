# Project 6: Environment Mapping

https://graphics.cs.utah.edu/courses/cs6610/spring2022/?prj=6

![Project 6 Teapot](./p6-teapot.gif)

# Usage

```sh
cargo run --bin proj-6-environment-mapping [OPTIONAL: Path to Wavefront OBJ file]
```

## Examples

**IMPORTANT: Current working directory is the workspace directory (repository root), not the project directory.**

### Teapot model

```sh
cargo run --bin proj-6-environment-mapping common-assets/teapot/teapot.obj
```

### Yoda model

```sh
cargo run --bin proj-6-environment-mapping common-assets/yoda/yoda.obj
```

### Sphere model

```sh
cargo run --bin proj-6-environment-mapping proj-6-environment-mapping/assets/sphere.obj
```

# Controls

| Mouse                          | Action                                       |
|--------------------------------|----------------------------------------------|
| Right button drag              | Camera zoom in/out                           |
| Left button drag               | Camera orbits                                |

| Key | Action                                 |
|-----|----------------------------------------|
|  0  | Ambient + Diffuse + Specular (default) |
|  1  | Normals                                |
|  2  | Ambient                                |
|  3  | Ambient + Diffuse                      |
|  4  | Specular                               |