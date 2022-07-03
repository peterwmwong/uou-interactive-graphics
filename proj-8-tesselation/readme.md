# Project 8: Shading Mapping

https://graphics.cs.utah.edu/courses/cs6610/spring2022/?prj=8

![Project 8](./p8.gif)

# Usage

**IMPORTANT: Current working directory is the workspace directory (repository root), not the project directory.**

```sh
cargo run --bin proj-8-tesselation
```

# Controls

| Mouse                          | Action                                       |
|--------------------------------|----------------------------------------------|
| Right button drag              | Camera distance                              |
| Left button drag               | Camera orbits                                |
| Ctrl + Right button drag       | Light orbits                                 |
| Ctrl + Left button drag        | Light distance                               |

| Key        | Action                                 |
|------------|----------------------------------------|
| 0          | Ambient + Diffuse + Specular (default) |
| 1          | Normals                                |
| 2          | Ambient                                |
| 3          | Ambient + Diffuse                      |
| 4          | Specular                               |
| Spacebar   | Toggle rendering triangulation         |
| Up/Down    | Increase/Decrease displacement         |
| Right/Left | Increase/Decrease tesselation          |