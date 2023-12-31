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

struct SeedUniform {
    seed: vec3<u32>,
};

struct LimitsUniform {
    num_bounces: u32,
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

const pi: f32 = 3.14159265359;

@group(0) @binding(0) var<uniform> cam: Camera;
@group(1) @binding(0) var raytraced: texture_storage_2d<rgba32float, read_write>;
@group(1) @binding(1) var<storage> spheresArr: Spheres;
@group(1) @binding(2) var<storage> materialsArr: Materials;
@group(1) @binding(3) var<uniform> seed_uniform: SeedUniform;
@group(1) @binding(4) var<uniform> limits_uniform: LimitsUniform;


const MAT_LAMBERTIAN: u32 = u32(0);
const MAT_METAL: u32 = u32(1);
const MAT_DIELECTRIC: u32 = u32(2);
const MAT_NORMAL_MAP: u32 = u32(3);

struct HitRecord {
    hit: bool,
    t: f32,
    point: vec3<f32>,
    normal: vec3<f32>,
    front_face: bool,
};

var<private> rnd : vec3u;

// Initializes the random number generator.
fn init_rand(invocation_id: vec3<u32>) {
    var A = vec3<u32>(u32(1741651 * 1009),
        u32(140893 * 1609 * 13),
        u32(6521 * 983 * 7 * 2));
    rnd = (invocation_id * A) ^ seed_uniform.seed;
}

// Returns a random number between 0 and 1.
fn rand() -> f32 {
    var C = vec3<u32>(u32(60493 * 9377),
        u32(11279 * 2539 * 23),
        u32(7919 * 631 * 5 * 3));

    rnd = (rnd * C) ^ (rnd.yzx >> vec3(4u));
    return f32(rnd.x ^ rnd.y) / 4294967295.0; // 4294967295.0 is f32(0xffffffff). See #337
}

fn rand_unit_sphere() -> vec3<f32> {
    var u = rand();
    var v = rand();
    var theta = u * 2.0 * pi;
    var phi = acos(2.0 * v - 1.0);
    var r = pow(rand(), 1.0 / 3.0);
    var sin_theta = sin(theta);
    var cos_theta = cos(theta);
    var sin_phi = sin(phi);
    var cos_phi = cos(phi);
    var x = r * sin_phi * sin_theta;
    var y = r * sin_phi * cos_theta;
    var z = r * cos_phi;
    return vec3<f32>(x, y, z);
}

fn rayAt(ray: Ray, t: f32) -> vec3<f32> {
    return ray.origin + ray.direction * t;
}

fn inside(x: f32, x_min: f32, x_max: f32) -> bool {
    return x > x_min && x < x_max;
}

fn reflect(direction: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    return direction - 2.0 * dot(direction, normal) * normal;
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

fn nearZero(v: vec3<f32>) -> bool {
    var va = abs(v);
    var s = 1e-8;
    return va.x < s && va.y < s && va.z < s;
}

fn initRay(x: f32, y: f32) -> Ray {
    var origin = cam.lookfrom;
    var pixel = (cam.top_left_pixel + x * cam.delta_u + y * cam.delta_v);
    var direction = pixel - origin;

    var sampleDiff = (-0.5 * rand() * cam.delta_u) + (-0.5 * rand() * cam.delta_v);

    var ray: Ray;
    ray.origin = origin;
    ray.direction = direction + sampleDiff;

    return ray;
}

fn writePixel(x: u32, y: u32, color: vec3<f32>) {
    var current = textureLoad(raytraced, vec2<u32>(x, y)).rgb;
    textureStore(raytraced, vec2<u32>(x, y), vec4<f32>(current + color, 1.0));
}

fn reflectance(cosine: f32, ref_idx: f32) -> f32 {
    var r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
    r0 = r0 * r0;
    return r0 + (1.0 - r0) * pow((1.0 - cosine), 5.0);
}

fn refract(uv: vec3<f32>, n: vec3<f32>, etai_over_etat: f32) -> vec3<f32> {
    var cos_theta = dot(-uv, n);
    var r_out_parallel = etai_over_etat * (uv + cos_theta * n);
    var r_out_perp = -sqrt(1.0 - dot(r_out_parallel, r_out_parallel)) * n;
    return r_out_parallel + r_out_perp;
}

@compute
@workgroup_size(1)
fn raytrace(@builtin(global_invocation_id) global_id: vec3<u32>) {
    init_rand(global_id.xyz);
    var pixel = vec2<f32>(f32(global_id.x), f32(global_id.y));
    var ray = initRay(pixel.x, pixel.y);

    var energy = vec3<f32>(1.0, 1.0, 1.0);
    for (var b = u32(0); b <= limits_uniform.num_bounces; b += u32(1)) {
        var t_max = 100000000000.0;
        var sphereIdx = u32(100000);
        var hitRecord: HitRecord;
        hitRecord.hit = false;

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

            if material.mat_type == MAT_NORMAL_MAP {
                var color = (hitRecord.normal + 1.0) * 0.5;
                writePixel(global_id.x, global_id.y, energy * color);
                return;
            } else if material.mat_type == MAT_LAMBERTIAN {
                var direction = (hitRecord.normal + rand_unit_sphere());

                if nearZero(direction) {
                    direction = hitRecord.normal;
                }

                energy = energy * material.albedo;
                ray.origin = hitRecord.point;
                ray.direction = direction;
            } else if material.mat_type == MAT_METAL {
                energy = energy * material.albedo;
                ray.origin = hitRecord.point;
                ray.direction = reflect(ray.direction, hitRecord.normal) + material.fuzz * rand_unit_sphere();
            } else if material.mat_type == MAT_DIELECTRIC {
                var refraction_ratio = material.refract_idx;
                if hitRecord.front_face {
                    refraction_ratio = 1.0 / refraction_ratio;
                }

                var unit_direction = normalize(ray.direction);
                var cos_theta = min(dot(-unit_direction, hitRecord.normal), 1.0);
                var sin_theta = sqrt(1.0 - cos_theta * cos_theta);

                var cannot_refract = refraction_ratio * sin_theta > 1.0;
                var direction: vec3<f32>;

                if cannot_refract || reflectance(cos_theta, refraction_ratio) > rand() {
                    direction = reflect(unit_direction, hitRecord.normal);
                } else {
                    direction = refract(unit_direction, hitRecord.normal, refraction_ratio);
                }

                ray.origin = hitRecord.point;
                ray.direction = direction;
            } else {
                writePixel(global_id.x, global_id.y, vec3<f32>(1.0, 0.0, 0.0));
                return;
            }
        } else {
            var unit_d = normalize(ray.direction);
            var t = 0.5 * (unit_d.y + 1.0);
            var color = mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.5, 0.7, 1.0), t);
            writePixel(global_id.x, global_id.y, energy * color);
            return;
        }
    }

    writePixel(global_id.x, global_id.y, vec3<f32>(0.0, 0.0, 0.0));
}
