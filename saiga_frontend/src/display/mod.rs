pub mod brush;
pub mod context;

use std::sync::Arc;

use brush::Rect;
use saiga_vte::ansi::handler::{Color, NamedColor};
use wgpu::RenderPass;
use winit::window::Window;

use crate::{size::Size, terminal::Terminal};

pub struct Display<'a> {
    pub context: context::Context<'a>,
    pub rect_brush: brush::RectBrush,
}

impl Display<'_> {
    pub async fn new(window: Arc<Window>) -> Self {
        let ctx = context::Context::new(window).await;
        let rect_brush = brush::RectBrush::new(&ctx);

        Self {
            context: ctx,
            rect_brush,
        }
    }

    pub fn window(&self) -> &Window {
        &self.context.window
    }

    pub fn render(&mut self, terminal: &mut Terminal) {
        match self.context.surface.get_current_texture() {
            Ok(surface) => self.render_surface(surface, terminal),
            Err(e) => {
                if e == wgpu::SurfaceError::OutOfMemory {
                    panic!("rendering cannot continue: swapchain error: {e}")
                }
            }
        }
    }

    fn render_surface(&mut self, surface: wgpu::SurfaceTexture, terminal: &mut Terminal) {
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let view = &surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let bg = terminal
                .theme
                .get_color(Color::Named(NamedColor::Background));

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                timestamp_writes: None,
                occlusion_query_set: None,
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(bg.into()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.render_rects(&mut rpass, terminal);
        }

        self.context.queue.submit(Some(encoder.finish()));
        surface.present();
    }

    fn render_rects(&mut self, rpass: &mut RenderPass<'_>, terminal: &mut Terminal) {
        let term_size = terminal.backend.size();

        let cell_width = term_size.cell_width as f32;
        let cell_height = term_size.cell_height as f32;
        let cell_size = Size::new(cell_width, cell_height);

        let grid = terminal.backend.prev_grid();

        let rects: Vec<_> = grid
            .display_iter()
            .map(|indexed| {
                let point = indexed.point;

                let (line, column) = (point.line, point.column);

                let x = column.0 as f32 * cell_width;
                let y = (line.0 as f32 + grid.display_offset() as f32) * cell_height;

                let color = terminal.theme.get_color(indexed.bg);

                Rect {
                    position: [x, y],
                    color: [color.r, color.g, color.b, color.a],
                    size: [cell_size.width, cell_size.height],
                }
            })
            .collect();

        self.rect_brush.render(&mut self.context, rpass, rects);
    }
}
