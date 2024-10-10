/// Terminal cursor shape.
#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, Hash)]
pub enum CursorShape {
    /// Cursor is a block like `▒`.
    #[default]
    Block,

    /// Cursor is an underscore like `_`.
    Underline,

    /// Cursor is a vertical bar `⎸`.
    Beam,

    /// Cursor is a box like `☐`.
    HollowBlock,

    /// Invisible cursor.
    Hidden,
}

pub type Column = usize;
pub type Line = i32;

pub struct Position {
    line: Line,
    column: Column,
}

pub trait Handler {
    fn set_title(&mut self, title: Option<String>);
    fn set_cursor_shape(&mut self, shape: CursorShape);

    fn set_cursor_position(&mut self, position: Position);

    fn ring_bell(&mut self);

    fn put_tab(&mut self);
    fn backspace(&mut self);
    fn linefeed(&mut self);
}
