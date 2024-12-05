use saiga_backend::{event::EventListener, grid::PositionedCell, Terminal};

use super::context;

pub mod glyph;
pub mod math;
pub mod rect;

#[derive(Debug)]
pub struct Brushes {
    // TODO: use glyphs
    glyph_brush: glyph::Brush,
    rect_brush: rect::Brush,
}

impl Brushes {
    pub fn new(ctx: &context::Context) -> Self {
        Self {
            glyph_brush: glyph::Brush::new(ctx),
            rect_brush: rect::Brush::new(ctx),
        }
    }

    pub fn resize(&mut self, ctx: &mut context::Context) {
        self.rect_brush.resize(ctx);
    }

    pub fn draw<E: EventListener>(
        &mut self,
        ctx: &mut context::Context,
        rpass: &mut wgpu::RenderPass,
        terminal: &mut Terminal<E>,
    ) {
        // TODO: make this value non-hardcoded
        const SIZE: f32 = 30.0;

        let rects = terminal
            .grid()
            .iter()
            .map(|c| {
                // TODO: use background
                let color = terminal.get_color(c.value.foreground);

                rect::Rect {
                    position: [
                        c.position.column as f32 * (SIZE / 2.0),
                        c.position.line as f32 * SIZE,
                    ],
                    color: [
                        color.r as f32 / u8::MAX as f32,
                        color.g as f32 / u8::MAX as f32,
                        color.b as f32 / u8::MAX as f32,
                        1.0,
                    ],
                    size: [SIZE, SIZE],
                }
            })
            .collect();

        self.rect_brush.draw(ctx, rpass, rects);
    }
}
