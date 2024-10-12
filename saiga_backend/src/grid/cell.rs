use saiga_vte::ansi::handler::{Color, NamedColor};

pub struct Cell {
    pub char: Option<char>,
    pub foreground: Color,
    pub background: Color,
    pub flags: Flags,
}

#[derive(Default)]
pub struct Flags {
    pub bold: bool,
    pub italic: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            char: None,
            foreground: Color::Named(NamedColor::Foreground),
            background: Color::Named(NamedColor::Background),
            flags: Flags::default(),
        }
    }
}
