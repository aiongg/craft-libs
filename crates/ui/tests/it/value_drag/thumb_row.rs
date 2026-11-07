//! `ThumbRow` rules: the order thumbs keep, split halves, wrapping.

use craft_behavior::{MoveMods, NavMove, PartKey};
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, Story, StoryData, StoryOutput, trial};
use craft_ui::value_drag::{Anchor, Order, RowOp, Thumb, ThumbOp, ThumbRow};
use egui::{Key, Modifiers, Ui};

fn forward(thumb: usize) -> RowOp {
    RowOp::new(thumb, ThumbOp::Move(NavMove::Forward, MoveMods::PLAIN))
}

/// A thumb stops at the gap it keeps from its neighbours, whether a step, Home, End or a set moves it, and it does
/// not push them.
#[trial]
fn a_thumb_stops_at_its_neighbours() {
    let thumbs = [Thumb::new("Black").order(Order::between(None, Some(1), 10.0)), Thumb::new("White").order(Order::between(Some(0), None, 10.0))];
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

/// Two thumbs with a default of their own and a binding whose reset is different from both.
fn levels(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let thumbs = [Thumb::new("Black").reset(10.0), Thumb::new("White").reset(90.0)];
    ThumbRow::new(Bind::bound(data.value("row", [20.0, 80.0])).reset([0.0, 100.0]), 0.0..=100.0)
        .step(1.0)
        .thumbs(thumbs)
        .label("Levels")
        .show(ui)
        .into()
}

fn levels_fixture() -> Fixture {
    let mut fixture = Fixture::story(&Story::new("levels", levels));
    fixture.settle();
    fixture
}

fn row(fixture: &Fixture) -> [f64; 2] {
    *fixture.data().value("row", [f64::NAN; 2])
}

fn thumb(fixture: &Fixture, i: u64) -> craft_ui::testing::Target {
    let id = fixture.root_id().expect("the row is shown");
    fixture.part(id, PartKey::new("thumb").at(i)).expect("the thumb has a sub-node")
}

/// A reset aimed at a thumb that has a default of its own sets that thumb alone, by a double click on it or by
/// Delete while it is focused; the other thumb keeps its value.
#[trial]
fn a_reset_of_a_thumb_with_a_default_sets_only_that_thumb() {
    let mut fixture = levels_fixture();
    let at = thumb(&fixture, 1).center();
    fixture.frame(vec![moved(at)]);
    for _ in 0..2 {
        fixture.frame(vec![press(at)]);
        fixture.frame(vec![release(at)]);
    }
    fixture.idle(2);
    assert_eq!(row(&fixture), [20.0, 90.0], "a double click on the second thumb restores its default alone");

    let mut fixture = levels_fixture();
    let first = thumb(&fixture, 0);
    fixture.access(AccessActionKind::Focus, &first);
    fixture.idle(2);
    fixture.key(Key::Delete, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(row(&fixture), [10.0, 80.0], "Reset with the first thumb focused restores its default alone");
}

/// A double click on the row's side label resets the whole binding, even while a thumb with a default of its own
/// holds focus.
#[trial]
fn a_label_reset_restores_the_whole_row_while_a_thumb_with_a_default_is_focused() {
    use craft_ui::contract::{ShowContainer as _, SideLabel};
    use craft_ui::testing::Frames;
    use craft_ui::testing::input::key_press;

    let mut frames = Frames::new();
    let mut value = [20.0, 80.0];
    let mut show = |ui: &mut Ui| {
        crate::label::kit::Field::new("Levels", SideLabel::new()).id_salt("field").show_at(
            ui,
            egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(300.0, 30.0)),
            |ui, _| {
                let thumbs = [Thumb::new("Black").reset(10.0), Thumb::new("White").reset(90.0)];
                ThumbRow::new(Bind::bound(&mut value).reset([0.0, 100.0]), 0.0..=100.0).step(1.0).thumbs(thumbs).show(ui);
            },
        );
    };
    frames.idle(3, &mut show);
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    let tree = frames.tree();
    let label = tree.keys().into_iter().find(|key| tree.role_of(*key) == Some(craft_ui::contract::Role::LABEL)).expect("the field shows its label");
    let at = tree.bounds_of(label).expect("the label has bounds").center();
    frames.frame(vec![moved(at)], &mut show);
    for _ in 0..2 {
        frames.frame(vec![press(at)], &mut show);
        frames.frame(vec![release(at)], &mut show);
    }
    frames.frame(vec![], &mut show);
    drop(show);
    let whole_default = [0.0, 100.0];
    assert_eq!(value, whole_default);
}
