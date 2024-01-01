# WGPU Raytracer

Based on [Raytracing in One Weekend](https://raytracing.github.io/books/RayTracingInOneWeekend.html) book.

Missing features:

- Defocus blur (depth-of-field effect). It is very simple to implement - I did not wanted to complicate camera code.
- Vertical FOV change support. I've decided to not add it since we already can move a look-from point of camera.

Added features:

- Ability to move camera using `WASD` (forward/backward/left/right) + `QZ` (up/down) keys.

### Approach

Computation is done on GPU using [wgpu](https://github.com/gfx-rs/wgpu).

All ray tracing computation is done in compute shader (located in `src/compute.wgsl`) which is performed `N` times where `N` is number of samples (configurable when creating camera). In one render pass `1/N`-th of color is being calculated - all rays are being traced up to `M` - max bounces (configurable when creating raytracer module).

Shader-level random functions are stolen from [cornell sample of WebGPU samples page](https://webgpu.github.io/webgpu-samples/samples/cornell)

### License

MIT.
