pub mod context;
mod renderer;

use std::sync::Arc;

use renderer::Renderer;
use saiga_backend::{event::EventListener, Terminal};
use winit::{dpi::PhysicalSize, window::Window};

#[derive(Debug)]
pub struct Display<'a> {
    renderer: Renderer<'a>,
    window: Arc<Window>,
}

impl Display<'_> {
    pub async fn new(window: Window) -> Self {
        let window = Arc::new(window);
        let renderer = Renderer::new(window.clone()).await;

        Self { renderer, window }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.renderer.resize(size);
    }

    pub fn draw<E: EventListener>(&mut self, terminal: &mut Terminal<E>) {
        self.renderer.draw_cells(terminal.grid().iter());
    }
}
