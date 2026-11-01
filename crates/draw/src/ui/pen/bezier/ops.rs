//! Path operations on the overlays' model: inserting and removing knots, the Curvature reflow, and
//! [`PathEdit::apply`], so a host whose path type is [`EditPath`] needs no edit code of its own,
//! and the pen draft edits itself with the same rules.

#![expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task PE-A2)")]

use super::model::{EditPath, EditSubpath, PathEdit};

impl EditSubpath {
    /// Add Anchor Point: inserts a knot into the segment leaving knot `segment` at its parameter
    /// `t` and returns the new knot's index (`segment + 1`). A curved segment is split at `t`
    /// (de Casteljau): its shape does not change, the two neighbours' facing handles shorten, and
    /// the new knot is smooth with the split's handles. A straight segment (both handles
    /// retracted) gets a corner knot at the cubic's point at `t` (the parameter hit tests and the
    /// preview anchor use, which is not linear along the line). `None`, and no change, when `t` is
    /// not finite or not strictly between 0 and 1, or no segment leaves `segment`.
    pub fn insert_at(&mut self, segment: usize, t: f64) -> Option<usize> {
        // pc: crates/vector/src/edit.rs:212-241 `add_anchor` (`0 < t < 1` strictly; the straight
        // case evaluates the cubic at `t`, edit.rs:223-227).
        todo!()
    }

    /// Delete Anchor Point: removes knot `knot` and returns whether it existed. Its neighbours are
    /// joined by one segment that keeps the previous knot's out handle and the next knot's in
    /// handle; a closed subpath stays closed. Removing the only knot leaves an empty subpath,
    /// which the caller drops (PhotoCraft removes the subpath and refuses to delete a path's only
    /// anchor; that check is the host's, on the whole path).
    pub fn remove_knot(&mut self, knot: usize) -> bool {
        // pc: crates/vector/src/edit.rs:243-260 `delete_anchor`.
        todo!()
    }

    /// The Curvature reflow: every smooth knot's handles become
    /// [`curvature_handles`](crate::geometry::curvature::curvature_handles) of the subpath's
    /// anchors, with the corner knots (`!smooth`) as corners and `closed` as it is. Corner knots
    /// keep their own handles.
    pub fn reflow(&mut self) {
        // pc: crates/vector/src/edit.rs:125-147 `move_anchors_curvature` (the loop after the move).
        todo!()
    }
}

impl PathEdit {
    /// Applies the edit to `path` and returns whether it changed it. Nothing changes when the
    /// edit's key is not `path.key` or a knot or segment it names does not exist. The rules are
    /// PhotoCraft's kernel's:
    ///
    /// - `MoveAnchors`: each listed knot moves once, with its handles, by `by`.
    /// - `MoveHandle`: the handle goes to `to`; `alone` makes the knot a corner; a smooth knot
    ///   turns its other handle to stay collinear, keeping that handle's length (a retracted one
    ///   stays retracted).
    /// - `BendSegment`: a straight segment moves with both its anchors; a curve's handles move so
    ///   the point at `t` (clamped to 0.05..=0.95, 0.5 when not finite) moves exactly `by`, the
    ///   nearer handle more; smooth end knots stay smooth.
    /// - `Convert`: `None` a corner with retracted handles; `Some(out)` a smooth knot with the out
    ///   handle at `out` and the in handle mirrored.
    /// - `MoveCurvature`: `MoveAnchors`, then [`EditSubpath::reflow`] on every subpath touched.
    /// - `InsertCurvature`: [`EditSubpath::insert_at`] (the new knot made smooth, on a straight
    ///   segment too, as Photoshop's Curvature Pen adds a smooth point), the new knot moved by
    ///   `by`, then the subpath reflowed.
    /// - `ToggleCurvature`: a smooth knot becomes a corner with retracted handles, a corner
    ///   becomes smooth, then the subpath reflows.
    pub fn apply(&self, path: &mut EditPath) -> bool {
        // pc: crates/vector/src/edit.rs:110-156 `move_anchors`, `move_anchors_curvature`,
        // `toggle_curvature_point`; :158-206 `move_handle` (with `keep_smooth` :96-108),
        // `convert_point`, `bend_segment`; crates/ui-egui/src/tools/direct_select.rs:406-443
        // `curvature_press` (insert, then the drag, one step).
        todo!()
    }
}
