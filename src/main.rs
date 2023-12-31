use anyhow::Result;

use raytracing::GpuRaytracer;
use tokio::task::JoinHandle;
use winit::keyboard::KeyCode;
use winit::window::Window;
use winit::{dpi::PhysicalSize, event_loop::EventLoop};

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

use gpu::Gpu;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, RwLock};

struct App {
    renderer: RwLock<Renderer>,
    raytracer: RwLock<GpuRaytracer>,
    gpu: RwLock<Gpu>,
    gpu_camera: RwLock<GpuCamera>,
    window: Window,
    tracer_tx: Sender<TracerMsg>,
}

enum TracerMsg {
    Quit,
    Recompute,
}

async fn run(event_loop: EventLoop<()>, app: Arc<App>) -> Result<()> {
    use winit::event::{Event, WindowEvent};

    let window = &app.window;
    let app = app.clone();

    event_loop.run(move |event: Event<()>, target| {
        if let Event::WindowEvent {
            window_id: window_event_id,
            event,
        } = event
        {
            use winit::keyboard::PhysicalKey;

            if window_event_id == window.id() {
                match event {
                    WindowEvent::RedrawRequested => {
                        app.render().unwrap();
                    }
                    WindowEvent::Resized(new_size) => {
                        app.on_resize(new_size).unwrap();
                    }
                    WindowEvent::CloseRequested => {
                        app.quit().unwrap();
                        target.exit();
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if event.state == winit::event::ElementState::Pressed {
                            match event.physical_key {
                                PhysicalKey::Code(key) => match key {
                                    KeyCode::KeyR => {
                                        app.recompute().unwrap();
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    })?;

    Ok(())
}

impl App {
    fn render(&self) -> Result<()> {
        self.renderer
            .read()
            .unwrap()
            .render(&self.gpu.read().unwrap(), &self.gpu_camera.read().unwrap())?;

        Ok(())
    }

    fn perform(&self) -> Result<()> {
        let raytracer = self.raytracer.read().unwrap();
        let gpu = self.gpu.read().unwrap();
        let gpu_camera = self.gpu_camera.read().unwrap();
        raytracer.perform(&gpu, &gpu_camera, &self.window)?;

        Ok(())
    }

    fn recompute(&self) -> Result<()> {
        self.tracer_tx.send(TracerMsg::Recompute)?;
        Ok(())
    }

    fn quit(&self) -> Result<()> {
        self.tracer_tx.send(TracerMsg::Quit)?;
        Ok(())
    }

    fn clear(&self) {
        self.renderer
            .read()
            .unwrap()
            .clear(&self.gpu.read().unwrap());
    }

    fn on_resize(&self, new_size: PhysicalSize<u32>) -> Result<()> {
        let mut changed = false;
        {
            let mut gpu_camera = self.gpu_camera.write().unwrap();

            if new_size.width != gpu_camera.camera().width
                || new_size.height != gpu_camera.camera().height
            {
                changed = true;
                let mut gpu = self.gpu.write().unwrap();
                let mut renderer = self.renderer.write().unwrap();
                let mut raytracer = self.raytracer.write().unwrap();
                gpu.on_resize((new_size.width, new_size.height));
                gpu_camera.on_resize(&gpu, (new_size.width, new_size.height))?;
                renderer.on_resize(&gpu, &gpu_camera)?;
                raytracer.on_resize(&gpu, &renderer)?;
            }
        }

        if changed {
            self.recompute()?;
        }

        Ok(())
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    let (window, event_loop) = create_window()?;
    let gpu = gpu::Gpu::from_window(&window).await?;
    let camera = Camera::new(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
        10,
        &window,
    );

    let mut scene = Scene::default();
    let material_center = Material::new_lambertian(Vec3::new(0.1, 0.2, 0.5));
    let material_ground = Material::new_lambertian(Vec3::new(0.8, 0.8, 0.0));

    scene.new_sphere(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5), material_center);
    scene.new_sphere(
        Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0),
        material_ground,
    );

    let gpu_camera: GpuCamera = GpuCamera::new(&gpu, camera)?;
    let renderer = Renderer::new(&gpu, &gpu_camera);
    let raytracer: GpuRaytracer = GpuRaytracer::new(&gpu, &gpu_camera, 50, &renderer, scene)?;

    let gpu = RwLock::new(gpu);
    let gpu_camera = RwLock::new(gpu_camera);
    let renderer = RwLock::new(renderer);
    let raytracer = RwLock::new(raytracer);

    let (tracer_tx, tracer_rx) = channel();

    let app = Arc::new(App {
        renderer,
        raytracer,
        gpu,
        gpu_camera,
        window,
        tracer_tx,
    });

    let handle: JoinHandle<()>;
    {
        let app = app.clone();
        handle = tokio::task::spawn_blocking(move || {
            while let Ok(msg) = tracer_rx.recv() {
                match msg {
                    TracerMsg::Quit => break,
                    TracerMsg::Recompute => {
                        app.clear();
                        app.perform().unwrap();
                    }
                }
            }
        });
    }

    run(event_loop, app.clone()).await?;
    handle.await?;

    Ok(())
}
