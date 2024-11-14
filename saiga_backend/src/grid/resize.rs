use super::{Dimensions, Grid, Row};

impl Grid {
    pub fn resize(&mut self, dimensions: Dimensions) {
        // TODO: rewrite this. just a placeholder
        self.dimensions = dimensions;

        let mut rows = Vec::with_capacity(dimensions.rows);

        rows.resize(dimensions.rows, Row::new(dimensions.columns));

        self.rows = rows;
    }
}
