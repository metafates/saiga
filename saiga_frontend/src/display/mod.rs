pub mod brush;
pub mod context;

use std::sync::Arc;

use saiga_backend::{event::EventListener, Terminal};
use winit::window::Window;

#[derive(Debug)]
pub struct Display<'a> {
    pub window: Arc<Window>,

    context: context::Context<'a>,
    brushes: brush::Brushes,
}

impl Display<'_> {
    pub async fn new(window: Window) -> Self {
        let window = Arc::new(window);
        let context = context::Context::new(window.clone()).await;
        let brushes = brush::Brushes::new(&context);

        Self {
            context,
            window,
            brushes,
        }
    }

    pub fn draw<E: EventListener>(&mut self, terminal: &mut Terminal<E>) {
        match self.context.surface.get_current_texture() {
            Ok(surface) => self.draw_surface(surface, terminal),
            Err(e) => {
                if e == wgpu::SurfaceError::OutOfMemory {
                    panic!("Swapchain error: {e}. Rendering cannot continue.")
                }
            }
        }
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.context.set_size(width, height);

        self.brushes.resize(&mut self.context);
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.context.set_scale_factor(scale_factor);

        self.brushes.resize(&mut self.context);
    }

    fn draw_surface<E: EventListener>(
        &mut self,
        surface: wgpu::SurfaceTexture,
        terminal: &mut Terminal<E>,
    ) {
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let view = &surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                timestamp_writes: None,
                occlusion_query_set: None,
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.brushes.draw(&mut self.context, &mut rpass, terminal);
        }

        self.context.queue.submit(Some(encoder.finish()));
        surface.present();
    }
}
