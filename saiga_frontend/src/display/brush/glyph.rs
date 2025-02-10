use glyphon::{
    Attrs, Buffer, Cache, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport,
};
use wgpu::MultisampleState;

use crate::{backend::TermSize, color::Color, display::context, size::Size, term_font::TermFont};

pub struct Glyph {
    pub value: String,
    pub color: Color,
    pub top: f32,
    pub left: f32,
    pub width: u16,
    pub height: u16,
}

pub struct Brush {
    swash_cache: SwashCache,
    atlas: TextAtlas,
    viewport: Viewport,
    text_renderer: TextRenderer,
}

impl Brush {
    pub fn new(ctx: &context::Context) -> Self {
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&ctx.device);
        let viewport = Viewport::new(&ctx.device, &cache);
        let mut atlas =
            TextAtlas::with_color_mode(&ctx.device, &ctx.queue, &cache, ctx.format, ctx.color_mode);
        let text_renderer =
            TextRenderer::new(&mut atlas, &ctx.device, MultisampleState::default(), None);

        Brush {
            swash_cache,
            atlas,
            viewport,
            text_renderer,
        }
    }

    pub fn resize(&mut self, ctx: &context::Context) {
        let size = ctx.window.inner_size();

        self.viewport.update(&ctx.queue, Resolution {
            width: size.width,
            height: size.height,
        });
    }

    pub fn render(
        &mut self,
        ctx: &mut context::Context,
        font: &TermFont,
        rpass: &mut wgpu::RenderPass,
        glyphs: Vec<Glyph>,
    ) {
        let scale_factor = ctx.window.scale_factor();

        let mut buffers: Vec<(Buffer, Glyph)> = Vec::with_capacity(glyphs.len());
        let attrs = font.font_type.attributes();

        for glyph in glyphs {
            let mut buf = Buffer::new(
                &mut ctx.font_system,
                Metrics::relative(font.size, font.scale_factor),
            );

            buf.set_size(
                &mut ctx.font_system,
                Some(glyph.width as f32),
                Some(glyph.height as f32),
            );

            buf.set_text(&mut ctx.font_system, &glyph.value, attrs, Shaping::Advanced);

            buffers.push((buf, glyph));
        }

        let text_areas = buffers.iter().map(|(buf, glyph)| TextArea {
            buffer: buf,
            left: glyph.left * scale_factor as f32,
            top: glyph.top * scale_factor as f32,
            scale: scale_factor as f32,
            bounds: TextBounds {
                left: 0,
                top: 0,
                right: Size::<f32>::INFINITY.width as i32,
                bottom: Size::<f32>::INFINITY.height as i32,
            },
            default_color: glyph.color.into(),
            custom_glyphs: &[],
        });

        self.text_renderer
            .prepare(
                &ctx.device,
                &ctx.queue,
                &mut ctx.font_system,
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
