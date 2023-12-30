use crate::{
    camera::{Camera, GpuCamera},
    gpu::Gpu,
    ray::Ray,
    render::Renderer,
};
use encase::ShaderType;

use anyhow::Result;

fn initial_rays(camera: &Camera) -> Vec<Ray> {
    let mut rays = Vec::with_capacity((camera.width * camera.height) as usize);

    for y in 0..camera.height {
        for x in 0..camera.width {
            rays.push(camera.ray(x as f32, y as f32));
        }
    }

    rays
}

pub struct GpuRaytracer {
    max_bounces: usize,
    ping: bool,
    pipeline: wgpu::ComputePipeline,
    ping_bg: wgpu::BindGroup,
    pong_bg: wgpu::BindGroup,
}

impl GpuRaytracer {
    pub fn new(
        gpu: &Gpu,
        gpu_camera: &GpuCamera,
        max_bounces: usize,
        renderer: &Renderer,
    ) -> Result<Self> {
        use wgpu::util::DeviceExt;

        let Gpu { device, .. } = gpu;

        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
        });

        let initial_rays: Vec<Ray> = initial_rays(gpu_camera.camera());

        let mut rays_buf_ping = encase::StorageBuffer::new(vec![]);
        rays_buf_ping.write(&initial_rays)?;

        let rays_ping_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: rays_buf_ping.into_inner().as_slice(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let rays_pong_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: Ray::min_size().get()
                * (gpu_camera.camera().width * gpu_camera.camera().height) as u64,
            mapped_at_creation: false,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let compute_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let compute_bg_ping = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: rays_ping_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rays_pong_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &renderer
                            .scene_texture()
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
            ],
        });

        let compute_bg_pong = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: rays_pong_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rays_ping_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &renderer
                            .scene_texture()
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
            ],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[gpu_camera.bind_group_layout(), &compute_bgl],
                    push_constant_ranges: &[],
                }),
            ),
            module: &compute_shader,
            entry_point: "raytrace",
        });

        Ok(Self {
            max_bounces,
            ping: true,
            pipeline: compute_pipeline,
            ping_bg: compute_bg_ping,
            pong_bg: compute_bg_pong,
        })
    }

    pub fn compute(&mut self, gpu: &Gpu, gpu_camera: &GpuCamera) -> Result<()> {
        let Gpu { device, queue, .. } = gpu;

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, gpu_camera.bind_group(), &[]);
            cpass.set_bind_group(
                1,
                if self.ping {
                    &self.ping_bg
                } else {
                    &self.pong_bg
                },
                &[],
            );
            cpass.dispatch_workgroups(gpu_camera.camera().width, gpu_camera.camera().height, 1);
        }

        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);

        Ok(())
    }
}
