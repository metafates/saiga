use crate::display::context;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use log::warn;
use saiga_backend::grid::Indexed;
use saiga_backend::index::Point;
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
        let lines = terminal.grid().display_iter().fold(
            Vec::<Vec<Indexed<&Cell>>>::new(),
            |mut lines, cell| match lines.last_mut() {
                Some(line) => {
                    if line.last().unwrap_or(&cell).point.line == cell.point.line {
                        line.push(cell);
                    } else {
                        lines.push(vec![cell]);
                    }

                    lines
                }
                None => vec![vec![cell]],
            },
        );

        let buffers: Vec<(Buffer, Point)> = lines
            .into_iter()
            .map(|line| {
                line.into_iter()
                    .fold((String::new(), Point::default()), |acc, cell| {
                        (acc.0 + cell.c.to_string().as_str(), cell.point)
                    })
            })
            .map(|(line, pos)| {
                let mut buffer =
                    Buffer::new(&mut self.font_system, Metrics::relative(FONT_SIZE, 1.3));

                buffer.set_size(
                    &mut self.font_system,
                    Some(ctx.size.cell_width * pos.column.0 as f32),
                    Some(ctx.size.cell_height),
                );

                buffer.set_text(
                    &mut self.font_system,
                    &line,
                    Attrs::new().family(Family::Name("JetBrainsMono Nerd Font")),
                    Shaping::Advanced,
                );

                (buffer, pos)
            })
            .collect();

        //let buffers: Vec<(Buffer, Point, Color)> = terminal
        //    .grid()
        //    .display_iter()
        //    .map(|cell| {
        //        let mut buffer =
        //            Buffer::new(&mut self.font_system, Metrics::relative(FONT_SIZE, 1.3));
        //
        //        buffer.set_size(
        //            &mut self.font_system,
        //            Some(ctx.size.cell_width),
        //            Some(ctx.size.cell_height),
        //        );
        //
        //        buffer.set_text(
        //            &mut self.font_system,
        //            cell.c.to_string().as_str(),
        //            Attrs::new().family(Family::Name("JetBrainsMono Nerd Font")),
        //            Shaping::Advanced,
        //        );
        //
        //        let colors = terminal.colors();
        //        let fg = match cell.fg {
        //            handler::Color::Named(named) => colors[named],
        //            handler::Color::Indexed(index) => colors[index as usize],
        //            handler::Color::Spec(rgb) => Some(rgb),
        //        }
        //        .unwrap_or(Rgb::new(255, 255, 255));
        //
        //        (buffer, cell.point, Color::rgb(fg.r, fg.g, fg.b))
        //    })
        //    .collect();

        let text_areas: Vec<TextArea> = buffers
            .iter()
            .map(|(buf, pos)| TextArea {
                buffer: buf,
                left: 0.0,
                top: pos.line.0 as f32 * ctx.size.cell_height,
                scale: ctx.size.scale_factor as f32,
                bounds: TextBounds {
                    left: 0,
                    top: pos.line.0 * ctx.size.cell_height as i32,
                    right: ((pos.column.0 + 1) * ctx.size.cell_width as usize) as i32,
                    bottom: (pos.line.0 + 2) * ctx.size.cell_height as i32,
                },
                default_color: Color::rgb(255, 255, 255),
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
