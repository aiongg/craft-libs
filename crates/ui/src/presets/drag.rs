//! The browser's drag to reorder: the rules a host sets ([`PresetReorder`]), the display order
//! the rules read, and the drop target under the pointer in a column of headers, list rows and
//! wrapped grid cells.

use std::hash::Hash;

use egui::{Modifiers, Pos2, Rect};

use super::layout::{Metrics, VisualRow, VisualRows};
use super::model::{PresetRow, nest_allowed};
use crate::behavior::{DisplayOrder, DropPosition, Move, Reorder, Resolved, UniformSpans, grid_drop};
use crate::list::spans::Spans;

type Accepts<'a, K> = Box<dyn Fn(&Move<K>, Modifiers) -> bool + 'a>;

/// What may be dragged, how deep groups may nest and what the host refuses. The browser always
/// keeps these rules: a preset drops next to another preset or into a group; a group drops next
/// to a group, or into one where `nest` allows.
///
/// ```
/// use craft_ui::presets::PresetReorder;
///
/// let reorder = PresetReorder::<u32>::items_and_groups().nest(1).accepts(|mv, _| mv.items != [7]);
/// ```
pub struct PresetReorder<'a, K> {
    pub(super) groups: bool,
    pub(super) nest: usize,
    accepts: Option<Accepts<'a, K>>,
}

impl<K> Default for PresetReorder<'_, K> {
    fn default() -> Self {
        Self::items()
    }
}

impl<'a, K> PresetReorder<'a, K> {
    /// Presets move, within and between groups; groups stay where they are.
    pub fn items() -> Self {
        Self { groups: false, nest: 0, accepts: None }
    }

    /// Presets and groups move.
    pub fn items_and_groups() -> Self {
        Self { groups: true, nest: 0, accepts: None }
    }

    /// The deepest level a group may sit at: 0 (the default) lets no group into another, 1 lets
    /// groups hold folders, and so on.
    #[must_use]
    pub fn nest(mut self, depth: usize) -> Self {
        self.nest = depth;
        self
    }

    /// Refuses the moves for which `accepts` is false: a refused target shows no marker. It gets
    /// the modifiers held.
    #[must_use]
    pub fn accepts(mut self, accepts: impl Fn(&Move<K>, Modifiers) -> bool + 'a) -> Self {
        self.accepts = Some(Box::new(accepts));
        self
    }
}

impl<K: Clone + PartialEq> PresetReorder<'_, K> {
    /// Whether the browser's rules and the host's allow `mv` over `rows`.
    pub(super) fn allows(&self, rows: &[PresetRow<K>], mv: &Move<K>, modifiers: Modifiers) -> bool {
        let at = |key: &K| rows.iter().position(|r| r.key == *key);
        let Some(target) = at(&mv.target) else { return false };
        let into_group = rows[target].is_group();
        for key in &mv.items {
            let Some(from) = at(key) else { return false };
            let ok = if rows[from].is_group() {
                self.groups
                    && into_group
                    && match mv.position {
                        DropPosition::Into => nest_allowed(rows, from, target, self.nest),
                        _ => rows[target].depth == 0 || nest_allowed(rows, from, target, self.nest + 1),
                    }
            } else {
                !into_group || mv.position == DropPosition::Into
            };
            if !ok {
                return false;
            }
        }
        self.accepts.as_ref().is_none_or(|accepts| accepts(mv, modifiers))
    }
}

/// The shown rows as a display order for [`Reorder`] and the tree keys: positions are indices
/// into `shown`.
pub(super) struct Order<'a, K> {
    pub rows: &'a [PresetRow<K>],
    pub shown: &'a [usize],
    /// Whether the group at each position is open.
    pub open: &'a [bool],
}

impl<K: PartialEq> DisplayOrder<K> for Order<'_, K> {
    fn len(&self) -> usize {
        self.shown.len()
    }

    fn key(&self, at: usize) -> &K {
        &self.rows[self.shown[at]].key
    }

    fn depth(&self, at: usize) -> usize {
        self.rows[self.shown[at]].depth
    }

    fn has_children(&self, at: usize) -> bool {
        self.rows[self.shown[at]].is_group()
    }

    fn open(&self, at: usize) -> bool {
        self.open[at]
    }
}

/// Where the rows and cells are on the screen, for reading a pointer.
pub(super) struct Surface<'a> {
    pub vrows: &'a VisualRows,
    pub spans: &'a Spans,
    pub metrics: &'a Metrics,
    /// Where the spans start (top) and the width of the column.
    pub area: Rect,
    pub columns: usize,
    pub rtl: bool,
}

impl Surface<'_> {
    /// The pointer as a coordinate along the shown positions, two units to each: the fraction of
    /// a header or list row the pointer is down, the half of a grid cell it is in. A pointer past
    /// the last row is past every position. `None` where nothing takes a drop.
    fn coordinate(&self, pointer: Pos2) -> Option<f32> {
        let y = pointer.y - self.area.top();
        if y < 0.0 {
            return Some(0.0);
        }
        if y >= self.spans.total() {
            return Some(2.0 * self.vrows.shown.len() as f32);
        }
        let line = self.spans.row_at(y)?;
        let (top, bottom) = (self.spans.top(line), self.spans.bottom(line));
        match &self.vrows.rows[line] {
            VisualRow::Header(pos) | VisualRow::Item(pos) => {
                let share = ((y - top) / (bottom - top).max(f32::EPSILON)).clamp(0.0, 0.999);
                Some(2.0 * *pos as f32 + 2.0 * share)
            }
            VisualRow::Cells { run, .. } => {
                let first = self.vrows.visual_row_of(run.start);
                let geometry = self.metrics.run_geometry(self.area, self.spans.top(first), run.len(), self.columns, self.rtl);
                let drop = grid_drop(&geometry, pointer)?;
                let half = if drop.position == DropPosition::Before { 0.5 } else { 1.5 };
                Some(2.0 * (run.start + drop.index) as f32 + half)
            }
        }
    }

    /// The target of a drag of `dragged` with the pointer at `pointer`.
    pub fn resolve<K: Clone + PartialEq>(
        &self,
        reorder: &Reorder<'_, K>,
        order: &(impl DisplayOrder<K> + ?Sized),
        pointer: Pos2,
        dragged: &[K],
        modifiers: Modifiers,
    ) -> Option<Resolved<K>> {
        let coordinate = self.coordinate(pointer)?;
        let spans = UniformSpans { origin: 0.0, extent: 2.0, gap: 0.0, rows: order.len() };
        reorder.resolve(order, &spans, coordinate, dragged, modifiers)
    }
}

/// A drag of a preset or group under way.
#[derive(Clone)]
pub(super) struct Dragging<K> {
    pub items: Vec<K>,
    pub label: String,
    pub group: bool,
}

/// Whether every key of `items` is still a row of `order`.
pub(super) fn all_shown<K: Eq + Hash>(order: &(impl DisplayOrder<K> + ?Sized), items: &[K]) -> bool {
    items.iter().all(|key| (0..order.len()).any(|at| order.key(at) == key))
}
