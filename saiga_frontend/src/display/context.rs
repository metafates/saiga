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
}

impl Context<'_> {
    pub async fn new(window: Arc<Window>) -> Self {
        let backends = wgpu::Backends::all();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let color_mode = glyphon::ColorMode::Accurate;
        let format = wgpu::TextureFormat::Rgba16Float;

        let surface = instance.create_surface(window.clone()).unwrap();

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

        let mut display = Self {
            window,
            device,
            surface,
            queue,
            format,
            alpha_mode,
            color_mode,
            font_system: FontSystem::new(),
        };

        display.configure_surface();

        display
    }

    fn configure_surface(&mut self) {
        let size = self.window.inner_size();

        self.surface
            .configure(&self.device, &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
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
