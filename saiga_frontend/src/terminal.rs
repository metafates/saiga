use crate::{backend::Backend, settings::Settings, term_font::TermFont, theme::Theme};

pub struct Terminal {
    pub font: TermFont,
    pub theme: Theme,
    pub backend: Backend,
}

impl Terminal {
    pub fn new(
        font_system: &mut glyphon::FontSystem,
        backend: Backend,
        settings: Settings,
    ) -> Self {
        Self {
            font: TermFont::new(font_system, settings.font),
            theme: Theme::new(settings.theme),
            backend,
        }
    }
}
