use saiga_backend::{event::EventListener, grid::PositionedCell, Terminal};
use saiga_vte::ansi::handler::Rgb;

use super::context;

pub mod glyph;
pub mod math;
pub mod rect;

#[derive(Debug)]
pub struct Brushes {
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
        self.glyph_brush.resize(ctx);
    }

    pub fn draw<E: EventListener>(
        &mut self,
        ctx: &mut context::Context,
        rpass: &mut wgpu::RenderPass,
        terminal: &mut Terminal<E>,
    ) {
        // TODO: make this value non-hardcoded
        const SIZE: f32 = 30.0;

        let grid = terminal.grid();

        let mut rects: Vec<rect::Rect> = Vec::with_capacity(grid.width() * grid.height());

        for c in grid.iter() {
            let color = terminal.get_color(c.value.background);

            rects.push(rect::Rect {
                position: [
                    c.position.column as f32 * (SIZE / 2.0),
                    c.position.line as f32 * SIZE,
                ],
                color: rgb_to_wgpu_color(color),
                size: [SIZE, SIZE],
            })
        }

        self.rect_brush.draw(ctx, rpass, rects);
        self.glyph_brush.draw(ctx, rpass, terminal);
    }
}

fn rgb_to_wgpu_color(rgb: Rgb) -> [f32; 4] {
    [
        rgb.r as f32 / u8::MAX as f32,
        rgb.g as f32 / u8::MAX as f32,
        rgb.b as f32 / u8::MAX as f32,
        1.0,
    ]
}
