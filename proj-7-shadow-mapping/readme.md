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
cargo run --bin proj-7-shadow-mapping proj-4-textures/assets/teapot.obj
```

### Yoda model

```sh
cargo run --bin proj-7-shadow-mapping proj-4-textures/assets/yoda/yoda.obj
```

# Controls

| Mouse                          | Action                                       |
|--------------------------------|----------------------------------------------|
| Right button drag              | Camera distance                              |
| Left button drag               | Camera orbits                                |
| Ctrl + Right button drag       | Light orbits                                 |
| Ctrl + Left button drag        | Light distance                               |
