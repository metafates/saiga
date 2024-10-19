use saiga_vte::ansi::handler::{Color, NamedColor};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnderlineType {
    Regular,
    Double,
    Dotted,
    Dashed,
    Curl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cell {
    pub char: Option<char>,
    pub background: Color,
    pub foreground: Color,
    pub italic: bool,
    pub bold: bool,
    pub dim: bool,
    pub reverse: bool,
    pub underline_type: Option<UnderlineType>,
    pub underline_color: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            char: None,
            background: Color::Named(NamedColor::Background),
            foreground: Color::Named(NamedColor::Foreground),
            italic: false,
            dim: false,
            bold: false,
            underline_type: None,
            underline_color: Color::Named(NamedColor::Foreground),
            reverse: false,
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

    pub fn reset_template(&mut self) {
        // TODO: optimize
        self.apply_template(&Cell::default());
    }
}
