use std::sync::Arc;

use glyphon::FontSystem;
use winit::window::Window;

pub struct Context<'a> {
    pub window: Arc<Window>,

    pub device: wgpu::Device,
    pub surface: wgpu::Surface<'a>,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    pub alpha_mode: wgpu::CompositeAlphaMode,
    pub color_mode: glyphon::ColorMode,
    pub font_system: FontSystem,

    // persistent
    pub texture: wgpu::Texture,
    pub texture_bind_group: wgpu::BindGroup,
}

impl Context<'_> {
    pub async fn new(window: Arc<Window>) -> Self {
        let backends = wgpu::Backends::all();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let color_mode = glyphon::ColorMode::Accurate;
        let format = wgpu::TextureFormat::Rgba16Float;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Request adapter");

        let (device, queue) = request_device(&adapter).await;

        let alpha_mode = wgpu::CompositeAlphaMode::default();

        let size = window.inner_size();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: None,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create texture sampler and bind group
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        let mut display = Self {
            window,
            device,
            surface,
            queue,
            format,
            alpha_mode,
            color_mode,
            texture,
            texture_bind_group,
            font_system: FontSystem::new(),
        };

        display.configure_surface();

        display
    }

    fn configure_surface(&mut self) {
        let size = self.window.inner_size();

        self.texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                .union(wgpu::TextureUsages::TEXTURE_BINDING)
                .union(wgpu::TextureUsages::COPY_SRC),
            label: None,
            view_formats: &[],
        });

        self.surface
            .configure(&self.device, &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::COPY_DST),
                format: self.format,
                width: size.width,
                height: size.height,
                view_formats: vec![],
                alpha_mode: self.alpha_mode,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
            });
    }

    pub fn sync_size(&mut self) {
        self.configure_surface();
    }
}

async fn request_device(adapter: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
    let result = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await;

    if let Ok(result) = result {
        return result;
    }

    let result = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                memory_hints: wgpu::MemoryHints::Performance,
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            },
            None,
        )
        .await;

    result.expect("request device")
}
