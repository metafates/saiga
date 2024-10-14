use saiga_vte::ansi::handler::{Color, NamedColor};

#[derive(Clone, Copy)]
pub struct Cell {
    pub char: Option<char>,
    pub background: Color,
    pub foreground: Color,
    pub italic: bool,
    pub bold: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            char: None,
            background: Color::Named(NamedColor::Background),
            foreground: Color::Named(NamedColor::Foreground),
            italic: false,
            bold: false,
        }
    }
}

impl Cell {
    pub fn apply_template(&mut self, template: &Cell) {
        self.background = template.background;
        self.foreground = template.foreground;
        self.italic = template.italic;
        self.bold = template.bold;
    }
}
