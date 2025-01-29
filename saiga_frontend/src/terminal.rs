use iced::Settings;

use crate::{font::TermFont, theme::Theme};

pub struct Terminal {
    pub(crate) font: TermFont,
    pub(crate) theme: Theme,
}

impl Terminal {
    pub fn new(settings: Settings) -> Self {
        Self {}
    }
}
