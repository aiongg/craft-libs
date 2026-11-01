//! The pen draft's Auto Add/Delete and close cursor (task PE-C2): what a press on the open draft
//! does instead of adding an anchor, and whether it would close the path.

use egui::{Modifiers, Pos2};

use super::bezier::BezierStyle;
use super::{BezierHit, BezierOverlay, EditPath, EditSubpath, HandlesShown, PathSelection, PenDraft, PenMode, PenStyle};
use craft_ui::canvas::{CanvasView, hit};

/// What Auto Add/Delete does to the draft for a press.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum AutoEdit {
    /// Insert an anchor into the segment leaving knot `segment` at `t`
    /// ([`EditSubpath::insert_at`]).
    Insert { segment: usize, t: f64 },
    /// Remove knot `knot` ([`EditSubpath::remove_knot`]).
    Delete { knot: usize },
}

impl PenDraft<'_> {
    /// Auto Add/Delete for a press at the screen point `at` with `mods` on `draft`: `None` unless
    /// [`auto_add_delete`](PenDraft::auto_add_delete) is on, the mode is the Pen, no ⇧, ⌥ or ⌘ is
    /// held and no handle is being pulled (`pulling`). Then, by
    /// [`BezierOverlay::hit_test`](super::BezierOverlay::hit_test) on the draft within
    /// `bezier.hit` (no handles shown): a segment gives [`AutoEdit::Insert`] at the hit's `t`; an
    /// anchor that is not the first or the last gives [`AutoEdit::Delete`]; the first and the last
    /// are left to the Pen (the first closes, the last retracts). The pass applies the edit as one
    /// Update of the draft gesture and takes no other action for the press. The hover draws the
    /// same answer's [`HitMark`](super::HitMark) (with `bezier`'s look) and no rubber band.
    pub(super) fn auto_edit(
        &self,
        draft: &EditSubpath,
        at: Pos2,
        mods: Modifiers,
        pulling: bool,
        view: &CanvasView,
        bezier: &BezierStyle,
    ) -> Option<AutoEdit> {
        // pc: crates/ui-egui/src/tools/anchor_tools.rs:50-84 `pen_auto` (the draft only while
        // drawing :63-69; ends :74-78), :116-128 `edit_pen`; crates/ui-egui/src/tools/vector_ui/
        // overlay.rs:201-207 (no rubber band where Auto acts).
        if !self.auto_add_delete || self.mode != PenMode::Pen || mods.shift || mods.alt || mods.command || pulling || draft.knots.is_empty() {
            return None;
        }
        let path = EditPath { key: 0, subpaths: vec![draft.clone()], color: None };
        let found = BezierOverlay::hit_test(&[path], &PathSelection::default(), HandlesShown::Selected, view, at, bezier.hit)?;
        match found {
            BezierHit::Segment { segment, t, .. } => Some(AutoEdit::Insert { segment: segment.knot, t }),
            BezierHit::Anchor { knot, .. } => {
                let end = !draft.closed && (knot.knot == 0 || knot.knot + 1 == draft.knots.len());
                (!end).then_some(AutoEdit::Delete { knot: knot.knot })
            }
            BezierHit::Handle { .. } => None,
        }
    }

    /// Whether a press at the screen point `pointer` would close `draft`: no press down, the
    /// draft open, within `style.hit` of the first anchor, with 2 or more anchors in Pen mode and
    /// 3 or more in Curvature mode. [`PenResponse::closes`](super::PenResponse::closes).
    pub(super) fn closes_at(&self, draft: &EditSubpath, pointer: Pos2, view: &CanvasView, style: &PenStyle) -> bool {
        // pc: crates/ui-egui/src/tools/vector_ui/pen.rs:20-34 `on_closing_anchor`, `closes_path_at`.
        let needed = match self.mode {
            PenMode::Pen => 2,
            PenMode::Curvature => 3,
        };
        draft.knots.len() >= needed && !draft.closed && hit::point(pointer, view.to_screen(draft.knots[0].anchor)) <= style.hit
    }
}
