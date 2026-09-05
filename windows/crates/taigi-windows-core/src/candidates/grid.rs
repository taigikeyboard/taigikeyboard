//! The expanded grid's geometry: column-spanned rows, pure arithmetic. Port
//! of `ExpandedGridLayout.swift` (MacishType-derived).

/// One candidate's place in the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridCell {
    pub candidate_index: usize,
    pub column_start: usize,
    pub column_span: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpandedGridLayout {
    pub rows: Vec<Vec<GridCell>>,
    pub column_count: usize,
}

impl ExpandedGridLayout {
    /// Lays `widths` into rows of `column_count` columns. Each item spans the
    /// columns it needs (at least one, at most a full row), and moves to the
    /// next row when it does not fit — unless the row is empty.
    ///
    /// Degenerate inputs are held to a valid grid rather than trusted: a
    /// zero column count becomes one, and a width that is not a positive
    /// finite number (or a non-positive column width) spans one column.
    pub fn compute(widths: &[f32], column_width: f32, column_count: usize) -> Self {
        let column_count = column_count.max(1);
        let mut rows: Vec<Vec<GridCell>> = Vec::new();
        let mut row: Vec<GridCell> = Vec::new();
        let mut column = 0;
        for (index, width) in widths.iter().enumerate() {
            let ratio = width / column_width;
            let span = if ratio.is_finite() && ratio > 0.0 {
                (ratio.ceil() as usize).clamp(1, column_count)
            } else {
                1
            };
            if column + span > column_count && !row.is_empty() {
                rows.push(std::mem::take(&mut row));
                column = 0;
            }
            row.push(GridCell {
                candidate_index: index,
                column_start: column,
                column_span: span,
            });
            column += span;
        }
        if !row.is_empty() {
            rows.push(row);
        }
        Self { rows, column_count }
    }

    pub fn position_of(&self, candidate_index: usize) -> Option<(usize, GridCell)> {
        self.rows.iter().enumerate().find_map(|(row_index, row)| {
            row.iter()
                .find(|cell| cell.candidate_index == candidate_index)
                .map(|cell| (row_index, *cell))
        })
    }

    /// Where a vertical step of `row_step` rows lands: the cell of the target
    /// row whose columns overlap the current cell's. `None` when the clamped
    /// target row is the one the selection is already on — at the top row
    /// that is where a collapse begins.
    pub fn vertical_target(&self, candidate_index: usize, row_step: i32) -> Option<usize> {
        let (row_index, cell) = self.position_of(candidate_index)?;
        let last_row = self.rows.len().checked_sub(1)?;
        let target_row = (row_index as i32 + row_step).clamp(0, last_row as i32) as usize;
        if target_row == row_index {
            return None;
        }
        self.overlapping_candidate(
            target_row,
            cell.column_start,
            cell.column_start + cell.column_span,
            row_step > 0,
        )
        .or_else(|| {
            self.rows[target_row]
                .last()
                .map(|cell| cell.candidate_index)
        })
    }

    /// The first cell of `row_index` whose columns overlap
    /// `column_start..column_end`, with the horizontal pages' backward
    /// tiebreak: stepping up prefers the neighbour the highlight sits over.
    pub fn overlapping_candidate(
        &self,
        row_index: usize,
        column_start: usize,
        column_end: usize,
        forward: bool,
    ) -> Option<usize> {
        let row = self.rows.get(row_index)?;
        for (position, cell) in row.iter().enumerate() {
            let cell_end = cell.column_start + cell.column_span;
            if cell.column_start >= column_end || cell_end <= column_start {
                continue;
            }
            if !forward
                && cell.column_start < column_start
                && position + 1 < row.len()
                && row[position + 1].column_start < column_end
            {
                return Some(row[position + 1].candidate_index);
            }
            return Some(cell.candidate_index);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(widths: &[f32]) -> ExpandedGridLayout {
        ExpandedGridLayout::compute(widths, 10.0, 6)
    }

    fn shape(layout: &ExpandedGridLayout) -> Vec<Vec<[usize; 3]>> {
        layout
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|c| [c.candidate_index, c.column_start, c.column_span])
                    .collect()
            })
            .collect()
    }

    #[test]
    fn compute_quantizes_widths_into_column_spans() {
        // trace: ExpandedGridLayoutTests.swift:14-34.
        assert_eq!(
            shape(&grid(&[10.0, 15.0, 25.0, 10.0])),
            [vec![[0, 0, 1], [1, 1, 2], [2, 3, 3]], vec![[3, 0, 1]]]
        );
        let oversized = grid(&[10.0, 200.0, 10.0]);
        assert_eq!(
            oversized
                .rows
                .iter()
                .map(|r| r.iter().map(|c| c.candidate_index).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            [[0], [1], [2]]
        );
        assert_eq!(oversized.rows[1][0].column_span, 6);
        // Degenerate inputs still yield a valid grid.
        let degenerate = ExpandedGridLayout::compute(&[f32::NAN, 10.0, f32::INFINITY], 0.0, 0);
        assert_eq!(degenerate.column_count, 1);
        assert!(degenerate
            .rows
            .iter()
            .all(|r| r.len() == 1 && r[0].column_span == 1));
    }

    #[test]
    fn vertical_target_lands_on_the_overlapping_cell_and_is_none_at_the_edge() {
        // trace: ExpandedGridLayoutTests.swift:36-63.
        let layout = grid(&[10.0, 15.0, 25.0, 25.0, 25.0]);
        assert_eq!(layout.vertical_target(1, 1), Some(3));
        assert_eq!(layout.vertical_target(4, -1), Some(2));
        assert_eq!(layout.vertical_target(0, -1), None);
        assert_eq!(layout.vertical_target(4, 1), None);
        let tiebreak = grid(&[40.0, 20.0, 10.0, 10.0, 10.0, 30.0]);
        assert_eq!(tiebreak.vertical_target(5, -1), Some(1));
    }
}
