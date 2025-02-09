pub mod brush;
pub mod context;

use std::{mem, sync::Arc};

use brush::{Glyph, Rect};
use saiga_backend::{
    grid::{Dimensions, Grid},
    term::{TermMode, cell::Cell},
};
use saiga_vte::ansi::handler::{Color, CursorShape, CursorStyle, NamedColor};
use wgpu::RenderPass;
use winit::window::Window;

use crate::{
    backend::{Damage, TermSize},
    term_font::TermFont,
    terminal::Terminal,
    theme::Theme,
};

struct RenderData<'a> {
    theme: &'a Theme,
    font: &'a TermFont,
    term_size: &'a TermSize,
    damage: &'a Damage,
    grid: &'a Grid<Cell>,
    mode: &'a TermMode,
    cursor_style: CursorStyle,
}

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
        let Some(ref mut backend) = terminal.backend else {
            return;
        };

        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let bg = terminal
            .theme
            .get_color(Color::Named(NamedColor::Background));

        let term_size = *backend.size();

        backend.with_term(|term| {
            let damage: Damage = term.damage().into();

            let is_full_damage = damage.is_full();

            {
                let load_op = if is_full_damage {
                    wgpu::LoadOp::Clear(bg.into())
                } else {
                    wgpu::LoadOp::Load
                };

                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self
                            .context
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: load_op,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

                self.render_cells(
                    &mut rpass,
                    &terminal.theme,
                    &terminal.font,
                    &term_size,
                    &damage,
                    term.grid(),
                    term.mode(),
                    term.cursor_style(),
                );
            }

            term.reset_damage();

            encoder.copy_texture_to_texture(
                self.context.texture.as_image_copy(),
                surface.texture.as_image_copy(),
                self.context.texture.size(),
            );
        });

        self.context.queue.submit(Some(encoder.finish()));
        surface.present();
    }

    fn render_cells(
        &mut self,
        rpass: &mut RenderPass<'_>,
        theme: &Theme,
        font: &TermFont,
        term_size: &TermSize,
        damage: &Damage,
        grid: &Grid<Cell>,
        mode: &TermMode,
        cursor_style: CursorStyle,
    ) {
        let cell_width = term_size.cell_width as f32;
        let cell_height = term_size.cell_height as f32;

        let show_cursor = mode.contains(TermMode::SHOW_CURSOR);

        let count = grid.columns() * grid.screen_lines();

        let mut rects = Vec::with_capacity(count);
        let mut glyphs = Vec::with_capacity(count);

        for indexed in grid
            .display_iter()
            .filter(|c| damage.contains(c.point.line.0 as usize, c.point.column.0))
        {
            let point = indexed.point;

            let (line, column) = (point.line, point.column);

            let x = column.0 as f32 * cell_width;
            let y = (line.0 as f32 + grid.display_offset() as f32) * cell_height;

            let mut fg = theme.get_color(indexed.fg);
            let mut bg = theme.get_color(indexed.bg);

            let mut cursor_rect = None;

            if show_cursor && grid.cursor.point == indexed.point {
                match cursor_style.shape {
                    CursorShape::Block => mem::swap(&mut fg, &mut bg),
                    CursorShape::Underline => {
                        let height = cell_height * 0.1;

                        cursor_rect = Some(Rect {
                            position: [x, y + cell_height - height],
                            color: fg.as_linear(),
                            size: [cell_width, height],
                        });
                    }
                    CursorShape::Beam => {
                        cursor_rect = Some(Rect {
                            position: [x, y],
                            color: fg.as_linear(),
                            size: [cell_width * 0.1, cell_height],
                        });
                    }
                    CursorShape::HollowBlock => todo!(),
                    CursorShape::Hidden => {}
                };
            }

            let rect = Rect {
                position: [x, y],
                color: bg.as_linear(),
                size: [cell_width, cell_height],
            };

            rects.push(rect);

            if let Some(cursor_rect) = cursor_rect {
                rects.push(cursor_rect);
            }

            if !indexed.c.is_whitespace() {
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
            .render(&mut self.context, font, rpass, glyphs);
    }
}
