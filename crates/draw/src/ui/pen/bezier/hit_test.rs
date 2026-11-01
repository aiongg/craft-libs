//! [`BezierOverlay::hit_test`]: the part of a path under a screen point, without a pass.

use egui::Pos2;

use super::{BezierHit, BezierOverlay, EditPath, HandlesShown, PathSelection};
use craft_ui::canvas::CanvasView;

impl BezierOverlay<'_> {
    /// The part of `paths` under the screen point `at`, answered as the overlay's press would
    /// take it (gap row 40): the handles `handles` shows for `selection` first, then anchors,
    /// then segments; within each kind the nearest within `reach` screen points (pass
    /// `bezier.hit`), of equal distances the path drawn last. Segments are measured on the
    /// screen, so a rotated or flipped view hits where it draws. Above
    /// [`MAX_ANCHORS`](super::MAX_ANCHORS) anchors only handles and segments count, as the overlay
    /// draws no anchors then.
    ///
    /// Hosts use it for a tool's hover cursor or for their own tool logic (PhotoCraft's Add and
    /// Delete Anchor targets) without showing an overlay.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands (task PE-A3)")]
    pub fn hit_test(
        paths: &[EditPath],
        selection: &PathSelection,
        handles: HandlesShown,
        view: &CanvasView,
        at: Pos2,
        reach: f32,
    ) -> Option<BezierHit> {
        // pc: crates/vector/src/edit.rs:337 `hit` (handles, anchors, segments; nearest per kind).
        // new: the body of the private `hit_at` and `shown_handles` moved here; `hit_at` then
        // calls this with the pass's handles.
        todo!()
    }
}
