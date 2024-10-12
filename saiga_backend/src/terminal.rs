use std::{cell::LazyCell, collections::HashMap, mem};

use saiga_vte::ansi::handler::{Column, Handler, Line};
use unicode_width::UnicodeWidthChar;

use crate::{
    event::{Event, EventListener},
    grid::{cell::Cell, Grid},
};

pub struct Terminal<E: EventListener> {
    grid: Grid,
    event_listener: E,
}

impl<E: EventListener> Terminal<E> {
    fn set_char_at_cursor(&mut self, c: char) {
        let cell = self.grid.cell_at_cursor_mut();

        cell.char = Some(c);
    }
}

impl<E: EventListener> Handler for Terminal<E> {
    fn set_title(&mut self, title: &str) {
        self.event_listener
            .event(Event::SetTitle(title.to_string()));
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
                    self.grid.cursor.position.line.saturating_add(count as Line)
            }
            Direction::Right => {
                self.grid.cursor.position.column = self
                    .grid
                    .cursor
                    .position
                    .column
                    .saturating_add(count as Column)
            }
            Direction::Down => {
                self.grid.cursor.position.line =
                    self.grid.cursor.position.line.saturating_sub(count as Line)
            }
            Direction::Left => {
                self.grid.cursor.position.column = self
                    .grid
                    .cursor
                    .position
                    .column
                    .saturating_sub(count as Column)
            }
        }

        if reset_column {
            self.grid.cursor.position.column = 0;
        }
    }

    fn put_char(&mut self, c: char) {
        let Some(width) = c.width() else {
            return;
        };

        if width == 0 {
            todo!("handle zero width")
        }

        if self.grid.cursor.position.column + width < self.grid.dimensions.columns {
            self.set_char_at_cursor(c);
        } // TODO: else wrap
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

    fn write_clipboard(&mut self, clipboard: u8) {
        todo!()
    }

    fn write_terminal(&mut self) {
        todo!()
    }

    fn clear_screen(&mut self, mode: saiga_vte::ansi::handler::ScreenClearMode) {
        todo!()
    }

    fn clear_line(&mut self, mode: saiga_vte::ansi::handler::LineClearMode) {
        todo!()
    }

    fn save_cursor_position(&mut self) {
        todo!()
    }

    fn restore_cursor_position(&mut self) {
        if let Some(saved) = mem::take(&mut self.grid.saved_cursor) {
            self.grid.cursor = saved
        }
    }

    fn carriage_return(&mut self) {
        todo!()
    }

    fn ring_bell(&mut self) {
        todo!("bell")
    }

    fn backspace(&mut self) {
        if self.grid.cursor.position.column == 0 {
            return;
        }

        self.grid.cursor.position.column -= 1;
    }

    fn linefeed(&mut self) {
        todo!()
    }

    fn substitute(&mut self) {
        todo!()
    }
}
