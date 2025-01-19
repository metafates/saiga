pub mod brush;
pub mod context;

use std::cmp;
use std::sync::Arc;

use saiga_backend::{event::EventListener};
use winit::window::Window;
use saiga_backend::grid::Dimensions as TermDimensions;
use saiga_backend::term::{MIN_COLUMNS, MIN_SCREEN_LINES, Term};

#[derive(Debug)]
pub struct Display<'a> {
    pub window: Arc<Window>,
    pub size_info: SizeInfo,

    context: context::Context<'a>,
    brushes: brush::Brushes,
}

impl Display<'_> {
    pub async fn new(window: Window) -> Self {
        let window = Arc::new(window);
        let context = context::Context::new(window.clone()).await;
        let brushes = brush::Brushes::new(&context);

        let viewport_size = window.inner_size();

        let size_info = SizeInfo::new(
            viewport_size.width as f32,
            viewport_size.height as f32,
            30.0,
            30.0,
            0.0,
            0.0,
            false,
        );

        Self {
            context,
            window,
            brushes,
            size_info,
        }
    }

    pub fn draw<E: EventListener>(&mut self, terminal: &mut Term<E>) {
        match self.context.surface.get_current_texture() {
            Ok(surface) => self.draw_surface(surface, terminal),
            Err(e) => {
                if e == wgpu::SurfaceError::OutOfMemory {
                    panic!("Swapchain error: {e}. Rendering cannot continue.")
                }
            }
        }
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.context.set_size(width, height);

        self.brushes.resize(&mut self.context);
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.context.set_scale_factor(scale_factor);

        self.brushes.resize(&mut self.context);
    }

    fn draw_surface<E: EventListener>(
        &mut self,
        surface: wgpu::SurfaceTexture,
        terminal: &mut Term<E>,
    ) {
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let view = &surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                timestamp_writes: None,
                occlusion_query_set: None,
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.brushes.draw(&mut self.context, &mut rpass, terminal);
        }

        self.context.queue.submit(Some(encoder.finish()));
        surface.present();
    }
}

/// Terminal size info.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SizeInfo<T = f32> {
    /// Terminal window width.
    width: T,

    /// Terminal window height.
    height: T,

    /// Width of individual cell.
    cell_width: T,

    /// Height of individual cell.
    cell_height: T,

    /// Horizontal window padding.
    padding_x: T,

    /// Vertical window padding.
    padding_y: T,

    /// Number of lines in the viewport.
    screen_lines: usize,

    /// Number of columns in the viewport.
    columns: usize,
}

impl From<SizeInfo<f32>> for SizeInfo<u32> {
    fn from(size_info: SizeInfo<f32>) -> Self {
        Self {
            width: size_info.width as u32,
            height: size_info.height as u32,
            cell_width: size_info.cell_width as u32,
            cell_height: size_info.cell_height as u32,
            padding_x: size_info.padding_x as u32,
            padding_y: size_info.padding_y as u32,
            screen_lines: size_info.screen_lines,
            columns: size_info.screen_lines,
        }
    }
}

// impl From<SizeInfo<f32>> for WindowSize {
//     fn from(size_info: SizeInfo<f32>) -> Self {
//         Self {
//             num_cols: size_info.columns() as u16,
//             num_lines: size_info.screen_lines() as u16,
//             cell_width: size_info.cell_width() as u16,
//             cell_height: size_info.cell_height() as u16,
//         }
//     }
// }

impl<T: Clone + Copy> SizeInfo<T> {
    #[inline]
    pub fn width(&self) -> T {
        self.width
    }

    #[inline]
    pub fn height(&self) -> T {
        self.height
    }

    #[inline]
    pub fn cell_width(&self) -> T {
        self.cell_width
    }

    #[inline]
    pub fn cell_height(&self) -> T {
        self.cell_height
    }

    #[inline]
    pub fn padding_x(&self) -> T {
        self.padding_x
    }

    #[inline]
    pub fn padding_y(&self) -> T {
        self.padding_y
    }
}

impl SizeInfo<f32> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        width: f32,
        height: f32,
        cell_width: f32,
        cell_height: f32,
        mut padding_x: f32,
        mut padding_y: f32,
        dynamic_padding: bool,
    ) -> SizeInfo {
        if dynamic_padding {
            padding_x = Self::dynamic_padding(padding_x.floor(), width, cell_width);
            padding_y = Self::dynamic_padding(padding_y.floor(), height, cell_height);
        }

        let lines = (height - 2. * padding_y) / cell_height;
        let screen_lines = cmp::max(lines as usize, MIN_SCREEN_LINES);

        let columns = (width - 2. * padding_x) / cell_width;
        let columns = cmp::max(columns as usize, MIN_COLUMNS);

        SizeInfo {
            width,
            height,
            cell_width,
            cell_height,
            padding_x: padding_x.floor(),
            padding_y: padding_y.floor(),
            screen_lines,
            columns,
        }
    }

    #[inline]
    pub fn reserve_lines(&mut self, count: usize) {
        self.screen_lines = cmp::max(self.screen_lines.saturating_sub(count), MIN_SCREEN_LINES);
    }

    /// Check if coordinates are inside the terminal grid.
    ///
    /// The padding, message bar or search are not counted as part of the grid.
    #[inline]
    pub fn contains_point(&self, x: usize, y: usize) -> bool {
        x <= (self.padding_x + self.columns as f32 * self.cell_width) as usize
            && x > self.padding_x as usize
            && y <= (self.padding_y + self.screen_lines as f32 * self.cell_height) as usize
            && y > self.padding_y as usize
    }

    /// Calculate padding to spread it evenly around the terminal content.
    #[inline]
    fn dynamic_padding(padding: f32, dimension: f32, cell_dimension: f32) -> f32 {
        padding + ((dimension - 2. * padding) % cell_dimension) / 2.
    }
}

impl TermDimensions for SizeInfo {
    #[inline]
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    #[inline]
    fn screen_lines(&self) -> usize {
        self.screen_lines
    }

    #[inline]
    fn columns(&self) -> usize {
        self.columns
    }
}
