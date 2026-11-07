//! `ThumbRow` rules: the order thumbs keep, split halves, wrapping.

use craft_behavior::{MoveMods, NavMove};
use craft_ui::contract::Bind;
use craft_ui::testing::trial;
use craft_ui::value_drag::{Anchor, Order, RowOp, Thumb, ThumbOp, ThumbRow};

fn forward(thumb: usize) -> RowOp {
    RowOp::new(thumb, ThumbOp::Move(NavMove::Forward, MoveMods::PLAIN))
}

/// A thumb stops at the gap it keeps from its neighbours, whether a step, Home, End or a set moves it, and it does
/// not push them.
#[trial]
fn a_thumb_stops_at_its_neighbours() {
    let thumbs = [
        Thumb::new("Black").order(Order::between(None, Some(1), 10.0)),
        Thumb::new("White").order(Order::between(Some(0), None, 10.0)),
    ];
    let row = ThumbRow::new(Bind::given([0.0, 100.0]), 0.0..=255.0).step(1.0).thumbs(thumbs);
    let rules = row.rules();
    assert_eq!(rules.apply(&RowOp::new(0, ThumbOp::Move(NavMove::End, MoveMods::PLAIN)), &[0.0, 100.0]), [90.0, 100.0]);
    assert_eq!(rules.apply(&RowOp::new(1, ThumbOp::Set(5.0)), &[0.0, 100.0]), [0.0, 10.0]);
    assert_eq!(rules.apply(&forward(0), &[89.0, 100.0]), [90.0, 100.0]);
    assert_eq!(rules.apply(&forward(0), &[90.0, 100.0]), [90.0, 100.0]);
}

/// The two halves of a split thumb never cross, and the pair stays inside the thumbs it is ordered with.
#[trial]
fn the_halves_of_a_split_thumb_do_not_cross() {
    let thumbs = [Thumb::new("Black").split(), Thumb::new("Black")];
    let row = ThumbRow::new(Bind::given([40.0, 60.0]), 0.0..=255.0).step(1.0).thumbs(thumbs);
    let rules = row.rules();
    assert_eq!(rules.apply(&RowOp::new(0, ThumbOp::Move(NavMove::End, MoveMods::PLAIN)), &[40.0, 60.0]), [60.0, 60.0]);
    assert_eq!(rules.apply(&RowOp::new(1, ThumbOp::Move(NavMove::Start, MoveMods::PLAIN)), &[40.0, 60.0]), [40.0, 40.0]);
}

/// A step past the end of a wrapping row comes back at the start; an unwrapping row stops at the end.
#[trial]
fn a_wrapping_row_comes_back_at_the_start() {
    let wrapping = ThumbRow::new(Bind::given([359.0, 10.0]), 0.0..=360.0).step(1.0).wrap().thumbs([Thumb::new("From"), Thumb::new("To")]);
    assert_eq!(wrapping.rules().apply(&forward(0), &[359.0, 10.0]), [0.0, 10.0]);
    let plain = ThumbRow::new(Bind::given([359.0, 10.0]), 0.0..=360.0).step(1.0).thumbs([Thumb::new("From"), Thumb::new("To")]);
    assert_eq!(plain.rules().apply(&forward(0), &[360.0, 10.0]), [360.0, 10.0]);
}

/// In an anchored row the last thumb stays within the span past the first and the thumbs keep their order round
/// the period.
#[trial]
fn an_anchored_row_keeps_its_span_and_order() {
    let thumbs = [Thumb::new("a"), Thumb::new("b"), Thumb::new("c"), Thumb::new("d")];
    let row = ThumbRow::new(Bind::given([340.0, 10.0, 50.0, 80.0]), 0.0..=360.0).step(1.0).anchor(Anchor::new(0, 100.0)).thumbs(thumbs);
    let rules = row.rules();
    let at = [340.0, 10.0, 50.0, 80.0];
    // The last thumb is 100 past the first at most: 340 + 100 = 80 round the period.
    assert_eq!(rules.apply(&forward(3), &at), at);
    // A middle thumb stops at the next one.
    assert_eq!(rules.apply(&RowOp::new(1, ThumbOp::Set(70.0)), &at), [340.0, 50.0, 50.0, 80.0]);
}
