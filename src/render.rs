use crate::camera::GpuCamera;
use crate::gpu::Gpu;
use anyhow::Result;

pub struct Renderer {
    scene_tex: wgpu::Texture,
    sampler: wgpu::Sampler,
    pipeline: wgpu::RenderPipeline,
    render_bg: wgpu::BindGroup,
    render_bgl: wgpu::BindGroupLayout,
}

impl Renderer {
    pub fn new(gpu: &Gpu, gpu_camera: &GpuCamera) -> Self {
        let Gpu { device, .. } = gpu;

        let swap_format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("render.wgsl").into()),
        });

        let scene_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let camera = gpu_camera.camera();

        let scene_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: camera.width,
                height: camera.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let render_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let render_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &render_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &scene_tex.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&scene_sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[gpu_camera.bind_group_layout(), &render_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: swap_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            scene_tex,
            pipeline,
            render_bg,
            render_bgl,
            sampler: scene_sampler,
        }
    }

    pub fn on_resize(&mut self, gpu: &Gpu, gpu_camera: &GpuCamera) -> Result<()> {
        let Gpu { device, queue, .. } = gpu;
        let camera = gpu_camera.camera();

        let new_scene_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: camera.width,
                height: camera.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // let new_size = camera.width * camera.height;
        // let old_size = self.scene_tex.size().width * self.scene_tex.size().height;

        // let mut encoder =
        //     device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // encoder.copy_texture_to_texture(
        //     self.scene_tex.as_image_copy(),
        //     new_scene_tex.as_image_copy(),
        //     if new_size > old_size {
        //         self.scene_tex.size()
        //     } else {
        //         new_scene_tex.size()
        //     },
        // );

        // queue.submit(Some(encoder.finish()));

        self.scene_tex = new_scene_tex;
        self.render_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.render_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &self
                            .scene_tex
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        Ok(())
    }

    pub fn render(&self, gpu: &Gpu, gpu_camera: &GpuCamera) -> Result<()> {
        let Gpu {
            device,
            queue,
            surface,
            ..
        } = gpu;

        let frame = surface.get_current_texture()?;
        let frame_tex_view: wgpu::TextureView = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_tex_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, gpu_camera.bind_group(), &[]);
            rpass.set_bind_group(1, &self.render_bg, &[]);
            rpass.draw(0..4, 0..1);
        }

        queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub fn clear(&self, gpu: &Gpu) {
        let Gpu { device, queue, .. } = gpu;
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.clear_texture(&self.scene_tex, &wgpu::ImageSubresourceRange::default());
        queue.submit(Some(encoder.finish()));
    }

    pub fn scene_texture(&self) -> &wgpu::Texture {
        &self.scene_tex
    }
}
