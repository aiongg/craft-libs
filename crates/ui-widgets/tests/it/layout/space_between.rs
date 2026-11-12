//! A `SpaceBetween` row narrows its children to fit as any row does, then spreads the room they leave: a wrapping
//! first child leaves the children after it their room, and a row that fits places and runs as before.

use craft_ui::buttons::Button;
use craft_ui::contract::{Props as _, RowAlign, Show as _, ShowContainer as _};
use craft_ui::layout_components::Row;
use craft_ui::testing::{Fixture, last_pass_runs, trial};
use egui::{Rect, Vec2};

const ROOM: Vec2 = Vec2::new(800.0, 600.0);
const SLACK: f32 = 0.5;
const WORDS: [&str; 3] = ["First choice", "Second choice", "Third choice"];
const END: &str = "End";

/// A row of a wrapping row of three buttons and one more button, spread (`spread`) or at the start, in a room `width`
/// wide.
fn fixture(width: f32, spread: bool) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let row = Row::new().fill_width();
        let row = if spread { row.align(RowAlign::SpaceBetween) } else { row };
        row.show(ui, |ui, _| {
            Row::new().wrap(true).show(ui, |ui, _| {
                for word in WORDS {
                    Button::new(word).show(ui);
                }
            });
            Button::new(END).show(ui);
        });
    });
    fixture.room(Vec2::new(width, ROOM.y));
    fixture
}

/// The rects of the three wrapped buttons and the last one.
fn rects(fixture: &Fixture) -> ([Rect; 3], Rect) {
    let rect = |label: &str| fixture.by_label(label).map_or(Rect::NOTHING, |target| target.rect);
    (WORDS.map(rect), rect(END))
}

/// In a room too short for every button on one line, the wrapping row wraps inside what the last button leaves: the
/// last button keeps its width inside the room, beside the wrapped buttons.
#[trial]
fn a_wrapping_first_child_of_a_spread_row_leaves_the_last_child_its_room() {
    let mut wide = fixture(ROOM.x, true);
    let ([first, second, _], end) = rects(wide.settle());
    assert!(first.width() > 0.0 && end.width() > 0.0, "the buttons measure: {first:?}, {end:?}");
    // Room for two of the three on a line, and the last button beside them.
    let width = second.right() - first.left() + (second.left() - first.right()) * 2.0 + end.width();
    let mut short = fixture(width, true);
    let ([first, second, third], cut) = rects(short.settle());
    assert!(third.top() > first.top() + SLACK, "the third button wraps under the first two: {first:?} {third:?}");
    assert!(cut.right() <= first.left() + width + SLACK, "the last button {cut:?} stays inside the room {width}");
    assert!((cut.width() - end.width()).abs() <= SLACK, "the last button keeps its width: {cut:?} against {end:?}");
    assert!(cut.left() >= second.right() - SLACK, "the last button sits after the wrapped ones: {second:?} {cut:?}");
}

/// A spread row that fits runs its buttons as often as a row at the start, and puts its last child at the room's end.
#[trial]
fn a_spread_row_that_fits_runs_as_often_as_a_start_row() {
    let runs = |spread: bool| {
        let mut fixture = fixture(ROOM.x, spread);
        fixture.frame(Vec::new());
        let buttons: u32 = last_pass_runs(fixture.ctx()).iter().filter(|(name, _)| name.ends_with("Button")).map(|(_, runs)| runs).sum();
        let (words, end) = rects(fixture.settle());
        (buttons, words[0], end)
    };
    let ((spread, first, end), (start, _, start_end)) = (runs(true), runs(false));
    assert!(spread > 0, "the first frame runs the buttons");
    assert_eq!(spread, start, "a spread row that fits runs its buttons as often as a start row");
    assert!(end.right() >= first.left() + ROOM.x - SLACK, "the last child sits at the room's end: {end:?}");
    assert!(start_end.right() < end.right() - SLACK, "a start row keeps it beside the others: {start_end:?}");
}
