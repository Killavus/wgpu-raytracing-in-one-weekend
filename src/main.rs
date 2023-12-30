use anyhow::Result;

use raytracing::GpuRaytracer;
use winit::event_loop::EventLoop;
use winit::window::Window;

use encase::ShaderType;
use nalgebra as na;

mod camera;
mod gpu;
mod ray;
mod raytracing;
mod render;
mod types;

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

async fn run(window: Window, event_loop: EventLoop<()>) -> Result<()> {
    let mut gpu = gpu::Gpu::from_window(&window).await?;
    let camera = Camera::new(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
        &window,
    );

    let gpu_camera: GpuCamera = GpuCamera::new(&gpu, camera)?;
    let renderer = Renderer::new(&gpu, &gpu_camera);
    let mut raytracer = GpuRaytracer::new(&gpu, &gpu_camera, 50, &renderer)?;

    raytracer.compute(&gpu, &gpu_camera)?;

    use winit::event::{Event, WindowEvent};

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
                        renderer.render(&gpu, &gpu_camera).unwrap();
                    }
                    WindowEvent::Resized(new_size) => {
                        gpu.on_resize((new_size.width, new_size.height));
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
