//! Tests of the crop session (task TO-C2).

use kurbo::{Rect, Size};

use super::crop_frame::CropBox;
use super::session::{CropDocKey, CropDocument, CropSession, LeaveFrame};
use craft_ui::canvas::CanvasView;

fn document(key: u64, width: f64, height: f64) -> CropDocument {
    CropDocument::new(CropDocKey(key), Size::new(width, height))
}

fn boxed(x0: f64, y0: f64, x1: f64, y1: f64) -> CropBox {
    CropBox::upright(Rect::new(x0, y0, x1, y1))
}

fn rect_of(session: &CropSession) -> Option<Rect> {
    session.frame().map(|f| f.rect)
}

// `ensure_frame` frames the selection's bounds clipped to the canvas, the canvas without a selection or with an empty one, marks it default, and makes none for an empty canvas.
#[test]
fn default_frame_is_the_selection_bounds_or_the_canvas() {
    let mut session = CropSession::new();
    assert!(session.ensure_frame(&document(1, 200.0, 100.0), false));
    assert_eq!(rect_of(&session), Some(Rect::new(0.0, 0.0, 200.0, 100.0)));
    assert!(session.is_default());
    assert!(!session.pending());
    assert!(!session.ensure_frame(&document(1, 200.0, 100.0), false), "a frame exists");

    let mut session = CropSession::new();
    let selected = document(1, 200.0, 100.0).selection(Some(Rect::new(150.0, 20.0, 300.0, 60.0)));
    assert!(session.ensure_frame(&selected, false));
    assert_eq!(rect_of(&session), Some(Rect::new(150.0, 20.0, 200.0, 60.0)));

    let mut session = CropSession::new();
    let empty = document(1, 200.0, 100.0).selection(Some(Rect::new(10.0, 10.0, 10.0, 50.0)));
    assert!(session.ensure_frame(&empty, false));
    assert_eq!(rect_of(&session), Some(Rect::new(0.0, 0.0, 200.0, 100.0)));

    let mut session = CropSession::new();
    assert!(!session.ensure_frame(&document(1, 0.0, 100.0), false));
    assert!(session.frame().is_none());
    assert!(!session.ensure_frame(&document(1, 50.0, 50.0), true), "a gesture runs");
}

// A default frame made for another document key or another size is replaced; `for_document` drops a drawn frame of another document and reports it.
#[test]
fn another_document_or_size_gets_its_own_default_frame() {
    let mut session = CropSession::new();
    session.ensure_frame(&document(1, 200.0, 100.0), false);
    assert!(session.ensure_frame(&document(1, 300.0, 100.0), false));
    assert_eq!(rect_of(&session), Some(Rect::new(0.0, 0.0, 300.0, 100.0)));
    assert!(session.ensure_frame(&document(2, 300.0, 100.0), false));
    assert!(session.is_default());

    assert!(!session.for_document(CropDocKey(2)));
    assert!(session.frame().is_some());
    session.set(boxed(10.0, 10.0, 50.0, 50.0), false, false);
    assert!(session.for_document(CropDocKey(3)), "a drawn frame was dropped");
    assert!(session.frame().is_none());
    assert!(!session.can_undo());

    session.ensure_frame(&document(3, 10.0, 10.0), false);
    assert!(!session.for_document(CropDocKey(4)), "an untouched default frame is no discard");
    assert!(session.frame().is_none());
}

// A busy drag settles as one step on its release, a key step is one step each; Undo and Redo go back and forth and give the frame with its default mark; a new step drops the redo side.
#[test]
fn box_undo_redo_steps_committed_gestures_only() {
    let mut session = CropSession::new();
    session.ensure_frame(&document(1, 200.0, 100.0), false);
    let start = session.frame();
    assert!(!session.can_undo());

    for x in [10.0, 20.0, 30.0] {
        session.set(boxed(x, 0.0, 100.0, 100.0), false, true);
    }
    assert!(session.can_undo(), "an unrecorded change counts, so Undo never falls through to the document");
    session.set(boxed(30.0, 0.0, 100.0, 100.0), false, false);
    assert!(session.pending());
    session.set(boxed(31.0, 0.0, 100.0, 100.0), false, false);

    assert!(session.undo());
    assert_eq!(session.frame(), Some(boxed(30.0, 0.0, 100.0, 100.0)));
    assert!(session.undo());
    assert_eq!(session.frame(), start);
    assert!(session.is_default(), "the default mark comes back with the step");
    assert!(!session.can_undo());
    assert!(!session.undo());
    assert!(session.can_redo());
    assert!(session.redo());
    assert_eq!(session.frame(), Some(boxed(30.0, 0.0, 100.0, 100.0)));
    assert!(!session.is_default());

    session.set(boxed(5.0, 5.0, 60.0, 60.0), false, false);
    assert!(!session.can_redo(), "a new step drops the redo side");
    assert!(!session.redo());
}

// After Undo back to the untouched default frame the box still owns Undo and Redo (`pending` is false there), so Redo steps the box and not the document; with nothing either way the box owns neither.
#[test]
fn the_box_owns_undo_after_undoing_back_to_the_default_frame() {
    let mut session = CropSession::new();
    session.ensure_frame(&document(1, 200.0, 100.0), false);
    assert!(!session.owns_undo());
    session.set(boxed(10.0, 10.0, 80.0, 60.0), false, false);
    assert!(session.owns_undo());
    assert!(session.undo());
    assert!(session.is_default() && !session.pending());
    assert!(session.owns_undo(), "Redo is still the box's");
    assert!(session.redo());
    assert_eq!(session.frame(), Some(boxed(10.0, 10.0, 80.0, 60.0)));
}

// `revert` goes back to the latest recorded step while a drag runs, without a copy of it in the session.
#[test]
fn revert_goes_back_to_the_latest_step() {
    let mut session = CropSession::new();
    session.ensure_frame(&document(1, 200.0, 100.0), false);
    let first = boxed(10.0, 10.0, 80.0, 60.0);
    session.set(first, false, false);
    session.set(boxed(20.0, 20.0, 90.0, 70.0), false, true);
    session.revert();
    assert_eq!(session.frame(), Some(first));
    assert!(session.pending());
}

// `leave`: Dropped for the untouched default frame, Apply for a drawn one, Kept while a gesture runs or the eraser stands in, Nothing without a frame; Straighten disarms.
#[test]
fn leaving_drops_a_default_frame_and_applies_a_drawn_one() {
    let mut session = CropSession::new();
    assert_eq!(session.leave(false), LeaveFrame::Nothing);

    session.ensure_frame(&document(1, 200.0, 100.0), false);
    session.straighten_mut().toggle();
    assert_eq!(session.leave(false), LeaveFrame::Dropped);
    assert!(session.frame().is_none());
    assert!(!session.straighten().armed());

    session.ensure_frame(&document(1, 200.0, 100.0), false);
    let drawn = boxed(10.0, 10.0, 80.0, 60.0);
    session.set(drawn, false, false);
    assert_eq!(session.leave(true), LeaveFrame::Kept);
    assert_eq!(session.frame(), Some(drawn));
    session.clear();
    session.ensure_frame(&document(1, 200.0, 100.0), false);
    assert_eq!(session.leave(true), LeaveFrame::Dropped, "an untouched default frame goes even while the tool is kept");
    session.ensure_frame(&document(1, 200.0, 100.0), false);
    session.set(drawn, false, false);
    assert_eq!(session.leave(false), LeaveFrame::Apply(drawn));
    assert_eq!(session.frame(), Some(drawn), "the app clears or restores");
    session.clear();
    assert_eq!(session.leave(false), LeaveFrame::Nothing);
}

// `keep` then `clear` then `restore`: frame, default mark, history and the kept view come back.
#[test]
fn a_refused_commit_gives_frame_and_steps_back() {
    let mut session = CropSession::new();
    session.ensure_frame(&document(1, 200.0, 100.0), false);
    let drawn = boxed(10.0, 10.0, 80.0, 60.0);
    session.set(drawn, false, false);
    session.press();
    let view = CanvasView::new(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 100.0)), kurbo::Point::new(5.0, 5.0), 2.0);
    let kept = session.keep(Some(view));

    session.clear();
    assert!(session.frame().is_none());
    assert!(!session.can_undo());

    assert_eq!(session.restore(kept).map(|v| v.zoom), Some(2.0));
    assert_eq!(session.frame(), Some(drawn));
    assert!(session.pending());
    assert!(session.editing());
    assert!(session.can_undo());
    assert!(session.undo());
    assert!(session.is_default());
}

// `ratio_base(true)` keeps the first frame of the run until `ratio_base(false)` (#7752).
#[test]
fn a_ratio_edit_fits_from_the_frame_it_began_with() {
    let mut session = CropSession::new();
    assert_eq!(session.ratio_base(true), None);
    session.ensure_frame(&document(1, 200.0, 100.0), false);
    let first = session.frame();
    assert_eq!(session.ratio_base(true), first);
    session.set(boxed(0.0, 0.0, 20.0, 100.0), false, true);
    assert_eq!(session.ratio_base(true), first, "the run keeps its first frame");
    session.ratio_base(false);
    assert_eq!(session.ratio_base(true), session.frame(), "a new run begins from the current frame");
}
