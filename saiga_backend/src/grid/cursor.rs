type Line = saiga_vte::ansi::handler::Line;
type Column = saiga_vte::ansi::handler::Column;

#[derive(Default)]
pub struct Position {
    pub line: Line,
    pub column: Column,
}

#[derive(Default)]
pub struct Cursor {
    pub position: Position,
}
