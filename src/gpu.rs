pub struct Gpu {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

use anyhow::Result;
use winit::window::Window;

impl Gpu {
    pub async fn from_window(window: &Window) -> Result<Self> {
        get_gpu(window).await
    }
}

async fn get_gpu(window: &Window) -> Result<Gpu> {
    let instance = wgpu::Instance::default();
    let surface = unsafe { instance.create_surface(&window)? };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .map_or(Err(anyhow::anyhow!("No adapter found")), Ok)?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: adapter.features(),
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await?;

    Ok(Gpu {
        instance,
        surface,
        adapter,
        device,
        queue,
    })
}
