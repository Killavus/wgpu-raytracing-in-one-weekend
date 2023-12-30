pub struct Gpu {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
}

use anyhow::Result;
use winit::window::Window;

impl Gpu {
    pub async fn from_window(window: &Window) -> Result<Self> {
        get_gpu(window).await
    }

    pub fn on_resize(&mut self, new_size: (u32, u32)) {
        self.surface_config.width = new_size.0;
        self.surface_config.height = new_size.1;
        self.surface.configure(&self.device, &self.surface_config);
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

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: window.inner_size().width,
        height: window.inner_size().height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(&device, &surface_config);

    Ok(Gpu {
        instance,
        surface,
        adapter,
        device,
        queue,
        surface_config,
    })
}
