//! Tests of the outline draft and the selection operation.

use egui::Modifiers;
use kurbo::Point;

use super::SelectionOperation;
use super::draft::{DraftEvent, DraftKind, DraftPoint, DraftReach, OutlineDraft};

const REACH: DraftReach = DraftReach { close: 8.0, same_point: 3.0 };
const NONE: Modifiers = Modifiers::NONE;
const ALT: Modifiers = Modifiers::ALT;

fn at(x: f64, y: f64, modifiers: Modifiers) -> DraftPoint {
    DraftPoint::new(Point::new(x, y), modifiers, 1.0)
}

fn at_zoom(x: f64, y: f64, zoom: f64) -> DraftPoint {
    DraftPoint::new(Point::new(x, y), NONE, zoom)
}

/// A click: press then release.
fn click(draft: &mut OutlineDraft, x: f64, y: f64, modifiers: Modifiers, has_selection: bool) -> Option<DraftEvent> {
    let pressed = draft.press(at(x, y, modifiers), has_selection);
    let released = draft.release(at(x, y, modifiers));
    released.or(pressed)
}

// Mixed, ⌥ pressed after the press: clicks add straight vertices, drags add freehand points;
// `straight()` holds exactly the clicked indices.
#[test]
fn draft_alt_latch_straight_and_freehand() {
    let mut draft = OutlineDraft::new(DraftKind::Mixed, REACH);
    draft.press(at(0.0, 0.0, NONE), true);
    draft.drag(at(10.0, 0.0, NONE));
    draft.drag(at(20.0, 0.0, NONE));
    draft.modifiers_changed(ALT);
    // A click with ⌥ held adds a straight vertex; the release keeps the outline open.
    assert_eq!(click(&mut draft, 40.0, 0.0, ALT, true), Some(DraftEvent::Changed));
    assert!(draft.is_open());
    // A drag with ⌥ held draws freehand again.
    draft.press(at(60.0, 0.0, ALT), true);
    draft.drag(at(60.0, 10.0, ALT));
    draft.drag(at(60.0, 20.0, ALT));
    draft.release(at(60.0, 20.0, ALT));
    assert_eq!(draft.straight(), &[3, 4]);
    assert_eq!(draft.points().len(), 7);
    draft.hover(at(70.0, 20.0, ALT));
    assert_eq!(draft.rubber_band(), Some([Point::new(60.0, 20.0), Point::new(70.0, 20.0)]));
}

// Mixed with a selection, ⌥ held at the press: `Started.intent` keeps ⌥ and the outline is
// freehand until ⌥ is released and pressed again.
#[test]
fn draft_alt_before_press_subtracts_until_released() {
    let mut draft = OutlineDraft::new(DraftKind::Mixed, REACH);
    assert_eq!(draft.press(at(0.0, 0.0, ALT), true), Some(DraftEvent::Started { intent: ALT }));
    draft.modifiers_changed(ALT);
    draft.drag(at(10.0, 0.0, ALT));
    assert!(draft.rubber_band().is_none());
    draft.modifiers_changed(NONE);
    draft.modifiers_changed(ALT);
    draft.release(at(10.0, 0.0, ALT));
    // Straight segments are on now, so the release left the outline open.
    assert!(draft.is_open());
    assert!(draft.takes_retract());
}

// Mixed and Polygonal with nothing selected: ⌥ at the press starts straight segments and is
// dropped from `Started.intent`.
#[test]
fn draft_alt_with_nothing_selected_is_polygonal() {
    for kind in [DraftKind::Mixed, DraftKind::Polygonal] {
        let mut draft = OutlineDraft::new(kind, REACH);
        assert_eq!(draft.press(at(0.0, 0.0, ALT), false), Some(DraftEvent::Started { intent: NONE }));
        draft.release(at(0.0, 0.0, ALT));
        assert!(draft.is_open(), "{kind:?} waits for the next click");
    }
}

// Close within 8 pt of the first point (3 or more points) and within 3 pt of the last point, at
// zoom 1 and 4; a segment shorter than 8 pt still places its vertex.
#[test]
fn draft_closes_near_first_and_on_repeat_click_only() {
    for zoom in [1.0, 4.0] {
        let unit = 1.0 / zoom;
        let build = || {
            let mut draft = OutlineDraft::new(DraftKind::Polygonal, REACH);
            for (x, y) in [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0)] {
                draft.press(at_zoom(x, y, zoom), true);
                draft.release(at_zoom(x, y, zoom));
            }
            draft
        };
        let mut draft = build();
        assert!(matches!(draft.press(at_zoom(7.0 * unit, 0.0, zoom), true), Some(DraftEvent::Closed { .. })));
        let mut draft = build();
        assert!(matches!(draft.press(at_zoom(9.0 * unit, 0.0, zoom), true), Some(DraftEvent::Changed)));
        // 5 pt from the last vertex: the next vertex, though the segment is shorter than 8 pt.
        let mut draft = build();
        assert_eq!(draft.press(at_zoom(100.0, 100.0 - 5.0 * unit, zoom), true), Some(DraftEvent::Changed));
        assert_eq!(draft.points().len(), 4);
        let mut draft = build();
        assert!(matches!(draft.press(at_zoom(100.0, 100.0 - 2.0 * unit, zoom), true), Some(DraftEvent::Closed { .. })));
    }
}

// Mixed: ⌥ released with the button up closes; with the button down drawing goes on.
#[test]
fn draft_release_alt_with_button_up_closes() {
    let mut draft = OutlineDraft::new(DraftKind::Mixed, REACH);
    draft.press(at(0.0, 0.0, ALT), false);
    draft.release(at(0.0, 0.0, ALT));
    click(&mut draft, 50.0, 0.0, ALT, false);
    click(&mut draft, 50.0, 50.0, ALT, false);
    match draft.modifiers_changed(NONE) {
        Some(DraftEvent::Closed { points, straight }) => {
            assert_eq!(points.len(), 3);
            assert_eq!(straight, vec![1, 2]);
        }
        other => panic!("expected Closed, got {other:?}"),
    }
    assert!(!draft.is_open());

    // Button down: ⌥ released goes on freehand, and the release then closes.
    draft.press(at(0.0, 0.0, ALT), false);
    draft.drag(at(10.0, 0.0, ALT));
    assert_eq!(draft.modifiers_changed(NONE), Some(DraftEvent::Changed));
    draft.drag(at(10.0, 10.0, NONE));
    assert!(matches!(draft.release(at(10.0, 10.0, NONE)), Some(DraftEvent::Closed { .. })));
}

// `retract` removes the last vertex between clicks, not while the button is down; the last one
// gives Cancelled; `straight` indices past the end go.
#[test]
fn draft_retract_removes_last_vertex_then_cancels() {
    let mut draft = OutlineDraft::new(DraftKind::Polygonal, REACH);
    click(&mut draft, 0.0, 0.0, NONE, true);
    click(&mut draft, 50.0, 0.0, NONE, true);
    draft.press(at(50.0, 50.0, NONE), true);
    assert_eq!(draft.retract(), None);
    draft.release(at(50.0, 50.0, NONE));
    assert_eq!(draft.straight(), &[1, 2]);
    assert_eq!(draft.retract(), Some(DraftEvent::Changed));
    assert_eq!(draft.straight(), &[1]);
    assert_eq!(draft.retract(), Some(DraftEvent::Changed));
    assert_eq!(draft.retract(), Some(DraftEvent::Cancelled));
    assert!(!draft.is_open());
    assert_eq!(draft.retract(), None);
}

// `cancel` and `for_document` with another key drop an open outline; an empty draft gives None.
#[test]
fn draft_dropped_on_tool_or_document_change() {
    let mut draft = OutlineDraft::new(DraftKind::Polygonal, REACH);
    assert_eq!(draft.cancel(), None);
    assert_eq!(draft.for_document(1), None);
    click(&mut draft, 0.0, 0.0, NONE, true);
    assert_eq!(draft.for_document(1), None);
    assert!(draft.is_open());
    assert_eq!(draft.for_document(2), Some(DraftEvent::Cancelled));
    assert!(draft.points().is_empty());
    click(&mut draft, 0.0, 0.0, NONE, true);
    assert_eq!(draft.cancel(), Some(DraftEvent::Cancelled));
    assert_eq!(draft.cancel(), None);
}

// Polygonal with ⇧: the second vertex turns to the nearest 45° keeping its distance; not the first
// vertex, not with ⌥; a close with 2 points cancels.
#[test]
fn draft_polygonal_shift_turns_to_45_degrees() {
    let shift = Modifiers::SHIFT;
    let mut draft = OutlineDraft::new(DraftKind::Polygonal, REACH);
    click(&mut draft, 3.0, 4.0, shift, true);
    assert_eq!(draft.points()[0], Point::new(3.0, 4.0));
    click(&mut draft, 103.0, 14.0, shift, true);
    let turned = draft.points()[1];
    assert!((turned.y - 4.0).abs() < 1e-9, "turned to horizontal: {turned:?}");
    let length = (103.0f64 - 3.0).hypot(10.0);
    assert!((turned.x - 3.0 - length).abs() < 1e-9);
    click(&mut draft, 200.0, 50.0, shift | ALT, true);
    assert_eq!(draft.points()[2], Point::new(200.0, 50.0));
    let mut two = OutlineDraft::new(DraftKind::Polygonal, REACH);
    click(&mut two, 0.0, 0.0, NONE, true);
    click(&mut two, 50.0, 0.0, NONE, true);
    assert_eq!(two.close(), Some(DraftEvent::Cancelled));
}

// `resolve`: modifiers override the bar; ⌥ or ⇧⌥ with nothing selected and bar New give New; a bar
// Subtract or Intersect is kept with nothing selected.
#[test]
fn resolve_alt_without_selection_is_new() {
    use SelectionOperation::*;
    let shift = Modifiers::SHIFT;
    assert_eq!(SelectionOperation::resolve(Subtract, shift, true), Add);
    assert_eq!(SelectionOperation::resolve(Add, NONE, true), Add);
    assert_eq!(SelectionOperation::resolve(New, ALT, true), Subtract);
    assert_eq!(SelectionOperation::resolve(New, ALT, false), New);
    assert_eq!(SelectionOperation::resolve(New, shift | ALT, false), New);
    assert_eq!(SelectionOperation::resolve(New, shift, false), Add);
    assert_eq!(SelectionOperation::resolve(Subtract, NONE, false), Subtract);
    assert_eq!(SelectionOperation::resolve(Intersect, NONE, false), Intersect);
    assert_eq!(SelectionOperation::resolve(Subtract, ALT, false), Subtract);
}

#[test]
fn reach_reads_the_outline_draft_tokens() {
    let ctx = egui::Context::default();
    crate::theme::install(&ctx, &crate::theme::Base::suite_default());
    assert_eq!(DraftReach::of(&crate::theme::current(&ctx)), REACH);
}
