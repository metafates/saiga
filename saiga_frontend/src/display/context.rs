use std::sync::Arc;

use winit::window::Window;

#[derive(Debug)]
pub struct Context<'a> {
    pub device: wgpu::Device,
    pub surface: wgpu::Surface<'a>,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    pub alpha_mode: wgpu::CompositeAlphaMode,

    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

impl Context<'_> {
    pub async fn new<'a>(window: Arc<Window>) -> Context<'a> {
        let size = window.inner_size();
        let backends = wgpu::util::backend_bits_from_env().unwrap_or(wgpu::Backends::all());

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Request adapter");

        let format = wgpu::TextureFormat::Bgra8Unorm;

        let (device, queue) = request_device(&adapter).await;

        let alpha_mode = wgpu::CompositeAlphaMode::default();

        let mut ctx = Context {
            device,
            surface,
            queue,
            alpha_mode,
            format,
            width: size.width,
            height: size.height,
            scale_factor: window.scale_factor(),
        };

        ctx.set_size(size.width, size.height);

        ctx
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        self.configure_surface();
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;

        self.configure_surface();
    }

    fn configure_surface(&mut self) {
        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                width: self.width,
                height: self.height,
                view_formats: vec![],
                alpha_mode: self.alpha_mode,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
            },
        );
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
