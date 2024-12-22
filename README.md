GPU path-tracer for the [Bevy](https://github.com/bevyengine/bevy) game engine.

### Some features
- Everything in compute shaders (no rtx-cores)
- Acceleration structure (BLAS and TLAS) build on cpu and sent to gpu
- GGX shading model
- Next event estimation (using emissive meshes as light sources)
- Works on normal Bevy meshes and supports a subset of Bevy's `StandardMaterial`

### Example
![screenshot](./pulse_screenshot.png)
