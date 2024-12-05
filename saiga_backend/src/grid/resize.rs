use std::{cmp::Ordering, mem};

use super::{cell::Cell, Dimensions, Grid, Row};

impl Grid {
    pub fn resize(&mut self, dimensions: Dimensions) {
        let template = mem::take(&mut self.cursor.template);

        match self.dimensions.lines.cmp(&dimensions.lines) {
            Ordering::Less => self.grow_lines_to(dimensions.lines),
            Ordering::Greater => self.shrink_lines_to(dimensions.lines),
            Ordering::Equal => (),
        }

        match self.dimensions.columns.cmp(&dimensions.columns) {
            Ordering::Less => self.grow_columns_to(dimensions.columns),
            Ordering::Greater => self.shrink_columns_to(dimensions.columns),
            Ordering::Equal => (),
        }

        self.cursor.template = template;
    }

    fn grow_lines_to(&mut self, target: usize) {
        let lines_added = target - self.dimensions.lines;

        let size = self.dimensions.lines + lines_added;

        self.rows
            .resize_with(size, || Row::new(self.dimensions.columns));

        self.dimensions.lines = target;
    }

    fn shrink_lines_to(&mut self, target: usize) {
        let lines_removed = self.dimensions.lines - target;

        let size = self.dimensions.lines - lines_removed;

        self.rows.truncate(size);

        self.dimensions.lines = target;
    }

    fn grow_columns_to(&mut self, target: usize) {
        // TODO: wrap

        let columns_added = target - self.dimensions.columns;

        let size = self.dimensions.columns + columns_added;

        let mut cell = Cell::default();
        cell.apply_template(&self.cursor.template);

        for row in self.rows.iter_mut() {
            row.resize_with(size, || cell);
        }

        self.dimensions.columns = target;
    }

    fn shrink_columns_to(&mut self, target: usize) {
        // TODO: wrap

        let columns_removed = self.dimensions.columns - target;

        let size = self.dimensions.columns - columns_removed;

        for row in self.rows.iter_mut() {
            row.truncate(size)
        }

        self.dimensions.columns = target;
    }
}
