use std::ops::{Deref, DerefMut, Index, IndexMut};

use cell::Cell;
use saiga_vte::ansi::handler::{Charset, CharsetIndex};

pub mod cell;

pub type Line = usize;
pub type Column = usize;

#[derive(Clone)]
pub struct Row(Vec<Cell>);

impl Deref for Row {
    type Target = Vec<Cell>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Row {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Row {
    pub fn new(columns: usize) -> Self {
        let mut inner = Vec::with_capacity(columns);

        inner.resize(columns, Cell::default());

        Self(inner)
    }
}

#[derive(Clone, Copy)]
pub struct Dimensions {
    pub rows: usize,
    pub columns: usize,
}

impl Default for Dimensions {
    fn default() -> Self {
        Self {
            rows: 80,
            columns: 40,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Position {
    pub line: Line,
    pub column: Column,
}

#[derive(Debug)]
pub struct PositionedCell {
    pub position: Position,
    pub value: Cell,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Charsets([Charset; 4]);

impl Index<CharsetIndex> for Charsets {
    type Output = Charset;

    fn index(&self, index: CharsetIndex) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<CharsetIndex> for Charsets {
    fn index_mut(&mut self, index: CharsetIndex) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

#[derive(Default, Clone)]
pub struct Cursor {
    pub position: Position,
    pub template: Cell,
    pub charsets: Charsets,
}

#[derive(Default)]
pub struct Grid {
    rows: Vec<Row>,

    pub cursor: Cursor,
    pub saved_cursor: Option<Cursor>,

    dimensions: Dimensions,
}

impl Grid {
    pub fn with_dimensions(dimensions: Dimensions) -> Self {
        let mut rows = Vec::with_capacity(dimensions.rows);

        rows.resize(dimensions.rows, Row::new(dimensions.columns));

        Self {
            rows,
            cursor: Cursor::default(),
            saved_cursor: None,
            dimensions,
        }
    }

    pub fn resize(&mut self, _dimensions: Dimensions) {
        todo!("resize")
    }

    pub fn width(&self) -> usize {
        self.dimensions.columns
    }

    pub fn height(&self) -> usize {
        self.dimensions.rows
    }

    pub fn iter(&self) -> GridIterator<'_> {
        let end = Position {
            line: self.height() - 1,
            column: self.width() - 1,
        };

        GridIterator {
            grid: self,
            current: None,
            end,
        }
    }

    pub fn cell_at_cursor(&self) -> &Cell {
        &self[self.cursor.position]
    }

    pub fn cell_at_cursor_mut(&mut self) -> &mut Cell {
        let position = self.cursor.position;
        &mut self[position]
    }
}

impl Index<Position> for Grid {
    type Output = Cell;

    fn index(&self, index: Position) -> &Self::Output {
        &self.rows[index.line].0[index.column]
    }
}

impl IndexMut<Position> for Grid {
    fn index_mut(&mut self, index: Position) -> &mut Self::Output {
        &mut self.rows[index.line].0[index.column]
    }
}

impl Index<Line> for Grid {
    type Output = Row;

    fn index(&self, index: Line) -> &Self::Output {
        &self.rows[index]
    }
}

impl IndexMut<Line> for Grid {
    fn index_mut(&mut self, index: Line) -> &mut Self::Output {
        &mut self.rows[index]
    }
}

pub struct GridIterator<'a> {
    grid: &'a Grid,
    current: Option<Position>,
    end: Position,
}

impl<'a> Iterator for GridIterator<'a> {
    type Item = PositionedCell;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_some_and(|p| p == self.end) {
            return None;
        }

        let position = self
            .current
            .map(|p| match p {
                Position { column, .. } if column == self.grid.width() - 1 => Position {
                    line: p.line + 1,
                    column: 0,
                },
                _ => Position {
                    line: p.line,
                    column: p.column + 1,
                },
            })
            .unwrap_or_default();

        let cell = PositionedCell {
            value: self.grid[position],
            position,
        };

        self.current = Some(position);

        Some(cell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize() {
        let dimensions = Dimensions {
            rows: 42,
            columns: 37,
        };

        let grid = Grid::with_dimensions(dimensions);

        assert_eq!(grid.rows.len(), dimensions.rows);

        for row in grid.rows {
            assert_eq!(row.len(), dimensions.columns);

            for cell in row.iter() {
                assert_eq!(cell, &Cell::default());
            }
        }
    }
}
