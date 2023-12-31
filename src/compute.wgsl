struct Camera {
    lookfrom: vec3<f32>,
    lookat: vec3<f32>,
    vup: vec3<f32>,
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

struct Spheres {
    length: u32,
    spheres: array<SceneSphere>,
};

struct SceneSphere {
    mat_id: u32,
    sphere: Sphere,
};

struct Sphere {
    center: vec3<f32>,
    radius: f32,
};

struct Material {
    mat_type: u32,
    albedo: vec3<f32>,
    fuzz: f32,
    refract_idx: f32,
};

struct Materials {
    length: u32,
    materials: array<Material>,
};



@group(0) @binding(0) var<uniform> cam: Camera;

@group(1) @binding(0) var<storage> rays_in: array<Ray>;
@group(1) @binding(1) var<storage, read_write> rays_out: array<Ray>;
@group(1) @binding(2) var raytraced: texture_storage_2d<rgba8unorm, write>;
@group(1) @binding(3) var<storage> spheresArr: Spheres;
@group(1) @binding(4) var<storage> materialsArr: Materials;

struct HitRecord {
    hit: bool,
    t: f32,
    point: vec3<f32>,
    normal: vec3<f32>,
    front_face: bool,
};

fn rayAt(ray: Ray, t: f32) -> vec3<f32> {
    return ray.origin + ray.direction * t;
}

fn inside(x: f32, x_min: f32, x_max: f32) -> bool {
    return x > x_min && x < x_max;
}

fn hitSphere(ray: Ray, sphere: Sphere, t_min: f32, t_max: f32) -> HitRecord {
    var oc = ray.origin - sphere.center;
    var a = dot(ray.direction, ray.direction);
    var b = 2.0 * dot(oc, ray.direction);
    var c = dot(oc, oc) - sphere.radius * sphere.radius;

    var discriminant = b * b - 4.0 * a * c;

    var record: HitRecord;
    record.hit = false;

    if discriminant == 0.0 {
        var t = -b / (2.0 * a);

        if inside(t, t_min, t_max) {
            record.hit = true;
            record.t = t;
            record.point = rayAt(ray, t);
            record.normal = (record.point - sphere.center) / sphere.radius;

            if dot(ray.direction, record.normal) < 0.0 {
                record.front_face = true;
            } else {
                record.normal = -record.normal;
                record.front_face = false;
            }
        }
    } else if discriminant >= 0.0 {
        var t1 = (-b - sqrt(discriminant)) / (2.0 * a);
        var t2 = (-b + sqrt(discriminant)) / (2.0 * a);

        var t: f32 = t1;
        if inside(t1, t_min, t_max) {
            t = t1;
            record.hit = true;
        } else if inside(t2, t_min, t_max) {
            t = t2;
            record.hit = true;
        }

        if record.hit {
            record.t = t;
            record.point = rayAt(ray, t);
            record.normal = (record.point - sphere.center) / sphere.radius;

            if dot(ray.direction, record.normal) < 0.0 {
                record.front_face = true;
            } else {
                record.normal = -record.normal;
                record.front_face = false;
            }
        }
    }

    if discriminant >= 0.0 {
        var t1 = (-b - sqrt(discriminant)) / (2.0 * a);
        var t2 = (-b + sqrt(discriminant)) / (2.0 * a);

        var t: f32 = t1;
        if t1 < t_max && t1 > t_min {
            t = t1;
            record.hit = true;
        } else if t2 < t_max && t2 > t_min {
            t = t2;
            record.hit = true;
        }

        if record.hit {
            record.t = t;
            record.point = rayAt(ray, t);
            record.normal = (record.point - sphere.center) / sphere.radius;

            if dot(ray.direction, record.normal) < 0.0 {
                record.front_face = true;
            } else {
                record.normal = -record.normal;
                record.front_face = false;
            }
        }
    }

    return record;
}

@compute
@workgroup_size(1)
fn raytrace(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var ray = rays_in[global_id.x + global_id.y * cam.width];

    if ray.finished == u32(1) {
        return;
    }

    var t_max = 100000000000.0;
    var sphereIdx = u32(100000);
    var hitRecord: HitRecord;

    for (var i = u32(0); i < spheresArr.length; i += u32(1)) {
        var record = hitSphere(ray, spheresArr.spheres[i].sphere, 0.001, t_max);

        if record.hit {
            t_max = record.t;
            sphereIdx = u32(i);
            hitRecord = record;
        }
    }

    if hitRecord.hit {
        var sphere = spheresArr.spheres[sphereIdx];
        var material = materialsArr.materials[sphere.mat_id];

        if material.mat_type == u32(3) {
            var color = (hitRecord.normal + 1.0) * 0.5;
            textureStore(raytraced, vec2<u32>(global_id.x, global_id.y), vec4<f32>(color, 1.0));
        } else {
            var color = vec3<f32>(1.0, 0.0, 0.0);
            textureStore(raytraced, vec2<u32>(global_id.x, global_id.y), vec4<f32>(color, 1.0));
        }
    } else {
        var unit_d = normalize(ray.direction);
        var t = 0.5 * (unit_d.y + 1.0);
        var color = mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.5, 0.7, 1.0), t);
        textureStore(raytraced, vec2<u32>(global_id.x, global_id.y), vec4<f32>(color, 1.0));
    }

    var outRay = rays_out[global_id.x + global_id.y * cam.width];

    outRay.direction = ray.direction;
    outRay.origin = ray.origin;
    outRay.finished = u32(1);
}
