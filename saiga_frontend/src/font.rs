use iced::{Font, Pixels, Size};
use iced_core::{
    alignment::{Horizontal, Vertical},
    text::{LineHeight, Paragraph, Shaping as TextShaping, Wrapping},
    Text,
};
use iced_graphics::text::paragraph;

use crate::settings::FontSettings;

#[derive(Debug, Clone)]
pub struct TermFont {
    pub(crate) size: f32,
    pub(crate) font_type: Font,
    pub(crate) scale_factor: f32,
    pub(crate) measure: Size<f32>,
}

impl TermFont {
    pub fn new(settings: FontSettings) -> Self {
        Self {
            size: settings.size,
            font_type: settings.font_type,
            scale_factor: settings.scale_factor,
            measure: measure_font(settings.size, settings.scale_factor, settings.font_type),
        }
    }
}

fn measure_font(font_size: f32, scale_factor: f32, font_type: Font) -> Size<f32> {
    let paragraph = paragraph::Paragraph::with_text(Text {
        content: "@",
        font: font_type,
        size: Pixels(font_size),
        vertical_alignment: Vertical::Center,
        horizontal_alignment: Horizontal::Center,
        shaping: TextShaping::Advanced,
        line_height: LineHeight::Relative(scale_factor),
        bounds: Size::INFINITY,
        wrapping: Wrapping::Glyph,
    });

    paragraph.min_bounds()
}
