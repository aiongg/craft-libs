//! Tests of the drag rules (L6).

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `accepts_move` refuses what the tree's `accepts_drop` refuses (the Background rules), so no
// marker shows.
#[test]
#[ignore = "skeleton"]
fn accepts_drop_refusal_shows_no_marker() {
    todo!()
}

// Effects rows `Into` another layer are a `Transfer` (Alt copies); onto their own layer refused.
#[test]
#[ignore = "skeleton"]
fn effects_drag_onto_a_layer_outlines_it_and_reports_a_transfer() {
    todo!()
}

// A Smart Filter row among its layer's filters is a `SubRows` move with `to` counted after taking
// it out; among another layer's filters refused.
#[test]
#[ignore = "skeleton"]
fn smart_filter_drag_reorders_within_its_object() {
    todo!()
}

// `drop_preview` is `Some` only while `preview_on_canvas_when_reordering` is on and the target is a
// layer move.
#[test]
#[ignore = "skeleton"]
fn drop_preview_is_reported_only_when_the_setting_is_on() {
    todo!()
}
