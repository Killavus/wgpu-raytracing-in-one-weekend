use crate::gpu::Gpu;
use crate::types::*;
use anyhow::Result;
use encase::ShaderType;
use winit::window::Window;

pub enum CameraChange {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
}

#[derive(ShaderType)]
pub struct Camera {
    pub num_samples: u32,
    lookfrom: Vec3,
    lookat: Vec3,
    vup: Vec3,
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
                visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
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

    pub fn on_resize(&mut self, gpu: &Gpu, new_size: (u32, u32)) -> Result<()> {
        self.camera.on_resize(new_size);

        let Gpu { queue, .. } = gpu;
        let mut camera_buf = encase::UniformBuffer::new(vec![]);
        camera_buf.write(&self.camera)?;
        queue.write_buffer(&self.camera_buf, 0, camera_buf.into_inner().as_slice());
        Ok(())
    }

    pub fn on_camera_change(&mut self, gpu: &Gpu, change: CameraChange) -> Result<()> {
        self.camera.on_camera_change(change);

        let Gpu { queue, .. } = gpu;
        let mut camera_buf = encase::UniformBuffer::new(vec![]);
        camera_buf.write(&self.camera)?;
        queue.write_buffer(&self.camera_buf, 0, camera_buf.into_inner().as_slice());
        Ok(())
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
    pub fn new(lookfrom: Vec3, lookat: Vec3, vup: Vec3, num_samples: u32, window: &Window) -> Self {
        let size = window.inner_size();
        let (image_width, image_height) = (size.width as f32, size.height as f32);

        let aspect_ratio = image_width / image_height;

        let focal_length = (lookat - lookfrom).norm();
        let viewport_height = 2.0 * focal_length;
        let viewport_width = viewport_height * aspect_ratio;

        let w = (lookfrom - lookat).normalize();
        let u = vup.cross(&w).normalize();
        let v = w.cross(&u);

        let viewport_u = u * viewport_width;
        let viewport_v = -v * viewport_height;

        let delta_u = viewport_u / image_width;
        let delta_v = viewport_v / image_height;

        let top_left = lookfrom - (focal_length * w) - viewport_u / 2.0 - viewport_v / 2.0;
        let top_left_pixel = top_left + 0.5 * (delta_u + delta_v);

        Camera {
            lookfrom,
            lookat,
            vup,
            top_left_pixel,
            num_samples,
            delta_u,
            delta_v,
            width: image_width as u32,
            height: image_height as u32,
        }
    }

    pub fn on_resize(&mut self, (image_width, image_height): (u32, u32)) {
        let Self {
            lookfrom,
            lookat,
            vup,
            ..
        } = self;

        let (image_width, image_height) = (image_width as f32, image_height as f32);
        let aspect_ratio = image_width / image_height;

        let focal_length = (*lookat - *lookfrom).norm();
        let viewport_height = 2.0 * focal_length;
        let viewport_width = viewport_height * aspect_ratio;

        let w = (*lookfrom - *lookat).normalize();
        let u = vup.cross(&w).normalize();
        let v = w.cross(&u);

        let viewport_u = u * viewport_width;
        let viewport_v = -v * viewport_height;

        let delta_u = viewport_u / image_width;
        let delta_v = viewport_v / image_height;

        let top_left = *lookfrom - (focal_length * w) - viewport_u / 2.0 - viewport_v / 2.0;
        let top_left_pixel = top_left + 0.5 * (delta_u + delta_v);

        self.top_left_pixel = top_left_pixel;
        self.delta_u = delta_u;
        self.delta_v = delta_v;
        self.width = image_width as u32;
        self.height = image_height as u32;
    }

    const MOVE_FACTOR: f32 = 0.1;

    pub fn on_camera_change(&mut self, change: CameraChange) {
        let Self {
            lookfrom,
            lookat,
            vup,
            width,
            height,
            ..
        } = self;

        let w = (*lookfrom - *lookat).normalize();
        let u = vup.cross(&w).normalize();
        let v = w.cross(&u);

        match change {
            CameraChange::Forward => *lookfrom -= w * Self::MOVE_FACTOR,
            CameraChange::Backward => *lookfrom += w * Self::MOVE_FACTOR,
            CameraChange::Left => *lookfrom -= u * Self::MOVE_FACTOR,
            CameraChange::Right => *lookfrom += u * Self::MOVE_FACTOR,
            CameraChange::Up => *lookfrom += v * Self::MOVE_FACTOR,
            CameraChange::Down => *lookfrom -= v * Self::MOVE_FACTOR,
        }

        *lookat = *lookfrom - w;

        let (image_width, image_height) = (*width as f32, *height as f32);
        let aspect_ratio = image_width / image_height;

        let focal_length = (*lookat - *lookfrom).norm();
        let viewport_height = 2.0 * focal_length;
        let viewport_width = viewport_height * aspect_ratio;

        let w = (*lookfrom - *lookat).normalize();
        let u = vup.cross(&w).normalize();
        let v = w.cross(&u);

        let viewport_u = u * viewport_width;
        let viewport_v = -v * viewport_height;

        let delta_u = viewport_u / image_width;
        let delta_v = viewport_v / image_height;

        let top_left = *lookfrom - (focal_length * w) - viewport_u / 2.0 - viewport_v / 2.0;
        let top_left_pixel = top_left + 0.5 * (delta_u + delta_v);

        self.top_left_pixel = top_left_pixel;
        self.delta_u = delta_u;
        self.delta_v = delta_v;
        self.width = image_width as u32;
        self.height = image_height as u32;
    }
}
