use crate::{font::TermFont, settings::Settings, theme::Theme};

pub struct Terminal {
    pub(crate) font: TermFont,
    pub(crate) theme: Theme,
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl Terminal {
    pub fn new(settings: Settings) -> Self {
        Self {
            font: TermFont::new(settings.font),
            theme: Theme::default(),
        }
    }
}
