use crate::display::context;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use saiga_backend::{event::EventListener, grid, term::Term};
use std::fmt::Debug;
use log::warn;
use wgpu::{MultisampleState, TextureFormat};
use saiga_backend::index::Point;
use saiga_backend::term::cell::ResetDiscriminant;
use saiga_vte::ansi::handler;
use saiga_vte::ansi::handler::Rgb;

const FONT_SIZE: u32 = 16;

pub struct Brush {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    buffers: Vec<Buffer>,
    viewport: Viewport,
    text_renderer: TextRenderer,
}

impl Debug for Brush {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Glyph Brush")
    }
}

impl Brush {
    pub fn new(ctx: &context::Context) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&ctx.device);
        let viewport = Viewport::new(&ctx.device, &cache);
        let mut atlas =
            TextAtlas::with_color_mode(&ctx.device, &ctx.queue, &cache, ctx.format, ctx.color_mode);
        let text_renderer =
            TextRenderer::new(&mut atlas, &ctx.device, MultisampleState::default(), None);

        Self {
            font_system,
            swash_cache,
            atlas,
            viewport,
            text_renderer,
            buffers: vec![],
        }
    }

    pub fn resize(&mut self, ctx: &mut context::Context) {
        let buffers = Vec::new();

        self.buffers = buffers;
    }

    pub fn draw<E: EventListener>(
        &mut self,
        ctx: &mut context::Context,
        rpass: &mut wgpu::RenderPass,
        terminal: &mut Term<E>,
    ) {
        self.viewport.update(
            &ctx.queue,
            Resolution {
                width: ctx.width,
                height: ctx.height,
            },
        );

        let mut buffers: Vec<(Buffer, Point, Color)> = Vec::new();

        for cell in terminal.grid().display_iter() {
            let mut buffer = Buffer::new(&mut self.font_system, Metrics::relative(16.0, 1.0));

            buffer.set_size(&mut self.font_system, Some(30.0), Some(30.0));
            buffer.set_text(
                &mut self.font_system,
                cell.c.to_string().as_str(),
                Attrs::new().family(Family::Monospace),
                Shaping::Basic,
            );

            let colors = terminal.colors();
            let fg = match cell.fg {
                handler::Color::Named(named) => colors[named],
                handler::Color::Indexed(index) => colors[index as usize],
                handler::Color::Spec(rgb) => Some(rgb),
            }.unwrap_or(Rgb::new(255, 255, 255));

            buffers.push((buffer, cell.point, Color::rgb(fg.r, fg.g, fg.b)));
        }

        // TODO: remove hardcode
        let text_areas: Vec<TextArea> = buffers
            .iter()
            .map(|(buf, pos, color)| TextArea {
                buffer: buf,
                left: pos.column.0 as f32 * 30.0,
                top: pos.line.0 as f32 * 30.0,
                scale: ctx.scale_factor as f32,
                bounds: TextBounds {
                    left: (pos.column.0 * 30) as i32,
                    top: pos.line.0 * 30,
                    right: (pos.column.0 * 30 + 30) as i32,
                    bottom: pos.line.0 * 30 + 60,
                },
                default_color: *color,
                custom_glyphs: &[],
            })
            .collect();

        self.text_renderer
            .prepare(
                &ctx.device,
                &ctx.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .unwrap();

        self.text_renderer
            .render(&self.atlas, &self.viewport, rpass)
            .unwrap();
    }
}
