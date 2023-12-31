use anyhow::Result;

use raytracing::GpuRaytracer;
use winit::event_loop::EventLoop;
use winit::window::Window;

mod camera;
mod gpu;
mod ray;
mod raytracing;
mod render;
mod scene;
mod types;

use camera::{Camera, GpuCamera};
use render::Renderer;
use scene::{Material, Scene, Sphere};
use types::*;

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

    let mut scene = Scene::default();
    let material_ground = Material::new_normal_map();

    scene.new_sphere(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5), material_ground);
    scene.new_sphere(
        Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0),
        material_ground,
    );

    let mut gpu_camera: GpuCamera = GpuCamera::new(&gpu, camera)?;
    let mut renderer = Renderer::new(&gpu, &gpu_camera);
    let mut raytracer = GpuRaytracer::new(&gpu, &gpu_camera, 50, &renderer, scene)?;

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
                        gpu_camera
                            .on_resize(&gpu, (new_size.width, new_size.height))
                            .unwrap();
                        renderer.on_resize(&gpu, &gpu_camera).unwrap();
                        raytracer.on_resize(&gpu, &gpu_camera, &renderer).unwrap();
                        raytracer.compute(&gpu, &gpu_camera).unwrap();

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

    Ok(())
}
