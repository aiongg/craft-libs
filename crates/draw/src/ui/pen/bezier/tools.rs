//! The Add Anchor Point, Delete Anchor Point, Auto Add/Delete (task PE-B1) and Curvature (task
//! PE-C1) tools of [`BezierOverlay`]. The overlay's pass calls these where a tool decides: which
//! presses it takes, what a click reports, the hover mark, and the Curvature gestures and keys.
//! Each task wires its calls into `bezier/mod.rs` (`takes`, `start`, `track`, `edit`, `pointer`,
//! `keys`, `paint`) and may change these private signatures.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land (tasks PE-B1, PE-C1)")]

use egui::{Modifiers, Pos2, Ui};

use super::{BezierHit, BezierOverlay, BezierResponse, BezierTool, Pass, PathEdit};
use craft_ui::canvas::claim::{Claim, PrimaryPress};
use craft_ui::edit::Gesture;

impl BezierOverlay<'_> {
    /// The tool a press acts as: [`BezierTool::AddAnchor`] and [`BezierTool::DeleteAnchor`] swap
    /// while ⌥ is held; every other tool is itself. (⌘ making the pointer Direct Selection is
    /// the host's, through [`PathMode`](crate::ui::pen::PathMode).)
    pub(super) fn acting_tool(&self, mods: Modifiers) -> BezierTool {
        // pc: crates/ui-egui/src/tools/anchor_tools.rs:27-35 `effective`.
        match (self.tool, mods.alt) {
            (BezierTool::AddAnchor, true) => BezierTool::DeleteAnchor,
            (BezierTool::DeleteAnchor, true) => BezierTool::AddAnchor,
            (tool, _) => tool,
        }
    }

    /// Whether the new tools take a press on `hit` (task PE-B1; PE-C1 adds Curvature):
    ///
    /// - Add Anchor: a segment, or an anchor (the press is claimed so it starts no path; it
    ///   reports nothing). Delete Anchor: an anchor, or a segment (claimed, nothing reported).
    ///   Handles are never taken.
    /// - Auto Add/Delete: a segment, or an anchor that is not the first or last of an open
    ///   subpath (those are the pen draft's: the first closes, the last retracts). Nothing else,
    ///   so the pen draft starts a path on empty canvas.
    /// - Curvature: an anchor or a segment, never a handle or empty canvas.
    pub(super) fn tool_takes(&self, hit: &BezierHit, mods: Modifiers) -> bool {
        // pc: crates/ui-egui/src/tools/anchor_tools.rs:37-48 `target`, :57-84 `pen_auto` (ends
        // excluded :74-78); crates/ui-egui/src/tools/direct_select.rs:368-376 `curvature_target`.
        match self.tool {
            // The swap with ⌥ changes what a press reports, not what it takes.
            BezierTool::AddAnchor | BezierTool::DeleteAnchor => !matches!(hit, BezierHit::Handle { .. }),
            BezierTool::AutoAddDelete => match *hit {
                BezierHit::Handle { .. } => false,
                BezierHit::Segment { .. } => true,
                BezierHit::Anchor { path, knot } => {
                    self.subpath(path, knot.subpath).is_some_and(|s| s.closed || (knot.knot != 0 && knot.knot + 1 != s.knots.len()))
                }
            },
            // Task PE-C1 takes anchors and segments.
            _ => false,
        }
    }

    /// A click of Add, Delete or Auto Add/Delete on `hit` (task PE-B1), reported on press as
    /// PhotoCraft does: a segment sets [`BezierResponse::insert`] with the hit's `t`; an anchor
    /// sets [`BezierResponse::delete`] with that one anchor. One undo step each; no gesture, no
    /// drag, no selection change. The pen draft, when the host passes it as one more path
    /// ([`PathMode::resolve`](crate::ui::pen::PathMode::resolve)), is hit and reported like any
    /// path, under the key the host gave it.
    pub(super) fn anchor_click(&self, hit: BezierHit, mods: Modifiers, out: &mut BezierResponse) {
        // pc: crates/ui-egui/src/tools/anchor_tools.rs:88-114 `pointer`, `click`; :130-150 `apply`.
        let tool = self.acting_tool(mods);
        match hit {
            BezierHit::Segment { path, segment, t } if matches!(tool, BezierTool::AddAnchor | BezierTool::AutoAddDelete) => {
                out.insert = Some((path, segment, t));
            }
            BezierHit::Anchor { path, knot } if matches!(tool, BezierTool::DeleteAnchor | BezierTool::AutoAddDelete) => {
                out.delete = Some((path, vec![knot]));
            }
            _ => {}
        }
    }

    /// The part the hover mark shows under `pointer` while no press is under way: what a press
    /// would take with the acting tool ([`HitMark`](super::HitMark) draws it). None for
    /// [`BezierTool::Direct`], [`BezierTool::Convert`] and [`BezierTool::Show`], and while
    /// another overlay owns the canvas.
    pub(super) fn hover_mark(&self, pass: &Pass<'_>, pointer: Pos2, mods: Modifiers) -> Option<BezierHit> {
        // pc: crates/ui-egui/src/tools/anchor_tools.rs:152-169 `draw_hover`, `draw_pen_hover`;
        // crates/ui-egui/src/tools/direct_select.rs:378-389 `curvature_hover`.
        if !matches!(self.tool, BezierTool::AddAnchor | BezierTool::DeleteAnchor | BezierTool::AutoAddDelete) || mods.command {
            return None;
        }
        if Claim::owner(&pass.ctx, pass.canvas.id).is_some_and(|owner| owner != pass.id) {
            return None;
        }
        let hit = self.hit_at(pass, pointer).filter(|hit| self.tool_takes(hit, mods))?;
        // Only what a press would edit: an anchor under Add and a segment under Delete are taken
        // and do nothing (PhotoCraft draws no mark there).
        let mut would = BezierResponse { id: pass.id, event: None, select: None, insert: None, delete: None, context: None, claimed: false };
        self.anchor_click(hit, mods, &mut would);
        (would.insert.is_some() || would.delete.is_some()).then_some(hit)
    }

    /// The Curvature tool's press (task PE-C1), the press's click count from
    /// [`PrimaryPress::clicks`]:
    ///
    /// - second click of a double-click on an anchor: `gesture.set(ToggleCurvature)`, no drag;
    /// - an anchor: picks it and selects it alone; past `bezier.click_slop` a drag reports
    ///   `MoveCurvature { anchors: [it], by }` (Begin, Update, Commit; Esc cancels);
    /// - a segment: an insert at the hit's `t` clamped to
    ///   [`CURVATURE_T_RANGE`](crate::geometry::curvature::CURVATURE_T_RANGE); a drag reports
    ///   `InsertCurvature { segment, t, by }` as one gesture, a click without drag one Set with
    ///   `by` zero.
    ///
    /// Returns the drag target the pass keeps (the private `Target` gains the two Curvature
    /// kinds).
    pub(super) fn curvature_press(&self, hit: BezierHit, press: PrimaryPress, ui: &Ui, gesture: &mut Gesture<PathEdit>, out: &mut BezierResponse) {
        // pc: crates/ui-egui/src/tools/direct_select.rs:406-443 `curvature_press`, :391-404
        // `curvature_toggle_active`; crates/ui-egui/src/tools/vector_ui/curvature.rs:69-86
        // (double-click toggles). new: the click count is `PrimaryPress::clicks` (lead call 30).
        todo!()
    }

    /// The Curvature tool's keys while the overlay has focus (task PE-C1): the arrows move the
    /// selected anchors as one `MoveCurvature` Set (1 unit, ⇧ 10: the overlay's
    /// [`steps`](BezierOverlay::steps)); Enter toggles the picked anchor (`ToggleCurvature` Set);
    /// Delete, Backspace, Insert, the Page keys and Esc as [`BezierTool::Direct`].
    pub(super) fn curvature_keys(&self, ui: &Ui, gesture: &mut Gesture<PathEdit>, out: &mut BezierResponse) {
        // new: Photoshop's Curvature Pen moves points with the arrows; the toggle on Enter is the
        // keyboard path to the double-click (step7-pen "Path overlay: new tools").
        todo!()
    }
}
