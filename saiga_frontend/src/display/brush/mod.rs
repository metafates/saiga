use rand::Rng;
use saiga_backend::grid::PositionedCell;

use super::context;

pub mod math;
pub mod rect;

#[derive(Debug)]
pub struct Brushes {
    rect: rect::Brush,
}

impl Brushes {
    pub fn new(ctx: &context::Context) -> Self {
        Self {
            rect: rect::Brush::new(ctx),
        }
    }

    pub fn resize(&mut self, ctx: &mut context::Context) {
        self.rect.resize(ctx);
    }

    pub fn draw(
        &mut self,
        ctx: &mut context::Context,
        rpass: &mut wgpu::RenderPass,
        cells: Vec<PositionedCell>,
    ) {
        {
            const SIZE: f32 = 30.0;

            // TODO: remove later
            let mut rng = rand::thread_rng();

            let rects = cells
                .iter()
                .map(|c| rect::Rect {
                    position: [
                        c.position.column as f32 * (SIZE / 2.0),
                        c.position.line as f32 * SIZE,
                    ],
                    color: [
                        rng.gen_range(0..100) as f32 / 100.0,
                        rng.gen_range(0..100) as f32 / 100.0,
                        rng.gen_range(0..100) as f32 / 100.0,
                        1.0,
                    ],
                    size: [SIZE, SIZE],
                })
                .collect();

            self.rect.draw(ctx, rpass, rects);
        }
    }
}
