use std::{fmt::Debug, sync::Arc};

use saiga_backend::grid::Dimensions;
use winit::window::Window;

use super::SizeInfo;

const USE_WEB_COLORS: bool = false;

#[derive(Debug)]
pub struct Context<'a> {
    pub device: wgpu::Device,
    pub surface: wgpu::Surface<'a>,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    pub alpha_mode: wgpu::CompositeAlphaMode,
    pub color_mode: glyphon::ColorMode,

    pub size: SizeInfo,
}

impl Context<'_> {
    pub async fn new<'a>(window: Arc<Window>) -> Context<'a> {
        let window_size = window.inner_size();
        let backends = wgpu::util::backend_bits_from_env().unwrap_or(wgpu::Backends::all());

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let (color_mode, format) = if USE_WEB_COLORS {
            (glyphon::ColorMode::Web, wgpu::TextureFormat::Bgra8Unorm)
        } else {
            (
                glyphon::ColorMode::Accurate,
                wgpu::TextureFormat::Bgra8UnormSrgb,
            )
        };

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

        let size_info = SizeInfo::new(
            window_size.width as f32,
            window_size.height as f32,
            window.scale_factor(),
            20.0,
            45.0,
            0.0,
            0.0,
            false,
        );

        let mut ctx = Context {
            device,
            surface,
            queue,
            alpha_mode,
            format,
            color_mode,
            size: size_info,
        };

        ctx.set_size(window_size.width, window_size.height);

        ctx
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.size = SizeInfo::new(
            width as f32,
            height as f32,
            self.size.scale_factor,
            self.size.cell_width,
            self.size.cell_height,
            self.size.padding_x,
            self.size.padding_y,
            false,
        );

        self.configure_surface();
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.size.scale_factor = scale_factor;

        self.configure_surface();
    }

    fn configure_surface(&mut self) {
        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                width: self.size.width as u32,
                height: self.size.height as u32,
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
