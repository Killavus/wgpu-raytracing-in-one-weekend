struct Camera {
    num_samples: u32,
    lookfrom: vec3<f32>,
    lookat: vec3<f32>,
    vup: vec3<f32>,
    top_left_pixel: vec3<f32>,
    delta_u: vec3<f32>,
    delta_v: vec3<f32>,
    width: u32,
    height: u32,
};

@group(0) @binding(0) var<uniform> cam: Camera;
@group(1) @binding(0) var scene: texture_2d<f32>;
@group(1) @binding(1) var sceneSampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    var VERTEX: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0)
    );

    var TEX: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0)
    );

    out.clip_position = vec4<f32>(VERTEX[in_vertex_index], 0.0, 1.0);
    out.tex_coords = vec2<f32>(TEX[in_vertex_index]);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(scene, sceneSampler, in.tex_coords);
    return vec4<f32>(color.xyz / f32(cam.num_samples), 1.0);
}
