pub mod brush;
pub mod context;

use std::{mem, sync::Arc};

use brush::{Glyph, Rect};
use saiga_backend::{
    grid::{Dimensions, Grid},
    term::{
        cell::{Cell, Flags},
        TermMode,
    },
};
use saiga_vte::ansi::{Color, CursorShape, CursorStyle, NamedColor};
use wgpu::RenderPass;
use winit::window::Window;

use crate::{backend::TermSize, term_font::TermFont, terminal::Terminal, theme::Theme};

struct Frame<'a> {
    theme: &'a Theme,
    font: &'a TermFont,
    term_size: &'a TermSize,
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
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                timestamp_writes: None,
                occlusion_query_set: None,
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default()),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(bg.into()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.render_cells(
                &mut rpass,
                &Frame {
                    theme: &terminal.theme,
                    font: &terminal.font,
                    term_size: &term_size,
                    grid: term.grid(),
                    mode: term.mode(),
                    cursor_style: term.cursor_style(),
                },
            );
        });

        self.window().pre_present_notify();
        self.context.queue.submit(Some(encoder.finish()));
        surface.present();
    }

    fn render_cells(&mut self, rpass: &mut RenderPass<'_>, frame: &Frame) {
        let show_cursor = frame.mode.contains(TermMode::SHOW_CURSOR);

        let count = frame.grid.columns() * frame.grid.screen_lines();

        let mut rects = Vec::with_capacity(count);
        let mut glyphs = Vec::with_capacity(count);

        for indexed in frame.grid.display_iter() {
            let point = indexed.point;

            let (line, column) = (point.line, point.column);

            let x = column.0 * frame.term_size.cell_width as usize;
            let y =
                (line.0 + frame.grid.display_offset() as i32) * frame.term_size.cell_height as i32;

            let mut fg = frame.theme.get_color(indexed.fg);
            let mut bg = frame.theme.get_color(indexed.bg);

            let mut cursor_rect = None;

            if show_cursor && frame.grid.cursor.point == indexed.point {
                match frame.cursor_style.shape {
                    CursorShape::Block => mem::swap(&mut fg, &mut bg),
                    CursorShape::Underline => {
                        let height = frame.term_size.cell_height as f32 * 0.1;

                        cursor_rect = Some(Rect {
                            position: [
                                x as f32,
                                (y + frame.term_size.cell_height as i32) as f32 - height,
                            ],
                            color: fg.to_linear(),
                            size: [frame.term_size.cell_width as f32, height],
                        });
                    }
                    CursorShape::Beam => {
                        cursor_rect = Some(Rect {
                            position: [x as f32, y as f32],
                            color: frame
                                .theme
                                .get_color(Color::Named(NamedColor::Foreground))
                                .to_linear(),
                            size: [
                                frame.term_size.cell_width as f32 * 0.1,
                                frame.term_size.cell_height as f32,
                            ],
                        });
                    }
                    CursorShape::HollowBlock => todo!(),
                    CursorShape::Hidden => {}
                };
            }

            let rect = Rect {
                position: [x as f32, y as f32],
                color: bg.to_linear(),
                size: [
                    frame.term_size.cell_width as f32,
                    frame.term_size.cell_height as f32,
                ],
            };

            rects.push(rect);

            if let Some(cursor_rect) = cursor_rect {
                rects.push(cursor_rect);
            }

            if !indexed.c.is_whitespace() {
                let (bold, italic) = if indexed.flags.contains(Flags::BOLD_ITALIC) {
                    (true, true)
                } else if indexed.flags.contains(Flags::BOLD) {
                    (true, false)
                } else if indexed.flags.contains(Flags::ITALIC) {
                    (false, true)
                } else {
                    (false, false)
                };

                let glyph = Glyph {
                    value: indexed.c.to_string(),
                    color: fg,
                    top: y as f32,
                    left: x as f32,
                    width: frame.font.measure.width,
                    height: frame.font.measure.height,
                    bold,
                    italic,
                };

                glyphs.push(glyph);
            }
        }

        self.rect_brush.render(&mut self.context, rpass, rects);
        self.glyph_brush
            .render(&mut self.context, frame.font, rpass, glyphs);
    }
}
