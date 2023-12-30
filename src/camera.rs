use crate::gpu::Gpu;
use crate::ray::Ray;
use crate::types::*;
use anyhow::Result;
use encase::ShaderType;
use winit::window::Window;

#[derive(ShaderType)]
pub struct Camera {
    lookfrom: Vec3,
    top_left_pixel: Vec3,
    delta_u: Vec3,
    delta_v: Vec3,
    pub width: u32,
    pub height: u32,
}

pub struct GpuCamera {
    camera: Camera,
    camera_buf: wgpu::Buffer,
    camera_bg: wgpu::BindGroup,
    camera_bgl: wgpu::BindGroupLayout,
}

impl GpuCamera {
    pub fn new(gpu: &Gpu, camera: Camera) -> Result<Self> {
        use wgpu::util::DeviceExt;
        let Gpu { device, .. } = gpu;

        let mut camera_buf = encase::UniformBuffer::new(vec![]);
        camera_buf.write(&camera)?;

        let camera_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: camera_buf.into_inner().as_slice(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buf.as_entire_binding(),
            }],
        });

        Ok(GpuCamera {
            camera,
            camera_buf,
            camera_bg,
            camera_bgl,
        })
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.camera_bgl
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.camera_bg
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }
}

impl Camera {
    pub fn new(lookfrom: Vec3, lookat: Vec3, vup: Vec3, window: &Window) -> Self {
        let size = window.inner_size();
        let (image_width, image_height) = (size.width as f32, size.height as f32);

        let aspect_ratio = image_width / image_height;

        let focal_length = (lookat - lookfrom).norm();
        let viewport_height = 2.0 * focal_length;
        let viewport_width = viewport_height * aspect_ratio;

        let w = (lookfrom - lookat).cross(&vup).normalize();
        let u = vup.cross(&w).normalize();
        let v = w.cross(&u);

        let viewport_u = u * viewport_width;
        let viewport_v = -v * viewport_height;

        let delta_u = viewport_u / image_width;
        let delta_v = viewport_v / image_height;

        let top_left = lookfrom - (focal_length * w) - viewport_u / 2.0 - viewport_v / 2.0;
        let top_left_pixel = top_left.add_scalar(0.5) + delta_u / 2.0 + delta_v / 2.0;

        Camera {
            lookfrom,
            top_left_pixel,
            delta_u,
            delta_v,
            width: image_width as u32,
            height: image_height as u32,
        }
    }

    pub fn ray(&self, u: f32, v: f32) -> Ray {
        let pixel = self.top_left_pixel + (u * self.delta_u) + (v * self.delta_v);
        let origin = self.lookfrom;

        let direction = pixel - origin;

        Ray::new(origin, direction)
    }
}
