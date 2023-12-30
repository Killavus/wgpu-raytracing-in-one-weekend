use anyhow::Result;

use winit::event_loop::EventLoop;
use winit::window::Window;

use encase::ShaderType;
use nalgebra as na;
use wgpu::{util::DeviceExt, TextureFormat};

mod camera;
mod gpu;
mod ray;
mod render;
mod types;

use crate::ray::Ray;
use crate::types::*;

use crate::render::Renderer;
use camera::{Camera, GpuCamera};

#[derive(ShaderType)]
struct Sphere {
    center: na::Vector3<f32>,
    radius: f32,
}

impl Sphere {
    fn new(center: na::Vector3<f32>, radius: f32) -> Self {
        Sphere { center, radius }
    }
}

use gpu::Gpu;

fn create_window() -> Result<(Window, EventLoop<()>)> {
    use winit::window::WindowBuilder;
    let event_loop = EventLoop::new()?;

    let window = WindowBuilder::new()
        .with_title("Raytracer")
        .with_inner_size(winit::dpi::LogicalSize::new(1200, 675))
        .build(&event_loop)?;

    Ok((window, event_loop))
}

fn initial_rays(camera: &Camera) -> Vec<Ray> {
    let mut rays = Vec::with_capacity((camera.width * camera.height) as usize);

    for y in 0..camera.height {
        for x in 0..camera.width {
            rays.push(camera.ray(x as f32, y as f32));
        }
    }

    rays
}

fn execute_raytracing(gpu: &Gpu, window: &Window) -> Result<()> {
    let Gpu { device, queue, .. } = gpu;

    let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
    });

    let camera = Camera::new(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
        window,
    );

    let gpu_camera = GpuCamera::new(gpu, camera)?;
    let renderer = Renderer::new(gpu, &gpu_camera);
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

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&compute_pipeline);
        cpass.set_bind_group(0, gpu_camera.bind_group(), &[]);
        cpass.set_bind_group(1, &compute_bg_ping, &[]);
        cpass.dispatch_workgroups(gpu_camera.camera().width, gpu_camera.camera().height, 1);
    }

    queue.submit(Some(encoder.finish()));
    device.poll(wgpu::Maintain::Wait);

    renderer.render(gpu, &gpu_camera)?;

    Ok(())
}

async fn run(window: Window, event_loop: EventLoop<()>) -> Result<()> {
    let gpu = gpu::Gpu::from_window(&window).await?;

    use winit::event::{Event, WindowEvent};
    let swapchain_capabilities = gpu.surface.get_capabilities(&gpu.adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: window.inner_size().width,
        height: window.inner_size().height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: vec![],
    };

    gpu.surface.configure(&gpu.device, &surface_config);

    let window = &window;
    event_loop.run(move |event: Event<()>, target| {
        if let Event::WindowEvent {
            window_id: window_event_id,
            event,
        } = event
        {
            if window_event_id == window.id() {
                match event {
                    WindowEvent::RedrawRequested => {
                        execute_raytracing(&gpu, window).unwrap();
                    }
                    WindowEvent::Resized(new_size) => {
                        surface_config.width = new_size.width;
                        surface_config.height = new_size.height;
                        gpu.surface.configure(&gpu.device, &surface_config);
                        window.request_redraw();
                    }
                    WindowEvent::CloseRequested => target.exit(),
                    _ => {}
                }
            }
        }
    })?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let (window, event_loop) = create_window()?;

    run(window, event_loop).await?;

    println!("Hello, world!");
    Ok(())
}
