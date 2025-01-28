use crate::display::context;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use log::warn;
use saiga_backend::grid::{Dimensions, Indexed};
use saiga_backend::index::{Line, Point};
use saiga_backend::term::cell::{Cell, ResetDiscriminant};
use saiga_backend::{event::EventListener, grid, term::Term};
use saiga_vte::ansi::handler;
use saiga_vte::ansi::handler::Rgb;
use std::fmt::Debug;
use wgpu::{MultisampleState, TextureFormat};

const FONT_SIZE: f32 = 16.0;

pub struct Brush {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
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

        Brush {
            font_system,
            swash_cache,
            atlas,
            viewport,
            text_renderer,
        }
    }

    pub fn resize(&mut self, ctx: &context::Context) {
        self.viewport.update(
            &ctx.queue,
            Resolution {
                width: ctx.size.width as u32,
                height: ctx.size.height as u32,
            },
        );
    }

    pub fn draw<E: EventListener>(
        &mut self,
        ctx: &mut context::Context,
        rpass: &mut wgpu::RenderPass,
        terminal: &mut Term<E>,
    ) {
        let colors = terminal.colors();

        let spans = terminal.grid().display_iter().fold(
            Vec::<(String, Attrs, Line)>::new(),
            |mut spans, cell| {
                let mut attrs = Attrs::new().family(Family::Name("JetBrainsMono Nerd Font Mono"));

                if let Some((prev_str, _, prev_line)) = spans.last() {
                    if prev_str != "\n" && *prev_line != cell.point.line {
                        spans.push(("\n".to_string(), attrs, *prev_line));
                    }
                }

                let color = match cell.fg {
                    handler::Color::Named(named_color) => colors[named_color],
                    handler::Color::Spec(rgb) => Some(rgb),
                    handler::Color::Indexed(index) => colors[index as usize],
                };

                if let Some(color) = color {
                    attrs = attrs.color(Color::rgb(color.r, color.g, color.b))
                }

                spans.push((cell.c.to_string(), attrs, cell.point.line));

                spans
            },
        );

        let mut buffer = Buffer::new(&mut self.font_system, Metrics::relative(FONT_SIZE, 1.4));

        let columns = terminal.grid().columns();
        let lines = terminal.grid().screen_lines();

        let width = columns as f32 * ctx.size.cell_width;
        let height = lines as f32 * ctx.size.cell_height;

        buffer.set_size(&mut self.font_system, Some(width), Some(height));

        buffer.set_rich_text(
            &mut self.font_system,
            spans
                .iter()
                .map(|(string, attrs, _)| (string.as_str(), *attrs)),
            Attrs::new().family(Family::Name("JetBrainsMono Nerd Font Mono")),
            Shaping::Advanced,
        );

        let text_area = TextArea {
            buffer: &buffer,
            left: 0.0,
            top: 0.0,
            scale: ctx.size.scale_factor as f32,
            bounds: TextBounds {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            },
            default_color: Color::rgb(255, 255, 255),
            custom_glyphs: &[],
        };

        self.text_renderer
            .prepare(
                &ctx.device,
                &ctx.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                [text_area],
                &mut self.swash_cache,
            )
            .unwrap();

        self.text_renderer
            .render(&self.atlas, &self.viewport, rpass)
            .unwrap();
    }
}
