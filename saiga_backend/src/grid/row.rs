use std::ops::{Index, IndexMut};

use saiga_vte::ansi::handler::Column;

use super::cell::Cell;

pub struct Row {
    inner: Vec<Cell>,
}

impl Index<Column> for Row {
    type Output = Cell;

    fn index(&self, index: Column) -> &Self::Output {
        &self.inner[index as usize]
    }
}

impl IndexMut<Column> for Row {
    fn index_mut(&mut self, index: Column) -> &mut Self::Output {
        &mut self.inner[index as usize]
    }
}
