//! `ThumbRow` rules: the order thumbs keep, split halves, wrapping.

use craft_behavior::{Mixed, MoveMods, NavMove, PartKey};
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, Frames, Story, StoryData, StoryOutput, trial};
use craft_ui::value_drag::{Anchor, Order, RowOp, Thumb, ThumbOp, ThumbRow};
use egui::{Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

fn forward(thumb: usize) -> RowOp {
    RowOp::new(thumb, ThumbOp::Move(NavMove::Forward, MoveMods::PLAIN))
}

/// A thumb stops at the gap it keeps from its neighbours, whether a step, Home, End or a set moves it, and it does
/// not push them.
#[trial]
fn a_thumb_stops_at_its_neighbours() {
    let thumbs = [Thumb::new("Black").order(Order::between(None, Some(1), 10.0)), Thumb::new("White").order(Order::between(Some(0), None, 10.0))];
    let row = ThumbRow::new(Bind::given(Mixed::each([0.0, 100.0])), 0.0..=255.0).step(1.0).thumbs(thumbs);
    let rules = row.rules();
    assert_eq!(rules.apply(&RowOp::new(0, ThumbOp::Move(NavMove::End, MoveMods::PLAIN)), &Mixed::each([0.0, 100.0])), Mixed::each([90.0, 100.0]));
    assert_eq!(rules.apply(&RowOp::new(1, ThumbOp::Set(5.0)), &Mixed::each([0.0, 100.0])), Mixed::each([0.0, 10.0]));
    assert_eq!(rules.apply(&forward(0), &Mixed::each([89.0, 100.0])), Mixed::each([90.0, 100.0]));
    assert_eq!(rules.apply(&forward(0), &Mixed::each([90.0, 100.0])), Mixed::each([90.0, 100.0]));
}

/// The two halves of a split thumb never cross, and the pair stays inside the thumbs it is ordered with.
#[trial]
fn the_halves_of_a_split_thumb_do_not_cross() {
    let thumbs = [Thumb::new("Black").split(), Thumb::new("Black")];
    let row = ThumbRow::new(Bind::given(Mixed::each([40.0, 60.0])), 0.0..=255.0).step(1.0).thumbs(thumbs);
    let rules = row.rules();
    assert_eq!(rules.apply(&RowOp::new(0, ThumbOp::Move(NavMove::End, MoveMods::PLAIN)), &Mixed::each([40.0, 60.0])), Mixed::each([60.0, 60.0]));
    assert_eq!(rules.apply(&RowOp::new(1, ThumbOp::Move(NavMove::Start, MoveMods::PLAIN)), &Mixed::each([40.0, 60.0])), Mixed::each([40.0, 40.0]));
}

/// A step past the end of a wrapping row comes back at the start; an unwrapping row stops at the end.
#[trial]
fn a_wrapping_row_comes_back_at_the_start() {
    let wrapping =
        ThumbRow::new(Bind::given(Mixed::each([359.0, 10.0])), 0.0..=360.0).step(1.0).wrap().thumbs([Thumb::new("From"), Thumb::new("To")]);
    assert_eq!(wrapping.rules().apply(&forward(0), &Mixed::each([359.0, 10.0])), Mixed::each([0.0, 10.0]));
    let plain = ThumbRow::new(Bind::given(Mixed::each([359.0, 10.0])), 0.0..=360.0).step(1.0).thumbs([Thumb::new("From"), Thumb::new("To")]);
    assert_eq!(plain.rules().apply(&forward(0), &Mixed::each([360.0, 10.0])), Mixed::each([360.0, 10.0]));
}

/// In an anchored row the last thumb stays within the span past the first and the thumbs keep their order round
/// the period.
#[trial]
fn an_anchored_row_keeps_its_span_and_order() {
    let thumbs = [Thumb::new("a"), Thumb::new("b"), Thumb::new("c"), Thumb::new("d")];
    let row = ThumbRow::new(Bind::given(Mixed::each([340.0, 10.0, 50.0, 80.0])), 0.0..=360.0).step(1.0).anchor(Anchor::new(0, 100.0)).thumbs(thumbs);
    let rules = row.rules();
    let at = Mixed::each([340.0, 10.0, 50.0, 80.0]);
    // The last thumb is 100 past the first at most: 340 + 100 = 80 round the period.
    assert_eq!(rules.apply(&forward(3), &at), at);
    // A middle thumb stops at the next one.
    assert_eq!(rules.apply(&RowOp::new(1, ThumbOp::Set(70.0)), &at), Mixed::each([340.0, 50.0, 50.0, 80.0]));
}

/// Two thumbs with a default of their own and a binding whose reset is different from both.
fn levels(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let thumbs = [Thumb::new("Black").reset(10.0), Thumb::new("White").reset(90.0)];
    ThumbRow::new(Bind::bound(data.value("row", Mixed::each([20.0, 80.0]))).reset(Mixed::each([0.0, 100.0])), 0.0..=100.0)
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
    let row = *fixture.data().value("row", Mixed::each([f64::NAN; 2]));
    row.map(|part| *part.same().expect("every thumb has a number"))
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
    let mut value = Mixed::each([20.0, 80.0]);
    let mut show = |ui: &mut Ui| {
        crate::label::kit::Field::new("Levels", SideLabel::new()).id_salt("field").show_at(
            ui,
            egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(300.0, 30.0)),
            |ui, _| {
                let thumbs = [Thumb::new("Black").reset(10.0), Thumb::new("White").reset(90.0)];
                ThumbRow::new(Bind::bound(&mut value).reset(Mixed::each([0.0, 100.0])), 0.0..=100.0).step(1.0).thumbs(thumbs).show(ui);
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
    let ends = Mixed::each([0.0, 100.0]);
    assert_eq!(value, ends);
}

fn mixed_row(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let thumbs = [Thumb::new("Black"), Thumb::new("White")];
    ThumbRow::new(Bind::bound(data.value("mixed", [Mixed::Mixed, Mixed::Mixed])), 0.0..=100.0)
        .step(1.0)
        .thumbs(thumbs)
        .label("Levels")
        .show(ui)
        .into()
}

/// A key on a thumb whose value is mixed gives that thumb a number and leaves the others mixed.
#[trial]
fn a_key_on_a_mixed_thumb_sets_that_thumb_alone() {
    let mut fixture = Fixture::story(&Story::new("mixed-row", mixed_row));
    fixture.settle();
    let id = fixture.root_id().expect("the row is shown");
    let first = fixture.part(id, PartKey::new("thumb").at(0)).expect("the thumb has a sub-node");
    fixture.access(AccessActionKind::Focus, &first);
    fixture.idle(2);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    let row = *fixture.data().value("mixed", [Mixed::<f64>::Mixed, Mixed::Mixed]);
    let stepped = [Mixed::Same(1.0), Mixed::Mixed];
    assert_eq!(row, stepped, "the first thumb steps from the start of its range; the second stays mixed");
}

const ROW: Rect = Rect { min: Pos2::new(20.0, 20.0), max: Pos2::new(320.0, 50.0) };
const BY: f32 = 10.0;

/// The row after dragging its first thumb by `BY`, with Alt held or not.
fn dragged(thumbs: fn() -> [Thumb; 2], alt: bool) -> [Mixed<f64>; 2] {
    let value = RefCell::new(Mixed::each([40.0, 60.0]));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let mut row = value.borrow_mut();
        ThumbRow::new(Bind::bound(&mut *row), 0.0..=100.0).thumbs(thumbs()).label("Levels").show_at(ui, ROW);
    };
    frames.idle(3, &mut show);
    let tree = frames.tree();
    let mut centres: Vec<Pos2> = tree
        .keys()
        .into_iter()
        .filter(|key| tree.role_of(*key) == Some(craft_ui::contract::Role::SLIDER))
        .filter_map(|key| tree.bounds_of(key))
        .map(|rect| rect.center())
        .collect();
    centres.sort_by(|a, b| a.x.total_cmp(&b.x));
    let from = centres[0];
    if alt {
        frames.hold(Modifiers::ALT);
    }
    frames.frame(vec![moved(from)], &mut show);
    frames.frame(vec![press(from)], &mut show);
    for step in 1..=4 {
        frames.frame(vec![moved(from + Vec2::new(BY * step as f32 / 4.0, 0.0))], &mut show);
    }
    frames.frame(vec![release(from + Vec2::new(BY, 0.0))], &mut show);
    frames.idle(2, &mut show);
    value.into_inner()
}

/// Alt on a split thumb moves one half alone, and that drag has no fine factor: it moves as far as the same drag
/// of a thumb that is not split.
#[trial]
fn an_alt_drag_on_a_split_thumb_moves_one_half_at_full_rate() {
    let free = dragged(|| [Thumb::new("Black"), Thumb::new("Black")], false);
    let split = dragged(|| [Thumb::new("Black").split(), Thumb::new("Black")], true);
    assert_ne!(free[0], Mixed::Same(40.0), "the free thumb moved");
    assert_eq!(split, free, "the half moved as far as a free thumb, and its partner stayed");
}

/// Alt on a thumb that reports it moves at full rate, like the same drag without Alt, and the row reports the
/// thumb and the modifier while the drag runs.
#[trial]
fn an_alt_drag_on_an_alt_reporting_thumb_moves_at_full_rate_and_reports_alt() {
    let free = dragged(|| [Thumb::new("Black").alt_reports(), Thumb::new("White")], false);
    let value = RefCell::new(Mixed::each([40.0, 60.0]));
    let seen = std::cell::Cell::new(None);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let mut row = value.borrow_mut();
        let out = ThumbRow::new(Bind::bound(&mut *row), 0.0..=100.0)
            .thumbs([Thumb::new("Black").alt_reports(), Thumb::new("White")])
            .label("Levels")
            .show_at(ui, ROW);
        if let Some(held) = out.held {
            seen.set(Some((held.thumb, held.mods.alt)));
        }
    };
    frames.idle(3, &mut show);
    let tree = frames.tree();
    let mut centres: Vec<Pos2> = tree
        .keys()
        .into_iter()
        .filter(|key| tree.role_of(*key) == Some(craft_ui::contract::Role::SLIDER))
        .filter_map(|key| tree.bounds_of(key))
        .map(|rect| rect.center())
        .collect();
    centres.sort_by(|a, b| a.x.total_cmp(&b.x));
    let from = centres[0];
    frames.hold(Modifiers::ALT);
    frames.frame(vec![moved(from)], &mut show);
    frames.frame(vec![press(from)], &mut show);
    for step in 1..=4 {
        frames.frame(vec![moved(from + Vec2::new(BY * step as f32 / 4.0, 0.0))], &mut show);
    }
    frames.frame(vec![release(from + Vec2::new(BY, 0.0))], &mut show);
    frames.idle(2, &mut show);
    drop(show);
    assert_eq!(value.into_inner(), free, "Alt is not the fine scale on this thumb");
    assert_eq!(seen.get(), Some((0, true)), "the drag of the first thumb reports Alt");
}

/// The fills of the thumbs' polygons in one frame: each thumb's ring, then its face.
fn thumb_fills(shapes: &[egui::Shape]) -> Vec<egui::Color32> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Path(path) if path.stroke.width == 0.0 => Some(path.fill),
            _ => None,
        })
        .collect()
}

/// With the pointer off the thumbs, the thumb a press would move takes the hover look and the other keeps its own.
#[trial]
fn the_thumb_a_press_would_move_takes_the_hover_look() {
    let mut frames = Frames::new();
    let mut value = Mixed::each([20.0, 80.0]);
    let area = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(300.0, 30.0));
    let mut show = |ui: &mut Ui| {
        ThumbRow::new(Bind::bound(&mut value), 0.0..=100.0).step(1.0).thumbs([Thumb::new("Black"), Thumb::new("White")]).show_at(ui, area);
    };
    frames.idle(3, &mut show);
    let rest = thumb_fills(&frames.frame_shapes(&mut show));
    assert_eq!(rest.len(), 4, "a ring and a face per thumb");
    let near_black = Pos2::new(area.left() + area.width() * 0.35, area.center().y);
    frames.frame(vec![moved(near_black)], &mut show);
    let lit = thumb_fills(&frames.frame_shapes(&mut show));
    assert_ne!(lit[0], rest[0], "the ring of the thumb nearest the pointer is lit");
    assert_eq!(lit[2], rest[2], "the other thumb's ring is not");
}

/// A row of two thumbs at the ends of its range, shown at `ROW`, with the output of its latest pass.
fn row_at_the_ends() -> (Fixture, std::rc::Rc<Cell<Option<(Option<usize>, egui::Rangef)>>>) {
    let seen = std::rc::Rc::new(Cell::new(None));
    let shown = seen.clone();
    let mut value = Mixed::each([0.0, 100.0]);
    let mut fixture = Fixture::new(move |ui| {
        let out = ThumbRow::new(Bind::bound(&mut value), 0.0..=100.0).step(1.0).thumbs([Thumb::new("Black"), Thumb::new("White")]).show_at(ui, ROW);
        shown.set(Some((out.held.map(|held| held.thumb), out.travel)));
    });
    fixture.settle();
    (fixture, seen)
}

/// The row reports the span its thumbs travel along: a thumb at the range minimum is centred on one end and one at
/// the maximum on the other, and the span lies inside the row.
#[trial]
fn the_row_reports_the_span_its_thumbs_travel_along() {
    let (fixture, seen) = row_at_the_ends();
    let (_, travel) = seen.get().expect("the row was shown");
    let mut thumbs = fixture.by_role(craft_ui::contract::Role::SLIDER);
    thumbs.sort_by(|a, b| a.center().x.total_cmp(&b.center().x));
    assert_eq!(thumbs.len(), 2);
    assert!((thumbs[0].center().x - travel.min).abs() < 0.5, "the thumb at the minimum is centred on the span's start");
    assert!((thumbs[1].center().x - travel.max).abs() < 0.5, "the thumb at the maximum is centred on the span's end");
    assert!(ROW.x_range().contains(travel.min) && ROW.x_range().contains(travel.max), "the span lies inside the row");
}

/// A drag that pauses before the release shows the running gesture to the trial, and the gesture ends with the release.
#[trial]
fn a_paused_drag_shows_the_running_gesture_before_the_release() {
    let (mut fixture, seen) = row_at_the_ends();
    let mut thumbs = fixture.by_role(craft_ui::contract::Role::SLIDER);
    thumbs.sort_by(|a, b| a.center().x.total_cmp(&b.center().x));
    let mut mid_drag = None;
    fixture.drag_at_pausing(thumbs[0].center(), Vec2::new(BY, 0.0), |_| {
        mid_drag = seen.get().and_then(|(held, _)| held).map(|held| held == 0);
    });
    assert_eq!(mid_drag, Some(true), "the first thumb is being dragged while the pointer is held");
    assert_eq!(seen.get().and_then(|(held, _)| held), None, "the release ended the drag");
}
