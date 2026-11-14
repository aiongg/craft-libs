//! [`PathsPanel`]: the Paths panel on a `List` with a footer.

use super::{PathRow, PathsOutput};
use craft_ui::runtime::env::Label;
use egui::Ui;
use std::fmt::Debug;
use std::hash::Hash;

/// The Paths panel: one row per path with a thumbnail of its outline, rename in place, and the footer's buttons.
///
/// A Cmd/Ctrl-click on the thumbnail loads the path as a selection. Fill Path and Stroke Path are greyed while
/// [`can_fill`](Self::can_fill) is false.
#[must_use = "show the panel with `show`"]
#[derive(craft_ui::contract::Pattern)]
#[pattern(probe(PathsPanel<'static, u32>))]
pub struct PathsPanel<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> {
    rows: &'a [PathRow<K>],
    document: kurbo::Size,
    can_fill: bool,
    work_name: Option<Label>,
    rename: Option<K>,
}

impl<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> PathsPanel<'a, K> {
    /// The panel showing `rows` in their order, thumbnails scaled to the `document`'s size.
    pub fn new(rows: &'a [PathRow<K>], document: kurbo::Size) -> Self {
        PathsPanel { rows, document, can_fill: true, work_name: None, rename: None }
    }

    /// Whether the document can be filled and stroked; true by default.
    pub fn can_fill(mut self, can_fill: bool) -> Self {
        self.can_fill = can_fill;
        self
    }

    /// The name the work path shows.
    pub fn work_name(mut self, name: impl Into<Label>) -> Self {
        self.work_name = Some(name.into());
        self
    }

    /// Opens the rename field on the row `key`.
    pub fn start_rename(mut self, key: K) -> Self {
        self.rename = Some(key);
        self
    }

    /// Shows the panel.
    pub fn show(self, ui: &mut Ui) -> PathsOutput<K> {
        let PathsPanel { rows, document, can_fill, work_name, rename } = self;
        let _ = (ui, rows, document, can_fill, work_name, rename);
        todo!("tl.T34")
    }
}
