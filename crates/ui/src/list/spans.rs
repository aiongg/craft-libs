//! Where a list's rows lie along its height, for laying out only the rows in view.

use std::ops::Range;

use crate::behavior::RowSpans;

/// The rows' extents from the top of the list's content: one height for every row, or the
/// bottom of each row (prefix sums of mixed heights), searched by bisection.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Spans {
    Uniform { height: f32, rows: usize },
    Mixed { bottoms: Vec<f32> },
}

impl Spans {
    pub(super) fn uniform(height: f32, rows: usize) -> Self {
        Spans::Uniform { height: height.max(1.0), rows }
    }

    /// Rows whose heights `height` gives, each at least one point.
    pub(super) fn mixed(rows: usize, height: impl Fn(usize) -> f32) -> Self {
        let mut bottom = 0.0;
        Spans::Mixed {
            bottoms: (0..rows)
                .map(|at| {
                    bottom += height(at).max(1.0);
                    bottom
                })
                .collect(),
        }
    }

    pub(super) fn len(&self) -> usize {
        match self {
            Spans::Uniform { rows, .. } => *rows,
            Spans::Mixed { bottoms } => bottoms.len(),
        }
    }

    pub(super) fn top(&self, at: usize) -> f32 {
        match self {
            Spans::Uniform { height, .. } => at as f32 * height,
            Spans::Mixed { bottoms } => at.checked_sub(1).and_then(|p| bottoms.get(p)).copied().unwrap_or(0.0),
        }
    }

    pub(super) fn bottom(&self, at: usize) -> f32 {
        match self {
            Spans::Uniform { height, .. } => (at + 1) as f32 * height,
            Spans::Mixed { bottoms } => bottoms.get(at).copied().unwrap_or(0.0),
        }
    }

    /// The height of all rows.
    pub(super) fn total(&self) -> f32 {
        match self.len() {
            0 => 0.0,
            len => self.bottom(len - 1),
        }
    }

    /// The row whose span holds `y`, clamped to the first and last rows; `None` with no rows.
    pub(super) fn row_at(&self, y: f32) -> Option<usize> {
        let last = self.len().checked_sub(1)?;
        Some(match self {
            Spans::Uniform { height, .. } => ((y / height).floor().max(0.0) as usize).min(last),
            Spans::Mixed { bottoms } => bottoms.partition_point(|&bottom| bottom <= y).min(last),
        })
    }

    /// The rows to lay out for the viewport `view`: those it shows, and one above and one below.
    pub(super) fn in_view(&self, view: Range<f32>) -> Range<usize> {
        let (Some(first), Some(last)) = (self.row_at(view.start), self.row_at((view.end - 0.5).max(view.start))) else {
            return 0..0;
        };
        first.saturating_sub(1)..(last + 2).min(self.len())
    }

    /// How many rows a page key moves: the rows the viewport `view` shows whole, at least one.
    pub(super) fn page(&self, view: Range<f32>) -> usize {
        self.in_view(view.clone()).filter(|&at| self.top(at) >= view.start - 0.5 && self.bottom(at) <= view.end + 0.5).count().max(1)
    }

    /// The scroll offset that shows row `at` in a viewport `height` high now at `offset`, when it
    /// isn't shown whole: the row at the top when it lies above, at the bottom when below (at the
    /// top when it is taller than the viewport). `None` when it is shown whole.
    pub(super) fn reveal(&self, at: usize, offset: f32, height: f32) -> Option<f32> {
        if at >= self.len() {
            return None;
        }
        let (top, bottom) = (self.top(at), self.bottom(at));
        if top < offset || bottom - top > height {
            Some(top)
        } else if bottom > offset + height {
            Some(bottom - height)
        } else {
            None
        }
    }
}

impl RowSpans for Spans {
    fn len(&self) -> usize {
        Spans::len(self)
    }

    fn span(&self, at: usize) -> Range<f32> {
        self.top(at)..self.bottom(at)
    }
}

#[cfg(test)]
mod tests {
    use super::Spans;

    #[test]
    fn list_spans_find_rows_in_view_for_uniform_and_mixed_heights() {
        let uniform = Spans::uniform(24.0, 10_000);
        assert_eq!(uniform.total(), 240_000.0);
        assert_eq!(uniform.in_view(0.0..100.0), 0..6, "rows 0..=4 shown, one below");
        assert_eq!(uniform.in_view(2400.0..2500.0), 99..106, "one above, one below");
        assert_eq!(uniform.in_view(239_900.0..240_000.0), 9994..10_000);
        assert_eq!(uniform.page(0.0..100.0), 4);

        let mixed = Spans::mixed(6, |at| if at % 2 == 0 { 10.0 } else { 30.0 });
        assert_eq!((mixed.top(3), mixed.bottom(3), mixed.total()), (50.0, 80.0, 120.0));
        assert_eq!(mixed.row_at(49.9), Some(2));
        assert_eq!(mixed.row_at(50.0), Some(3));
        assert_eq!(mixed.row_at(1e9), Some(5));
        assert_eq!(mixed.in_view(45.0..85.0), 1..6);
        assert_eq!(Spans::mixed(0, |_| 10.0).in_view(0.0..100.0), 0..0);
    }

    #[test]
    fn list_spans_reveal_only_a_hidden_row() {
        let spans = Spans::uniform(20.0, 100);
        assert_eq!(spans.reveal(5, 0.0, 200.0), None);
        assert_eq!(spans.reveal(10, 0.0, 200.0), Some(20.0), "below: to the bottom edge");
        assert_eq!(spans.reveal(3, 100.0, 200.0), Some(60.0), "above: to the top edge");
        assert_eq!(spans.reveal(3, 0.0, 10.0), Some(60.0), "taller than the viewport: its top");
    }
}
