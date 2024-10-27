mod brush;

use std::sync::Arc;

use saiga_backend::grid::PositionedCell;
use winit::window::Window;

use super::context::Context;

#[derive(Debug)]
pub struct Renderer<'a> {
    context: Context<'a>,
}

impl Renderer<'_> {
    pub async fn new(window: Arc<Window>) -> Self {
        let context = Context::new(window).await;

        Self { context }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.context.resize(size);
    }

    pub fn draw_cells<I: Iterator<Item = PositionedCell>>(&mut self, cells: I) {
        for cell in cells {
            self.draw_cell(&cell);
        }
    }

    fn draw_cell(&mut self, cell: &PositionedCell) {
        todo!()
    }
}
