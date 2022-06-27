# Project 7: Shading Mapping

https://graphics.cs.utah.edu/courses/cs6610/spring2022/?prj=7

![Project 7 Teapot](./p7-teapot.gif)

# Usage

```sh
cargo run --bin proj-7-shadow-mapping [OPTIONAL: Path to Wavefront OBJ file]
```

## Examples

**IMPORTANT: Current working directory is the workspace directory (repository root), not the project directory.**

### Teapot model

```sh
cargo run --bin proj-7-shadow-mapping proj-4-textures/assets/teapot.obj
```

### Yoda model

```sh
cargo run --bin proj-7-shadow-mapping proj-4-textures/assets/yoda/yoda.obj
```

### Sphere model

```sh
cargo run --bin proj-7-shadow-mapping proj-6-environment-mapping/assets/sphere.obj
```

# Controls

| Mouse                          | Action                                       |
|--------------------------------|----------------------------------------------|
| Right button drag              | Camera zoom in/out                           |
| Left button drag               | Camera orbits                                |
