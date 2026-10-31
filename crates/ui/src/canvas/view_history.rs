//! [`ViewHistory`]: Go Back and Go Forward through the places a window has jumped between, in the
//! order it showed them, like a browser's Back and Forward (InDesign's Layout › Go Back / Go
//! Forward). Session state, not document history.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use kurbo::Point;

/// Where a view stood: what it showed (`key`: a page, an artboard, a document) and the view's
/// centre and zoom there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewPlace<K> {
    pub key: K,
    /// The document point at the canvas's centre.
    pub center: Point,
    /// Screen points per document unit ([`CanvasView::zoom`](super::CanvasView::zoom)).
    pub zoom: f64,
}

/// The places a window jumped between, oldest first, with the one it shows now. The app records
/// a jump (a page picked in the Pages panel, Go to Page, a link followed) with
/// [`leave`](Self::leave) before it and [`visit`](Self::visit) after; scrolling within a place is
/// not a jump. Bounded to [`LIMIT`](Self::LIMIT) entries.
///
/// ```
/// use craft_ui::canvas::{ViewHistory, ViewPlace, kurbo::Point};
/// let mut history = ViewHistory::default();
/// let at = |key| ViewPlace { key, center: Point::ZERO, zoom: 1.0 };
/// history.visit(at(1));
/// history.leave(at(1));
/// history.visit(at(2));
/// # let _ = history.can_go(-1, |_| true);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ViewHistory<K> {
    entries: Vec<ViewPlace<K>>,
    at: usize,
}

impl<K> Default for ViewHistory<K> {
    fn default() -> Self {
        Self { entries: Vec::new(), at: 0 }
    }
}

impl<K: Clone + PartialEq> ViewHistory<K> {
    /// Entries kept; the oldest go first.
    pub const LIMIT: usize = 256;

    /// The view is about to jump away from `place`: its entry keeps the centre and zoom the
    /// jump left it at, so Go Back returns there.
    // designcraft: crates/ui-egui/src/page_history.rs:22 `leave`.
    pub fn leave(&mut self, place: ViewPlace<K>) {
        todo!()
    }

    /// The view jumped to `place`. Visiting after Go Back drops the Go Forward entries; visiting
    /// the place already current only updates it.
    // designcraft: crates/ui-egui/src/page_history.rs:32 `visit`.
    pub fn visit(&mut self, place: ViewPlace<K>) {
        todo!()
    }

    /// Whether Go Back (`step` < 0) or Go Forward (`step` > 0) has a place to go to, skipping
    /// places whose key no longer `exists` (a deleted page): for the menu items' enabled state.
    // designcraft: crates/ui-egui/src/page_history.rs:56 `can_go`.
    pub fn can_go(&self, step: isize, exists: impl Fn(&K) -> bool) -> bool {
        todo!()
    }

    /// Go Back or Go Forward: the place to show, skipping places that no longer exist. Going
    /// does not record the place it leaves.
    // designcraft: crates/ui-egui/src/page_history.rs:62 `go`.
    pub fn go(&mut self, step: isize, exists: impl Fn(&K) -> bool) -> Option<ViewPlace<K>> {
        todo!()
    }
}
