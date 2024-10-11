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
    pub line: Line,
    pub column: Column,
}

/// Identifiers which can be assigned to a graphic character set.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CharsetIndex {
    /// Default set, is designated as ASCII at startup.
    #[default]
    G0,
    G1,
    G2,
    G3,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Hyperlink {
    /// Identifier for the given hyperlink.
    pub id: Option<String>,
    /// Resource identifier of the hyperlink.
    pub uri: String,
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

/// Mode for clearing terminal.
///
/// Relative to cursor.
#[derive(Debug)]
pub enum ScreenClearMode {
    /// Clear below cursor.
    Below,
    /// Clear above cursor.
    Above,
    /// Clear entire terminal.
    All,
    /// Clear 'saved' lines (scrollback).
    Saved,
}

/// Mode for clearing line.
///
/// Relative to cursor.
#[derive(Debug)]
pub enum LineClearMode {
    /// Clear right of cursor.
    Right,
    /// Clear left of cursor.
    Left,
    /// Clear entire line.
    All,
}

pub trait Handler {
    fn set_title(&mut self, title: &str);
    fn set_cursor_shape(&mut self, shape: CursorShape);
    fn set_cursor_position(&mut self, position: Position);
    fn set_cursor_line(&mut self, line: Line);
    fn set_cursor_column(&mut self, column: Column);
    fn set_charset(&mut self, charset: CharsetIndex);
    fn set_clipboard(&mut self, clipboard: u8, payload: &[u8]);

    fn move_cursor(&mut self, direction: Direction, count: usize, reset_column: bool);

    fn put_char(&mut self, c: char);
    fn put_tab(&mut self);
    fn put_hyperlink(&mut self, hyperlink: Hyperlink);
    fn put_blank(&mut self, count: usize);

    fn write_clipboard(&mut self, clipboard: u8);
    fn write_terminal(&mut self);

    fn clear_screen(&mut self, mode: ScreenClearMode);
    fn clear_line(&mut self, mode: LineClearMode);

    fn save_cursor_position(&mut self);
    fn restore_cursor_position(&mut self);

    fn carriage_return(&mut self);
    fn ring_bell(&mut self);
    fn backspace(&mut self);
    fn linefeed(&mut self);
    fn substitute(&mut self);
}
