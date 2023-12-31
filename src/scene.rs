use crate::types::*;
use anyhow::Result;
use encase::{ArrayLength, ShaderType};

#[derive(ShaderType, Clone, Copy, Debug)]
pub struct Sphere {
    center: Vec3,
    radius: f32,
}

#[derive(ShaderType, Clone, Copy, Debug)]
struct SceneSphere {
    mat_id: u32,
    sphere: Sphere,
}

#[derive(ShaderType)]
struct GpuMats {
    length: ArrayLength,
    #[size(runtime)]
    mats: Vec<Material>,
}

#[derive(ShaderType)]
struct GpuSpheres {
    length: ArrayLength,
    #[size(runtime)]
    spheres: Vec<SceneSphere>,
}

#[derive(ShaderType, Default, PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct Material {
    mat_type: u32,
    albedo: Vec3,
    fuzz: f32,
    refract_idx: f32,
}

impl Material {
    pub fn new_lambertian(albedo: Vec3) -> Self {
        Material {
            mat_type: 0,
            albedo,
            ..Default::default()
        }
    }

    // This is for Debug only.
    #[allow(unused)]
    pub fn new_normal_map() -> Self {
        Material {
            mat_type: 3,
            ..Default::default()
        }
    }

    pub fn new_metal(albedo: Vec3, fuzz: f32) -> Self {
        Material {
            mat_type: 1,
            albedo,
            fuzz,
            ..Default::default()
        }
    }

    pub fn new_dielectric(refract_idx: f32) -> Self {
        Material {
            mat_type: 2,
            refract_idx,
            ..Default::default()
        }
    }
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Sphere { center, radius }
    }
}

#[derive(Default, Debug)]
pub struct Scene {
    spheres: Vec<SceneSphere>,
    mats: Vec<Material>,
}

type StorageBuf = encase::StorageBuffer<Vec<u8>>;

impl Scene {
    pub fn new_sphere(&mut self, sphere: Sphere, material: Material) {
        let mut mat_id: u32 = u32::MAX;
        if let Some(found_id) = self.mats.iter().position(|m| *m == material) {
            mat_id = found_id as u32;
        }

        if mat_id == u32::MAX {
            mat_id = self.mats.len() as u32;
            self.mats.push(material);
        }

        self.spheres.push(SceneSphere { mat_id, sphere });
    }

    pub fn into_gpu_buffers(self) -> Result<(StorageBuf, StorageBuf)> {
        let Scene { spheres, mats } = self;

        let mut spheres_buf = encase::StorageBuffer::new(vec![]);
        spheres_buf.write(&GpuSpheres {
            length: ArrayLength,
            spheres,
        })?;

        let mut mats_buf = encase::StorageBuffer::new(vec![]);
        mats_buf.write(&GpuMats {
            length: ArrayLength,
            mats,
        })?;

        Ok((spheres_buf, mats_buf))
    }
}
