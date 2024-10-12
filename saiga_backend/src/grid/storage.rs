use std::ops::{Index, IndexMut};

use saiga_vte::ansi::handler::Line;

use super::row::Row;

pub struct Storage {
    inner: Vec<Row>,
}

impl Storage {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }
}

impl Index<Line> for Storage {
    type Output = Row;

    fn index(&self, index: Line) -> &Self::Output {
        &self.inner[index as usize]
    }
}

impl IndexMut<Line> for Storage {
    fn index_mut(&mut self, index: Line) -> &mut Self::Output {
        &mut self.inner[index as usize]
    }
}
