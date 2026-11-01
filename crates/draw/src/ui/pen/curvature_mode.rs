//! The pen draft's Curvature mode ([`PenMode::Curvature`](super::PenMode::Curvature), task
//! PE-B2): the Curvature Pen while a path is being drawn.

use egui::{Response, Ui};

use super::{PenDraft, PenResponse};
use craft_ui::canvas::CanvasView;

impl PenDraft<'_> {
    /// One pass in Curvature mode, with the Pen's id, gesture, claim, waiting press
    /// ([`Claim::turn`](craft_ui::canvas::Claim::turn)), keys, AccessKit parts and response as
    /// [`show`](PenDraft::show) has them (share its private helpers rather than copy them):
    ///
    /// - Press on the first anchor of a draft with 3 or more anchors: closes at once (`closed`
    ///   set, Commit, [`finished`](PenResponse::finished)), as PhotoCraft and Photoshop; the Pen
    ///   closes on release instead (lead call 8).
    /// - Press on another anchor (within `bezier.hit`): grabs it; past `bezier.click_slop` it
    ///   follows the pointer and the draft reflows on every move ([`EditSubpath::reflow`](super::EditSubpath::reflow)).
    /// - Press elsewhere: a new smooth anchor at the press (an Update); it follows the pointer
    ///   while the button is down, past `bezier.click_slop` (lead call 9: PhotoCraft moves it on
    ///   any move). The first two anchors give straight segments; from three on the handles
    ///   follow [`curvature_handles`](crate::geometry::curvature::curvature_handles).
    /// - The second click of a double-click ([`PrimaryPress::clicks`](craft_ui::canvas::claim::PrimaryPress::clicks)
    ///   == 2) on an anchor toggles it between smooth and corner and reflows; on empty canvas the
    ///   first click's anchor is under it, so a double-click places a corner point.
    /// - ⇧ and ⌥ change nothing; no resume from an open path's end (`ends` is ignored).
    /// - Keys: Enter and Esc end the draft as the Pen's; Backspace, Delete and
    ///   [`remove_last`](PenDraft::remove_last) remove the last anchor and reflow.
    /// - Look: the draft line, the anchors (squares, lead call 7; the last filled), no handles;
    ///   the hover preview: with 2 or more anchors the last segment, dashed (`bezier.line`,
    ///   `bezier.rubber_dash`, `bezier.rubber_gap`), of the curve `curvature_handles` makes
    ///   through the anchors plus the pointer, from the last anchor to the pointer; with one
    ///   anchor a straight dashed line; none while a press is down; the close cue over the first
    ///   anchor with 3 or more anchors.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands (task PE-B2)")]
    pub(super) fn show_curvature(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> PenResponse {
        // pc: crates/ui-egui/src/tools/vector_ui/curvature.rs:29-52 `curvature_down` (close with
        // 3+ at :39, grab :47-50), :54-61 `curvature_move`, :69-86 `curvature_toggle`;
        // crates/ui-egui/src/tools/vector_ui/overlay.rs:202-222 (preview curve :208-215, `dashed_line(4, 3)`).
        todo!()
    }
}
