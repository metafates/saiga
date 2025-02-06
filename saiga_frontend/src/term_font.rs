use crate::{font::Font, settings::FontSettings, size::Size};

#[derive(Debug, Clone)]
pub struct TermFont {
    pub size: f32,
    pub font_type: Font,
    pub scale_factor: f32,
    pub measure: Size<f32>,
}

impl TermFont {
    pub fn new(font_system: &mut glyphon::FontSystem, settings: FontSettings) -> Self {
        let measure = measure_font(
            font_system,
            settings.size,
            settings.scale_factor,
            settings.font_type,
        );

        Self {
            size: settings.size,
            font_type: settings.font_type,
            scale_factor: settings.scale_factor,
            measure,
        }
    }
}

fn measure_font(
    font_system: &mut glyphon::FontSystem,
    font_size: f32,
    scale_factor: f32,
    font_type: Font,
) -> Size<f32> {
    let mut buffer = glyphon::Buffer::new(
        font_system,
        glyphon::Metrics::relative(font_size, scale_factor),
    );

    let bounds = Size::<f32>::INFINITY;
    buffer.set_size(font_system, Some(bounds.width), Some(bounds.height));

    buffer.set_text(
        font_system,
        "@",
        font_type.attributes(),
        glyphon::Shaping::Advanced,
    );

    let (width, height) = buffer
        .layout_runs()
        .fold((0.0, 0.0), |(width, height), run| {
            (run.line_w.max(width), height + run.line_height)
        });

    Size::new(width, height)
}
