pub mod brush;
pub mod context;

use std::{mem, sync::Arc};

use brush::{Glyph, Rect};
use saiga_backend::grid::Dimensions;
use saiga_vte::ansi::handler::{Color, NamedColor};
use wgpu::RenderPass;
use winit::window::Window;

use crate::terminal::Terminal;

pub struct Display<'a> {
    pub context: context::Context<'a>,
    pub rect_brush: brush::RectBrush,
    pub glyph_brush: brush::GlyphBrush,
}

impl Display<'_> {
    pub async fn new(window: Arc<Window>) -> Self {
        let ctx = context::Context::new(window).await;
        let rect_brush = brush::RectBrush::new(&ctx);
        let glyph_brush = brush::GlyphBrush::new(&ctx);

        Self {
            context: ctx,
            rect_brush,
            glyph_brush,
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

    pub fn sync_size(&mut self) {
        self.context.sync_size();

        self.rect_brush.resize(&mut self.context);
        self.glyph_brush.resize(&self.context);
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

            self.render_cells(&mut rpass, terminal);
        }

        self.context.queue.submit(Some(encoder.finish()));
        surface.present();
    }

    fn render_cells(&mut self, rpass: &mut RenderPass<'_>, terminal: &mut Terminal) {
        let Some(ref backend) = terminal.backend else {
            return;
        };

        let term_size = backend.size();

        let cell_width = term_size.cell_width as f32;
        let cell_height = term_size.cell_height as f32;

        let frame = backend.prev_frame();
        let grid = &frame.grid;

        let count = grid.columns() * grid.screen_lines();

        let mut rects = Vec::with_capacity(count);
        let mut glyphs = Vec::with_capacity(count);

        for indexed in grid.display_iter() {
            let point = indexed.point;

            let (line, column) = (point.line, point.column);

            let x = column.0 as f32 * cell_width;
            let y = (line.0 as f32 + grid.display_offset() as f32) * cell_height;

            let mut fg = terminal.theme.get_color(indexed.fg);
            let mut bg = terminal.theme.get_color(indexed.bg);

            if frame.cursor == indexed.point {
                mem::swap(&mut fg, &mut bg);
            }

            let rect = Rect {
                position: [x, y],
                color: bg.as_linear(),
                size: [cell_width, cell_height],
            };

            rects.push(rect);

            if indexed.c != ' ' || indexed.c != '\t' {
                let glyph = Glyph {
                    value: indexed.c.to_string(),
                    color: fg,
                    top: y,
                    left: x,
                };

                glyphs.push(glyph);
            }
        }

        self.rect_brush.render(&mut self.context, rpass, rects);
        self.glyph_brush
            .render(&mut self.context, &terminal.font, rpass, glyphs);
    }
}
