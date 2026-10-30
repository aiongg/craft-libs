//! The data the host supplies to a [`PresetBrowser`](super::PresetBrowser): its flat display list
//! of groups and items, the shape of a cell, and what the thumbnail closure is told.

use std::fmt;
use std::sync::Arc;

use egui::{Painter, Rect, Vec2};

use crate::icons::Icon;

/// The shape of a grid cell's thumbnail.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum CellShape {
    /// As wide as high.
    #[default]
    Square,
    /// Width over height: 32:18 for gradients is `Ratio(32.0 / 18.0)`.
    Ratio(f32),
}

impl CellShape {
    /// The size of a thumbnail `thumb` points wide.
    pub fn size(self, thumb: f32) -> Vec2 {
        let ratio = match self {
            CellShape::Square => 1.0,
            CellShape::Ratio(r) if r.is_finite() && r > 0.0 => r,
            CellShape::Ratio(_) => 1.0,
        };
        Vec2::new(thumb, thumb / ratio)
    }
}

/// What the thumbnail closure is told about the cell it paints.
#[derive(Clone, Copy, Debug)]
pub struct ThumbInfo<'a, K> {
    /// The preset's key.
    pub key: &'a K,
    /// The size of the rect the thumbnail is painted in.
    pub size: Vec2,
    /// Whether the preset is the selected one.
    pub selected: bool,
    /// Whether the pointer is over the cell.
    pub hovered: bool,
}

/// Paints a [`Preview`] band.
pub type PreviewPaint = Arc<dyn Fn(&Painter, Rect) + Send + Sync>;

/// A second band of a list row, in list view only: the brush's stroke preview.
#[derive(Clone)]
pub struct Preview {
    /// Width over height of the band.
    pub aspect: f32,
    /// Paints the band into the rect, clipped to it.
    pub paint: PreviewPaint,
}

impl fmt::Debug for Preview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Preview").field("aspect", &self.aspect).finish_non_exhaustive()
    }
}

/// Whether a row is a group (or a folder inside one) or a preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetKind {
    /// A group or folder: a header row with children.
    Group,
    /// A preset: a thumbnail cell or list row.
    Item,
}

/// One entry of the host's flat display list: a group (or folder) or a preset. `K` is one type
/// for both, so a canvas that takes drops matches the variant it wants.
///
/// ```
/// use craft_ui::presets::PresetRow;
///
/// let rows = [
///     PresetRow::group("Default", "Default", 0).count(2),
///     PresetRow::item("Red", "Red", 1),
///     PresetRow::item("Blue", "Blue", 1).caption("12"),
/// ];
/// assert!(rows[0].is_group() && !rows[1].is_group());
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PresetRow<K> {
    /// The key events name the row by.
    pub key: K,
    /// The name, shown on a header and in a list row, and read by a screen reader.
    pub label: String,
    /// The nesting level: 0 at the top. A row is inside the nearest row above it of lower depth.
    pub depth: usize,
    /// Group or item.
    pub kind: PresetKind,
    /// The number shown at the trailing end of a header.
    pub count: Option<usize>,
    /// A short text under a grid thumbnail and as the detail line of a list row.
    pub caption: Option<String>,
    /// An icon shown instead of a thumbnail.
    pub icon: Option<Icon>,
    /// The stroke preview band of a list row.
    pub preview: Option<Preview>,
    /// Drawn faded.
    pub dimmed: bool,
    /// A tooltip other than the label.
    pub tooltip: Option<String>,
}

impl<K> PresetRow<K> {
    fn new(key: K, label: impl Into<String>, depth: usize, kind: PresetKind) -> Self {
        Self { key, label: label.into(), depth, kind, count: None, caption: None, icon: None, preview: None, dimmed: false, tooltip: None }
    }

    /// A group or folder at `depth`.
    pub fn group(key: K, label: impl Into<String>, depth: usize) -> Self {
        Self::new(key, label, depth, PresetKind::Group)
    }

    /// A preset at `depth`.
    pub fn item(key: K, label: impl Into<String>, depth: usize) -> Self {
        Self::new(key, label, depth, PresetKind::Item)
    }

    /// Shows `count` at the trailing end of the header.
    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Shows `caption` under the thumbnail.
    #[must_use]
    pub fn caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// Shows `icon` instead of a thumbnail.
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Adds a band of `aspect` (width over height) in list view, painted by `paint`.
    #[must_use]
    pub fn preview(mut self, aspect: f32, paint: impl Fn(&Painter, Rect) + Send + Sync + 'static) -> Self {
        self.preview = Some(Preview { aspect, paint: Arc::new(paint) });
        self
    }

    /// Draws the row faded.
    #[must_use]
    pub fn dimmed(mut self, dimmed: bool) -> Self {
        self.dimmed = dimmed;
        self
    }

    /// Sets the tooltip, instead of the label.
    #[must_use]
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Whether the row is a group or folder.
    pub fn is_group(&self) -> bool {
        self.kind == PresetKind::Group
    }
}

/// Whether any row has a caption, so that every grid cell makes room for one line.
pub fn has_captions<K>(rows: &[PresetRow<K>]) -> bool {
    rows.iter().any(|r| !r.is_group() && r.caption.is_some())
}

/// Whether a row's children are in the list after it: the rows below it that are deeper.
pub fn children_end<K>(rows: &[PresetRow<K>], at: usize) -> usize {
    let depth = rows.get(at).map_or(0, |r| r.depth);
    rows.iter().skip(at + 1).position(|r| r.depth <= depth).map_or(rows.len(), |n| at + 1 + n)
}

/// How many levels of groups the subtree at `at` has below its own row (0 for a group with no
/// folder inside).
fn group_levels_below<K>(rows: &[PresetRow<K>], at: usize) -> usize {
    let base = rows.get(at).map_or(0, |r| r.depth);
    rows[at + 1..children_end(rows, at)].iter().filter(|r| r.is_group()).map(|r| r.depth - base).max().unwrap_or(0)
}

/// Whether the group at `dragged` may nest into the group at `target` when no group may sit
/// deeper than `max_nest` (`PresetReorder::nest`; 0 lets no group into another), counting the
/// folders the dragged group carries. Nothing nests
/// into itself or its own folders.
pub fn nest_allowed<K>(rows: &[PresetRow<K>], dragged: usize, target: usize, max_nest: usize) -> bool {
    let (Some(from), Some(into)) = (rows.get(dragged), rows.get(target)) else {
        return false;
    };
    if !from.is_group() || !into.is_group() || (dragged..children_end(rows, dragged)).contains(&target) {
        return false;
    }
    into.depth + 1 + group_levels_below(rows, dragged) <= max_nest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<PresetRow<&'static str>> {
        vec![
            PresetRow::group("a", "A", 0),
            PresetRow::item("a1", "A1", 1),
            PresetRow::group("a-sub", "Sub", 1),
            PresetRow::item("a2", "A2", 2),
            PresetRow::group("b", "B", 0),
            PresetRow::group("b-sub", "Sub", 1),
        ]
    }

    #[test]
    fn presets_children_end_stops_at_the_next_sibling() {
        let rows = rows();
        assert_eq!(children_end(&rows, 0), 4);
        assert_eq!(children_end(&rows, 2), 4);
        assert_eq!(children_end(&rows, 4), 6);
        assert_eq!(children_end(&rows, 5), 6);
        assert_eq!(children_end(&rows, 1), 2, "an item has none");
    }

    #[test]
    fn presets_nesting_depth_is_cut() {
        let rows = rows();
        assert!(!nest_allowed(&rows, 4, 0, 0), "no nesting at 0");
        assert!(!nest_allowed(&rows, 5, 0, 0));
        assert!(nest_allowed(&rows, 5, 0, 1), "a folder moves to another group");
        assert!(!nest_allowed(&rows, 4, 5, 9), "not into its own folder");
        assert!(!nest_allowed(&rows, 0, 4, 1), "A carries a folder, so it would reach depth 2");
        assert!(nest_allowed(&rows, 0, 4, 2));
        assert!(!nest_allowed(&rows, 0, 2, 9), "not into its own folder");
        assert!(!nest_allowed(&rows, 1, 4, 9), "an item is not nested");
    }

    #[test]
    fn presets_cell_shapes_and_keys() {
        assert_eq!(CellShape::Square.size(40.0), Vec2::new(40.0, 40.0));
        assert_eq!(CellShape::Ratio(2.0).size(64.0), Vec2::new(64.0, 32.0));
        assert_eq!(CellShape::Ratio(0.0).size(10.0), Vec2::new(10.0, 10.0));
        let rows = [PresetRow::group("x", "x", 0), PresetRow::item("x", "x", 1)];
        assert_eq!(rows[0].key, rows[1].key, "the host's key type decides what is distinct");
        assert!(!has_captions(&rows));
    }
}
