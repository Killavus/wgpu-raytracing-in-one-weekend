use crate::types::*;
use crate::{camera::GpuCamera, gpu::Gpu, render::Renderer, scene::Scene};
use encase::ShaderType;

use anyhow::Result;

pub struct GpuRaytracer {
    max_bounces: usize,
    pipeline: wgpu::ComputePipeline,
    compute_bg: wgpu::BindGroup,
    spheres_buf: wgpu::Buffer,
    mats_buf: wgpu::Buffer,
    seed_buf: wgpu::Buffer,
    limits_buf: wgpu::Buffer,
    compute_bgl: wgpu::BindGroupLayout,
}

#[derive(ShaderType, Debug)]
struct SeedUniform {
    seed: Vec3U,
}

#[derive(ShaderType)]
struct LimitUniform {
    max_bounces: u32,
}

fn generate_seed() -> Vec3U {
    use rand::Rng;

    let mut rng = rand::thread_rng();
    let x = rng.gen();
    let y = rng.gen();
    let z = rng.gen();

    Vec3U::new(x, y, z)
}

impl GpuRaytracer {
    pub fn new(
        gpu: &Gpu,
        gpu_camera: &GpuCamera,
        max_bounces: usize,
        renderer: &Renderer,
        scene: Scene,
    ) -> Result<Self> {
        use wgpu::util::DeviceExt;

        let Gpu { device, .. } = gpu;

        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
        });

        let (spheres, mats) = scene.into_gpu_buffers()?;

        let spheres_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: spheres.into_inner().as_slice(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let mats_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: mats.into_inner().as_slice(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let seed_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: SeedUniform::min_size().get(),
            mapped_at_creation: false,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let mut limits = encase::UniformBuffer::new(vec![]);
        limits.write(&LimitUniform {
            max_bounces: max_bounces as u32,
        })?;

        let limits_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: limits.into_inner().as_slice(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let compute_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadWrite,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let compute_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &renderer
                            .scene_texture()
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: spheres_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: mats_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: seed_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: limits_buf.as_entire_binding(),
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
            pipeline: compute_pipeline,
            compute_bg,
            spheres_buf,
            seed_buf,
            mats_buf,
            limits_buf,
            compute_bgl,
        })
    }

    fn compute(&self, gpu: &Gpu, gpu_camera: &GpuCamera) -> Result<()> {
        let Gpu { device, queue, .. } = gpu;
        let mut seed_uniform = encase::UniformBuffer::new(vec![]);

        let seed_uniform_contents = SeedUniform {
            seed: generate_seed(),
        };

        seed_uniform.write(&seed_uniform_contents)?;
        queue.write_buffer(&self.seed_buf, 0, seed_uniform.into_inner().as_slice());

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, gpu_camera.bind_group(), &[]);
            cpass.set_bind_group(1, &self.compute_bg, &[]);
            cpass.dispatch_workgroups(gpu_camera.camera().width, gpu_camera.camera().height, 1);
        }

        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);
        Ok(())
    }

    pub fn on_resize(&mut self, gpu: &Gpu, renderer: &Renderer) -> Result<()> {
        let Gpu { device, .. } = gpu;

        let compute_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.compute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &renderer
                            .scene_texture()
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.spheres_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.mats_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.seed_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.limits_buf.as_entire_binding(),
                },
            ],
        });

        self.compute_bg = compute_bg;
        Ok(())
    }

    pub fn perform(
        &self,
        gpu: &Gpu,
        gpu_camera: &GpuCamera,
        window: &winit::window::Window,
    ) -> Result<()> {
        for _ in 0..gpu_camera.camera().num_samples {
            self.compute(gpu, gpu_camera)?;
            window.request_redraw();
        }

        Ok(())
    }
}
