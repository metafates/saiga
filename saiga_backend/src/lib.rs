use std::mem;

use event::{Event, EventListener};
use grid::{cell::UnderlineType, Dimensions, Grid};
use saiga_vte::ansi::handler::{Attribute, Handler, NamedMode, NamedPrivateMode, PrivateMode};
use unicode_width::UnicodeWidthChar;

pub mod event;
pub mod grid;

#[derive(Default)]
pub struct TerminalMode {
    pub alterantive_screen: bool,
    pub line_feed_new_line: bool,
    pub insert: bool,
}

pub struct Terminal<E: EventListener> {
    grid: Grid,
    secondary_grid: Grid,

    mode: TerminalMode,

    event_listener: E,
}

impl<E: EventListener> Terminal<E> {
    pub fn new(dimensions: Dimensions, event_listener: E) -> Self {
        Self {
            grid: Grid::with_dimensions(dimensions),
            secondary_grid: Grid::with_dimensions(dimensions),
            mode: TerminalMode::default(),
            event_listener,
        }
    }

    pub fn resize(&mut self, dimensions: Dimensions) {
        self.grid.resize(dimensions);
        self.secondary_grid.resize(dimensions);
    }

    fn swap_grids(&mut self) {
        mem::swap(&mut self.grid, &mut self.secondary_grid);
    }

    fn write_at_cursor(&mut self, c: char) {
        let template = self.grid.cursor.template;
        let cell_at_cursor = self.grid.cell_at_cursor_mut();

        cell_at_cursor.apply_template(&template);
        cell_at_cursor.char = Some(c);
    }
}

impl<E: EventListener> Handler for Terminal<E> {
    fn set_title(&mut self, title: &str) {
        self.event_listener
            .on_event(Event::SetTitle(title.to_string()));
    }

    fn set_cursor_shape(&mut self, shape: saiga_vte::ansi::handler::CursorShape) {
        todo!()
    }

    fn set_cursor_position(&mut self, position: saiga_vte::ansi::handler::Position) {
        self.grid.cursor.position.line = position.line;
        self.grid.cursor.position.column = position.column;
    }

    fn set_cursor_line(&mut self, line: saiga_vte::ansi::handler::Line) {
        self.grid.cursor.position.line = line;
    }

    fn set_cursor_column(&mut self, column: saiga_vte::ansi::handler::Column) {
        self.grid.cursor.position.column = column;
    }

    fn set_charset(&mut self, charset: saiga_vte::ansi::handler::CharsetIndex) {
        todo!()
    }

    fn set_clipboard(&mut self, clipboard: u8, payload: &[u8]) {
        todo!()
    }

    fn set_mode(&mut self, mode: saiga_vte::ansi::handler::Mode) {
        use saiga_vte::ansi::handler::Mode as HandlerMode;

        let HandlerMode::Named(mode) = mode else {
            return;
        };

        match mode {
            NamedMode::Insert => self.mode.insert = true,
            NamedMode::LineFeedNewLine => self.mode.line_feed_new_line = true,
        }
    }

    fn set_private_mode(&mut self, mode: saiga_vte::ansi::handler::PrivateMode) {
        let PrivateMode::Named(mode) = mode else {
            return;
        };

        match mode {
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
            NamedPrivateMode::UrgencyHints => todo!(),
            NamedPrivateMode::SwapScreenAndSetRestoreCursor => todo!(),
            NamedPrivateMode::BracketedPaste => todo!(),
            NamedPrivateMode::SyncUpdate => todo!(),
        }
    }

    fn set_attribute(&mut self, attribute: saiga_vte::ansi::handler::Attribute) {
        let template = &mut self.grid.cursor.template;

        match attribute {
            Attribute::Reset => template.reset_template(),
            Attribute::Bold => template.bold = true,
            Attribute::Dim => template.dim = true,
            Attribute::Italic => template.italic = true,
            Attribute::Underline => template.underline_type = Some(UnderlineType::Regular),
            Attribute::DoubleUnderline => template.underline_type = Some(UnderlineType::Double),
            Attribute::Undercurl => template.underline_type = Some(UnderlineType::Curl),
            Attribute::DottedUnderline => template.underline_type = Some(UnderlineType::Dotted),
            Attribute::DashedUnderline => template.underline_type = Some(UnderlineType::Dashed),
            Attribute::BlinkSlow => todo!(),
            Attribute::BlinkFast => todo!(),
            Attribute::Reverse => template.reverse = true,
            Attribute::Hidden => todo!(),
            Attribute::Strike => todo!(),
            Attribute::CancelBold => template.bold = false,
            Attribute::CancelBoldDim => {
                template.bold = false;
                template.dim = false;
            }
            Attribute::CancelItalic => template.italic = false,
            Attribute::CancelUnderline => template.underline_type = None,
            Attribute::CancelBlink => todo!(),
            Attribute::CancelReverse => template.reverse = false,
            Attribute::CancelHidden => todo!(),
            Attribute::CancelStrike => todo!(),
            Attribute::Foreground(color) => template.foreground = color,
            Attribute::Background(color) => template.background = color,
            Attribute::UnderlineColor(color) => {
                // TODO: do we need to reset the color if it's None?
                if let Some(color) = color {
                    template.underline_color = color
                }
            }
        }
    }

    fn reset_state(&mut self) {
        if self.mode.alterantive_screen {
            self.swap_grids();
        }

        self.mode = TerminalMode::default();
    }

    fn move_cursor(
        &mut self,
        direction: saiga_vte::ansi::handler::Direction,
        count: usize,
        reset_column: bool,
    ) {
        use saiga_vte::ansi::handler::Direction;

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
            todo!("wrap")
        }
    }

    fn put_tab(&mut self) {
        todo!()
    }

    fn put_hyperlink(&mut self, hyperlink: saiga_vte::ansi::handler::Hyperlink) {
        todo!()
    }

    fn put_blank(&mut self, count: usize) {
        todo!()
    }

    fn report_clipboard(&mut self, clipboard: u8) {
        todo!()
    }

    fn report_terminal(&mut self) {
        todo!()
    }

    fn report_mode(&mut self, mode: saiga_vte::ansi::handler::Mode) {
        todo!()
    }

    fn clear_screen(&mut self, mode: saiga_vte::ansi::handler::ScreenClearMode) {
        todo!()
    }

    fn clear_line(&mut self, mode: saiga_vte::ansi::handler::LineClearMode) {
        todo!()
    }

    fn save_cursor_position(&mut self) {
        self.grid.saved_cursor = Some(self.grid.cursor.clone());
    }

    fn restore_cursor_position(&mut self) {
        let Some(saved_cursor) = mem::take(&mut self.grid.saved_cursor) else {
            return;
        };

        self.grid.cursor = saved_cursor;
    }

    fn delete_lines(&mut self, count: usize) {
        todo!()
    }

    fn delete_chars(&mut self, count: usize) {
        todo!()
    }

    fn erase_chars(&mut self, count: usize) {
        todo!()
    }

    fn newline(&mut self) {
        self.linefeed();

        if self.mode.line_feed_new_line {
            self.carriage_return();
        }
    }

    fn carriage_return(&mut self) {
        self.grid.cursor.position.column = 0;
    }

    fn ring_bell(&mut self) {
        todo!()
    }

    fn backspace(&mut self) {
        if self.grid.cursor.position.column == 0 {
            return;
        }

        self.grid.cursor.position.column -= 1;
    }

    fn linefeed(&mut self) {
        let next_line = self.grid.cursor.position.line + 1;

        // TODO
        if next_line >= self.grid.height() {
            self.grid.cursor.position.line = next_line;
        }
    }

    fn substitute(&mut self) {
        todo!()
    }
}
