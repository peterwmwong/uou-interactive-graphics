# X-Project 6: Ray Traced Reflections/Environment Mapping

An alternative method of rendering reflections and environment mapping using Ray Tracing.

https://graphics.cs.utah.edu/courses/cs6610/spring2022/?prj=6

![X-Project 6 Teapot Ray Traced](./p6-rt-teapot.gif)

Laser pointer for assessing reflections!

![X-Project 6 Teapot Laser Pointer](./p6-rt-teapot-laser-pointer.gif)

# Usage

```sh
cargo run --bin proj-6-ray-traced-reflections [OPTIONAL: Path to Wavefront OBJ file]
```

## Examples

**IMPORTANT: Current working directory is the workspace directory (repository root), not the project directory.**

### Teapot model

```sh
cargo run --bin proj-6-ray-traced-reflections common-assets/teapot/teapot.obj
```

### Yoda model

```sh
cargo run --bin proj-6-ray-traced-reflections common-assets/yoda/yoda.obj
```

# Controls

| Mouse                          | Action                                       |
|--------------------------------|----------------------------------------------|
| Right button drag              | Camera zoom in/out                           |
| Left button drag               | Camera orbits                                |

| Key         | Action                                 |
|-------------|----------------------------------------|
|  0          | Ambient + Diffuse + Specular (default) |
|  1          | Normals                                |
|  2          | Ambient                                |
|  3          | Ambient + Diffuse                      |
|  4          | Specular                               |
|  P          | Toggle rendering laser pointer         |
|  Shift + P  | Toggle freeze laser pointer            |