struct Camera {
    lookfrom: vec3<f32>,
    top_left_pixel: vec3<f32>,
    delta_u: vec3<f32>,
    delta_v: vec3<f32>,
    width: u32,
    height: u32,
};

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
    finished: u32,
};


@group(0) @binding(0) var<uniform> cam: Camera;

@group(1) @binding(0) var<storage> rays_in: array<Ray>;
@group(1) @binding(1) var<storage, read_write> rays_out: array<Ray>;
@group(1) @binding(2) var raytraced: texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(1)
fn raytrace(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var ray = rays_in[global_id.x + global_id.y * cam.width];

    if ray.finished == u32(1) {
        return;
    }

    var unit_d = normalize(ray.direction);
    var t = 0.5 * (unit_d.y + 1.0);
    var color = mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.5, 0.7, 1.0), t);

    textureStore(raytraced, vec2<u32>(global_id.x, global_id.y), vec4<f32>(color, 1.0));
    var outRay = rays_out[global_id.x + global_id.y * cam.width];

    outRay.direction = ray.direction;
    outRay.origin = ray.origin;
    outRay.finished = u32(1);
}
