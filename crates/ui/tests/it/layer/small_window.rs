//! Layers inside a small window: a dialog taller than the window keeps its footer in view and scrolls its body, a
//! popup flips or clamps to stay inside the window, and a container closed through its binding closes the popups
//! opened from its body.

use craft_behavior::MotionMode;
use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::popover::Popover;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Pos2, Rect, Vec2};
use std::cell::Cell;
use std::rc::Rc;

const ROOM: Vec2 = Vec2::new(600.0, 400.0);
// Edges snap to whole pixels, so a rectangle may overhang its neighbour by half a point.
const EDGE_SLACK: f32 = 0.5;
/// Far more rows than the window holds.
const ROWS: usize = 30;

fn window() -> Rect {
    Rect::from_min_size(Pos2::ZERO, ROOM).expand(EDGE_SLACK)
}

fn row(at: usize) -> String {
    format!("Row {at}")
}

/// A dialog whose body holds `ROWS` rows, with a footer button "Done".
fn tall_dialog() -> Fixture {
    let mut fixture = Fixture::new(|ui| {
        let mut open = true;
        Dialog::new("Tall").opened(Bind::bound(&mut open)).buttons([FooterButton::new("Done", DialogRole::Accept)]).show(ui, |ui, _| {
            for at in 0..ROWS {
                let _ = Button::new(row(at)).show(ui);
            }
        });
    });
    fixture.room(ROOM).motion(MotionMode::Off);
    fixture.settle();
    fixture
}

/// A dialog with content taller than the window lies inside it with its footer button in view, and its body scrolls
/// to the last row, which then lies above the footer.
#[trial]
fn a_dialog_taller_than_the_window_lies_inside_it_and_scrolls_its_body() {
    let mut fixture = tall_dialog();
    let dialog = fixture.by_role(Role::DIALOG).first().expect("the dialog shows").rect;
    assert!(window().contains_rect(dialog), "the dialog lies inside the window: {dialog:?}");
    let done = fixture.by_label("Done").expect("the footer shows its button").rect;
    assert!(window().contains_rect(done), "the footer button lies inside the window: {done:?} in {dialog:?}");
    let first = fixture.by_label(&row(0)).expect("the first row shows").rect;
    fixture.frame(vec![pointer::moved(first.center())]);
    for _ in 0..ROWS {
        fixture.frame(vec![pointer::wheel(-120.0)]);
    }
    fixture.settle();
    let last = fixture.by_label(&row(ROWS - 1)).expect("the last row is reachable").rect;
    let done = fixture.by_label("Done").expect("the footer still shows its button").rect;
    assert!(last.top() >= dialog.top() - EDGE_SLACK, "the last row scrolled into the dialog: {last:?} in {dialog:?}");
    assert!(last.bottom() <= done.top() + EDGE_SLACK, "the last row lies above the footer: {last:?} over {done:?}");
}

/// A popover open from the start on a button inside `anchor`, holding `rows` rows; returns the fixture and the
/// popover's rect in the last pass.
fn anchored_popover(anchor: Rect, rows: usize) -> (Fixture, Rc<Cell<Rect>>) {
    let seen = Rc::new(Cell::new(Rect::NOTHING));
    let log = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let opener = in_region(ui, anchor, |ui| Button::new("Open").show(ui));
        let mut open = true;
        let shown = Popover::new("Options", Bind::bound(&mut open)).opened_by(&opener).show(ui, |ui, _| {
            for at in 0..rows {
                let _ = Button::new(row(at)).show(ui);
            }
        });
        log.set(shown.output.response.rect);
    });
    fixture.room(ROOM).motion(MotionMode::Off);
    fixture.settle();
    (fixture, seen)
}

/// A popup anchored near the bottom of the window, with too little room below, opens above its anchor.
#[trial]
fn a_popup_anchored_near_the_bottom_opens_above_its_anchor() {
    let (fixture, popup) = anchored_popover(Rect::from_min_size(Pos2::new(20.0, 350.0), Vec2::new(200.0, 30.0)), 5);
    let opener = fixture.by_label("Open").expect("the opener shows").rect;
    let popup = popup.get();
    assert!(fixture.by_label(&row(0)).is_some(), "the popup shows");
    assert!(popup.bottom() <= opener.top() + EDGE_SLACK, "the popup opens above its anchor: {popup:?} over {opener:?}");
    assert!(window().contains_rect(popup), "the popup lies inside the window: {popup:?}");
}

/// A popup with too little room on either side of its anchor lies inside the window.
#[trial]
fn a_popup_with_too_little_room_either_side_lies_inside_the_window() {
    let (fixture, popup) = anchored_popover(Rect::from_min_size(Pos2::new(20.0, 185.0), Vec2::new(200.0, 30.0)), ROWS);
    let popup = popup.get();
    assert!(fixture.by_label(&row(0)).is_some(), "the popup shows");
    assert!(window().contains_rect(popup), "the popup lies inside the window: {popup:?}");
}

/// A container closed through its binding with motion off closes the popover opened from its body: shown again, the
/// container shows no popup, and the popup's binding is false.
#[trial]
fn a_container_closed_through_its_binding_leaves_no_child_popup_open() {
    let (outer, inner) = (Rc::new(Cell::new(true)), Rc::new(Cell::new(true)));
    let (outer_held, inner_held) = (outer.clone(), inner.clone());
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Brush").show(ui);
        let mut outer_open = outer_held.get();
        Popover::new("Brush options", Bind::bound(&mut outer_open)).opened_by(&opener).show(ui, |ui, _| {
            let more = Button::new("More").show(ui);
            let mut inner_open = inner_held.get();
            Popover::new("More options", Bind::bound(&mut inner_open)).opened_by(&more).show(ui, |ui, _| {
                let _ = Button::new("Deep").show(ui);
            });
            inner_held.set(inner_open);
        });
        outer_held.set(outer_open);
    });
    fixture.room(ROOM).motion(MotionMode::Off);
    fixture.settle();
    assert!(fixture.by_label("Deep").is_some(), "the popup inside the container shows");
    outer.set(false);
    fixture.settle();
    assert!(fixture.by_label("More").is_none(), "the container closed");
    outer.set(true);
    fixture.settle();
    assert!(fixture.by_label("More").is_some(), "the container shows again");
    assert!(fixture.by_label("Deep").is_none(), "the popup opened from the container's body stays closed");
    assert!(!inner.get(), "the popup's binding is false");
}
