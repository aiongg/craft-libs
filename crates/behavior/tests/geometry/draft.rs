//! The outline draft: closing by clicking its start, and retracting a vertex.

use craft_behavior::geometry::{DraftEvent, DraftKind, DraftPoint, DraftReach, OutlineDraft};
use craft_keymap::Mods;
use kurbo::Point;

const REACH: DraftReach = DraftReach::new(8.0, 3.0);

fn at(x: f64, y: f64, zoom: f64) -> DraftPoint {
    DraftPoint::new(Point::new(x, y), Mods::NONE, zoom)
}

fn click(draft: &mut OutlineDraft, x: f64, y: f64) -> Option<DraftEvent> {
    let pressed = draft.press(at(x, y, 1.0), true);
    draft.release(at(x, y, 1.0)).or(pressed)
}

#[test]
fn a_polygon_closes_by_clicking_near_its_start_at_any_zoom() {
    for zoom in [1.0, 4.0] {
        let unit = 1.0 / zoom;
        let build = || {
            let mut draft = OutlineDraft::new(DraftKind::Polygonal, REACH);
            for (x, y) in [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0)] {
                draft.press(at(x, y, zoom), true);
                draft.release(at(x, y, zoom));
            }
            draft
        };
        assert!(matches!(build().press(at(7.0 * unit, 0.0, zoom), true), Some(DraftEvent::Closed { .. })));
        assert_eq!(build().press(at(9.0 * unit, 0.0, zoom), true), Some(DraftEvent::Changed));
    }
}

#[test]
fn retract_removes_the_last_vertex_then_cancels() {
    let mut draft = OutlineDraft::new(DraftKind::Polygonal, REACH);
    click(&mut draft, 0.0, 0.0);
    click(&mut draft, 50.0, 0.0);
    click(&mut draft, 50.0, 50.0);
    assert_eq!(draft.straight(), &[1, 2]);
    assert_eq!(draft.retract(), Some(DraftEvent::Changed));
    assert_eq!(draft.straight(), &[1]);
    assert_eq!(draft.retract(), Some(DraftEvent::Changed));
    assert_eq!(draft.retract(), Some(DraftEvent::Cancelled));
    assert!(!draft.is_open());
}

#[test]
fn alt_with_nothing_selected_starts_straight_segments_without_the_alt_intent() {
    let alt = Mods::ALT;
    for kind in [DraftKind::Mixed, DraftKind::Polygonal] {
        let mut draft = OutlineDraft::new(kind, REACH);
        let started = draft.press(DraftPoint::new(Point::ZERO, alt, 1.0), false);
        assert!(matches!(started, Some(DraftEvent::Started { intent, .. }) if intent == Mods::NONE));
        draft.release(DraftPoint::new(Point::ZERO, alt, 1.0));
        assert!(draft.is_open(), "{kind:?} waits for the next click");
    }
}
