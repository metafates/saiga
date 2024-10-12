use std::ops::{Index, IndexMut};

use cell::Cell;
use cursor::Cursor;
use row::Row;
use saiga_vte::ansi::handler::Line;
use storage::Storage;

pub mod cell;
pub mod cursor;
pub mod row;
pub mod storage;

pub struct Dimensions {
    pub lines: usize,
    pub columns: usize,
}

pub struct Grid {
    pub cursor: Cursor,

    pub saved_cursor: Option<Cursor>,

    pub dimensions: Dimensions,

    raw: Storage,
}

impl Grid {
    pub fn new(dimensions: Dimensions) -> Self {
        Self {
            cursor: Cursor::default(),
            saved_cursor: None,
            dimensions,
            raw: Storage::new(),
        }
    }

    pub fn reset(&mut self) {
        self.cursor = Cursor::default();
        self.saved_cursor = None;

        // TODO: reset lines
    }

    pub fn cell_at_cursor_mut(&mut self) -> &mut Cell {
        &mut self.raw[self.cursor.position.line][self.cursor.position.column]
    }
}

impl Index<Line> for Grid {
    type Output = Row;

    fn index(&self, index: Line) -> &Self::Output {
        &self.raw[index]
    }
}

impl IndexMut<Line> for Grid {
    fn index_mut(&mut self, index: Line) -> &mut Self::Output {
        &mut self.raw[index]
    }
}
