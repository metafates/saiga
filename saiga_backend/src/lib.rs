use std::{collections::HashMap, mem};

use event::{Event, EventListener};
use grid::{cell::Cell, Column, Dimensions, Grid};
use log::{debug, trace};
use saiga_vte::ansi::handler::{
    Attribute, Charset, CharsetIndex, Color, Handler, LineClearMode, NamedColor, NamedPrivateMode,
    PrivateMode, Rgb,
};
use unicode_width::UnicodeWidthChar;

pub mod event;
pub mod grid;
pub mod pty;

#[derive(Default)]
pub struct TerminalMode {
    pub alterantive_screen: bool,
    pub line_feed_new_line: bool,
    pub insert: bool,
    pub bracketed_paste: bool,
    pub urgency_hints: bool,
}

pub struct ColorPalette {}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {}
    }
}

impl ColorPalette {
    pub fn get_color(&self, color: Color) -> Rgb {
        match color {
            Color::Spec(rgb) => rgb,
            Color::Named(named_color) => match named_color {
                NamedColor::Black => todo!(),
                NamedColor::Red => todo!(),
                NamedColor::Green => Rgb::new(0, 255, 0),
                NamedColor::Yellow => todo!(),
                NamedColor::Blue => todo!(),
                NamedColor::Magenta => todo!(),
                NamedColor::Cyan => todo!(),
                NamedColor::White => todo!(),
                NamedColor::BrightBlack => todo!(),
                NamedColor::BrightRed => todo!(),
                NamedColor::BrightGreen => Rgb::new(170, 255, 0),
                NamedColor::BrightYellow => todo!(),
                NamedColor::BrightBlue => todo!(),
                NamedColor::BrightMagenta => todo!(),
                NamedColor::BrightCyan => todo!(),
                NamedColor::BrightWhite => todo!(),
                NamedColor::Foreground => Rgb::new(255, 255, 255),
                NamedColor::Background => Rgb::new(0, 0, 0),
                NamedColor::Cursor => todo!(),
                NamedColor::DimBlack => todo!(),
                NamedColor::DimRed => todo!(),
                NamedColor::DimGreen => todo!(),
                NamedColor::DimYellow => todo!(),
                NamedColor::DimBlue => todo!(),
                NamedColor::DimMagenta => todo!(),
                NamedColor::DimCyan => todo!(),
                NamedColor::DimWhite => todo!(),
                NamedColor::BrightForeground => todo!(),
                NamedColor::DimForeground => todo!(),
            },
            Color::Indexed(index) => todo!(),
        }
    }
}

pub struct Terminal<E: EventListener> {
    grid: Grid,

    mode: TerminalMode,
    active_charset: CharsetIndex,
    color_palette: ColorPalette,

    event_listener: E,
}

impl<E: EventListener> Terminal<E> {
    pub fn new(dimensions: Dimensions, event_listener: E) -> Self {
        Self {
            grid: Grid::with_dimensions(dimensions),
            mode: TerminalMode::default(),
            active_charset: CharsetIndex::default(),
            color_palette: Default::default(),
            event_listener,
        }
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn resize(&mut self, dimensions: Dimensions) {
        self.grid.resize(dimensions);
    }

    pub fn get_color(&self, color: Color) -> Rgb {
        self.color_palette.get_color(color)
    }

    fn write_at_cursor(&mut self, c: char) {
        let c = self.grid.cursor.charsets[self.active_charset].map(c);
        let template = self.grid.cursor.template;
        let cell_at_cursor = self.grid.cell_at_cursor_mut();

        cell_at_cursor.apply_template(&template);
        cell_at_cursor.char = Some(c);
    }
}

impl<E: EventListener> Handler for Terminal<E> {
    fn move_cursor(
        &mut self,
        direction: saiga_vte::ansi::handler::Direction,
        count: usize,
        reset_column: bool,
    ) {
        use saiga_vte::ansi::handler::Direction;

        trace!(
            "move_cursor: direction={direction:?} count={count:?} reset_column={reset_column:?}"
        );

        match direction {
            Direction::Up => {
                self.grid.cursor.position.line =
                    self.grid.cursor.position.line.saturating_add(count)
            }
            Direction::Right => {
                self.grid.cursor.position.column =
                    self.grid.cursor.position.column.saturating_add(count)
            }
            Direction::Down => {
                self.grid.cursor.position.line =
                    self.grid.cursor.position.line.saturating_sub(count)
            }
            Direction::Left => {
                self.grid.cursor.position.column =
                    self.grid.cursor.position.column.saturating_sub(count)
            }
        };

        if reset_column {
            self.grid.cursor.position.column = 0;
        }
    }

    fn put_char(&mut self, c: char) {
        trace!("put_char: c={c:?}");

        let Some(width) = c.width() else { return };

        if width == 0 {
            todo!("handle zero width characters")
        }

        let grid_columns = self.grid.width();

        // shift cells after cursor to the right
        if self.grid.cursor.position.column + width < grid_columns {
            let cursor_line = self.grid.cursor.position.line;
            let cursor_column = self.grid.cursor.position.column;

            let row = &mut self.grid[cursor_line];

            for column in (cursor_column..(grid_columns - width)).rev() {
                row.swap(column + width, column);
            }
        }

        if width > 1 {
            todo!("wide chars")
        }

        self.write_at_cursor(c);

        if self.grid.cursor.position.column + 1 < grid_columns {
            self.grid.cursor.position.column += 1;
        } else {
            //todo!("wrap")
        }
    }

    fn newline(&mut self) {
        trace!("newline");

        self.linefeed();

        if self.mode.line_feed_new_line {
            self.carriage_return();
        }
    }

    fn carriage_return(&mut self) {
        trace!("carriage_return");

        self.grid.cursor.position.column = 0;
    }

    fn linefeed(&mut self) {
        trace!("linefeed");

        let next_line = self.grid.cursor.position.line + 1;

        if next_line < self.grid.height() {
            self.grid.cursor.position.line = next_line;
            return;
        }

        todo!()
    }

    fn set_title(&mut self, title: &str) {
        trace!("set_title: title={title:?}");

        self.event_listener
            .on_event(Event::SetTitle(title.to_string()));
    }

    fn set_cursor_shape(&mut self, shape: saiga_vte::ansi::handler::CursorShape) {
        todo!()
    }

    fn set_cursor_position(&mut self, position: saiga_vte::ansi::handler::Position) {
        todo!()
    }

    fn set_cursor_line(&mut self, line: saiga_vte::ansi::handler::Line) {
        todo!()
    }

    fn set_cursor_column(&mut self, column: saiga_vte::ansi::handler::Column) {
        todo!()
    }

    fn set_active_charset(&mut self, charset: saiga_vte::ansi::handler::CharsetIndex) {
        trace!("set_active_charset: charset={charset:?}");
        self.active_charset = charset;
    }

    fn set_clipboard(&mut self, clipboard: u8, payload: &[u8]) {
        trace!("set_clipboard: clipboard={clipboard:?} payload={payload:?}");

        todo!()
    }

    fn set_mode(&mut self, mode: saiga_vte::ansi::handler::Mode) {
        trace!("set_mode: mode={mode:?}");

        todo!()
    }

    fn set_private_mode(&mut self, mode: PrivateMode) {
        trace!("set_private_mode: {mode:?}");

        let mode = match mode {
            PrivateMode::Named(mode) => mode,
            PrivateMode::Unknown(mode) => {
                debug!("ignoring unknown private mode {mode} in set_private_mode");
                return;
            }
        };

        match mode {
            NamedPrivateMode::SwapScreenAndSetRestoreCursor => {}
            NamedPrivateMode::CursorKeys => todo!(),
            NamedPrivateMode::ColumnMode => todo!(),
            NamedPrivateMode::Origin => todo!(),
            NamedPrivateMode::LineWrap => todo!(),
            NamedPrivateMode::BlinkingCursor => todo!(),
            NamedPrivateMode::ShowCursor => todo!(),
            NamedPrivateMode::ReportMouseClicks => todo!(),
            NamedPrivateMode::ReportCellMouseMotion => todo!(),
            NamedPrivateMode::ReportAllMouseMotion => todo!(),
            NamedPrivateMode::ReportFocusInOut => todo!(),
            NamedPrivateMode::Utf8Mouse => todo!(),
            NamedPrivateMode::SgrMouse => todo!(),
            NamedPrivateMode::AlternateScroll => todo!(),
            NamedPrivateMode::UrgencyHints => self.mode.urgency_hints = true,
            NamedPrivateMode::BracketedPaste => self.mode.bracketed_paste = true,
            NamedPrivateMode::SyncUpdate => {}
        };
    }

    fn set_attribute(&mut self, attribute: Attribute) {
        trace!("set_attribute: attribute={attribute:?}");

        match attribute {
            Attribute::Reset => self.grid.cursor.template.reset_template(),
            Attribute::Bold => self.grid.cursor.template.bold = true,
            Attribute::Dim => todo!(),
            Attribute::Italic => self.grid.cursor.template.italic = true,
            Attribute::Underline => todo!(),
            Attribute::DoubleUnderline => todo!(),
            Attribute::Undercurl => todo!(),
            Attribute::DottedUnderline => todo!(),
            Attribute::DashedUnderline => todo!(),
            Attribute::BlinkSlow => todo!(),
            Attribute::BlinkFast => todo!(),
            Attribute::Reverse => todo!(),
            Attribute::Hidden => todo!(),
            Attribute::Strike => todo!(),
            Attribute::CancelBold => todo!(),
            Attribute::CancelBoldDim => todo!(),
            Attribute::CancelItalic => todo!(),
            Attribute::CancelUnderline => todo!(),
            Attribute::CancelBlink => todo!(),
            Attribute::CancelReverse => todo!(),
            Attribute::CancelHidden => todo!(),
            Attribute::CancelStrike => todo!(),
            Attribute::Foreground(color) => self.grid.cursor.template.foreground = color,
            Attribute::Background(color) => self.grid.cursor.template.background = color,
            Attribute::UnderlineColor(color) => todo!(),
        }
    }

    fn reset_state(&mut self) {
        trace!("reset_state");

        todo!()
    }

    fn put_tab(&mut self) {
        trace!("put_tab");

        todo!()
    }

    fn put_hyperlink(&mut self, hyperlink: saiga_vte::ansi::handler::Hyperlink) {
        trace!("put_hyperlink: hyperlink={hyperlink:?}");

        todo!()
    }

    fn put_blank(&mut self, count: usize) {
        trace!("put_blank: count={count:?}");

        todo!()
    }

    fn report_clipboard(&mut self, clipboard: u8) {
        trace!("report_clipboard: clipboard={clipboard:?}");

        todo!()
    }

    fn report_terminal(&mut self) {
        trace!("report_terminal");

        todo!()
    }

    fn report_mode(&mut self, mode: saiga_vte::ansi::handler::Mode) {
        trace!("report_mode: mode={mode:?}");

        todo!()
    }

    fn clear_screen(&mut self, mode: saiga_vte::ansi::handler::ScreenClearMode) {
        trace!("clear_screen: mode={mode:?}");

        todo!()
    }

    fn clear_line(&mut self, mode: LineClearMode) {
        trace!("clear_line: mode={mode:?}");

        let cursor = &self.grid.cursor;

        let (left, right) = match mode {
            LineClearMode::Right => (cursor.position.column, self.grid.width()),
            LineClearMode::Left => (0, cursor.position.column),
            LineClearMode::All => (0, self.grid.width()),
        };

        let line = cursor.position.line;
        for cell in &mut self.grid[line][left..right] {
            // TODO: create cell from template as a method
            let mut new_cell = Cell::default();
            new_cell.apply_template(cell);

            *cell = new_cell
        }
    }

    fn save_cursor_position(&mut self) {
        trace!("save_cursor_position");

        todo!()
    }

    fn restore_cursor_position(&mut self) {
        trace!("restore_cursor_position");

        todo!()
    }

    fn delete_lines(&mut self, count: usize) {
        trace!("delete_lines: count={count:?}");

        todo!()
    }

    fn delete_chars(&mut self, count: usize) {
        trace!("delete_chars: count={count:?}");

        todo!()
    }

    fn erase_chars(&mut self, count: usize) {
        trace!("erase_chars: count={count:?}");

        todo!()
    }

    fn ring_bell(&mut self) {
        trace!("ring_bell");

        todo!()
    }

    fn backspace(&mut self) {
        trace!("backspace");

        todo!()
    }

    fn substitute(&mut self) {
        trace!("substitute");

        todo!()
    }

    fn set_charset_index(
        &mut self,
        index: saiga_vte::ansi::handler::CharsetIndex,
        charset: saiga_vte::ansi::handler::Charset,
    ) {
        trace!("set_charset_index: index={index:?} charset={charset:?}");

        self.grid.cursor.charsets[index] = charset;
    }

    fn report_keyboard_mode(&mut self) {
        trace!("report_keyboard_mode");

        todo!()
    }
}
