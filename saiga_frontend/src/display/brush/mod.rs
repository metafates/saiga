use saiga_backend::grid::Dimensions;
use saiga_backend::{event::EventListener, term::Term};
use saiga_vte::ansi::handler;
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
        terminal: &mut Term<E>,
    ) {
        let grid = terminal.grid();

        let mut rects: Vec<rect::Rect> = Vec::with_capacity(grid.columns() * grid.screen_lines());

        for c in grid.display_iter() {
            let colors = terminal.colors();

            let bg = match c.bg {
                handler::Color::Named(named) => colors[named],
                handler::Color::Indexed(index) => colors[index as usize],
                handler::Color::Spec(rgb) => Some(rgb),
            }
            .unwrap_or(Rgb::new(0, 0, 0));

            rects.push(rect::Rect {
                position: [
                    c.point.column.0 as f32 * ctx.size.width,
                    c.point.line.0 as f32 * ctx.size.height,
                ],
                color: rgb_to_wgpu_color(bg),
                size: [ctx.size.width, ctx.size.height],
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
