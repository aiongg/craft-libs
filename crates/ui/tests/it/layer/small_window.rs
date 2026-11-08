//! Layers inside a small window: a dialog taller than the window keeps its footer in view and scrolls its body, a
//! popup flips or clamps to stay inside the window, and a container closed through its binding closes the popups
//! opened from its body.

use craft_behavior::{DisplayOrder, ItemSource, MotionMode};
use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Props as _, Role, Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::list::{List, ListRow};
use craft_ui::popover::Popover;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Pos2, Rect, Vec2};
use std::cell::Cell;
use std::rc::Rc;

/// The window: the fixture's screen is at least 800 by 600 points.
const ROOM: Vec2 = Vec2::new(800.0, 600.0);
// Edges snap to whole pixels, so a rectangle may overhang its neighbour by half a point.
const EDGE_SLACK: f32 = 0.5;
/// Far more rows than the window holds.
const ROWS: usize = 40;

/// `ROWS` rows keyed by position.
struct Rows;

impl DisplayOrder<usize> for Rows {
    fn len(&self) -> usize {
        ROWS
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for Rows {}

fn window() -> Rect {
    Rect::from_min_size(Pos2::ZERO, ROOM).expand(EDGE_SLACK)
}

fn row(at: usize) -> String {
    format!("Row {at}")
}

/// A dialog whose body holds `ROWS` rows, with a footer button "Done": a stack of buttons, or with `list` one list
/// that is the dialog's collection body.
fn tall_dialog(list: bool) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        let dialog = Dialog::new("Tall").opened(Bind::bound(&mut open)).buttons([FooterButton::new("Done", DialogRole::Accept)]);
        let dialog = if list { dialog.collection_body() } else { dialog };
        dialog.show(ui, |ui, _| {
            if list {
                let _ = List::new("Items").show_items(ui, &Rows, &mut |key, _| ListRow::new(row(key)));
            } else {
                for at in 0..ROWS {
                    let _ = Button::new(row(at)).show(ui);
                }
            }
        });
    });
    fixture.room(ROOM).motion(MotionMode::Off);
    fixture.settle();
    fixture
}

/// A dialog with content taller than the window lies inside it with its footer button in view, and its body scrolls
/// to the last row, which then lies above the footer.
fn fits_and_scrolls(list: bool) {
    let mut fixture = tall_dialog(list);
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

#[trial]
fn a_dialog_taller_than_the_window_lies_inside_it_and_scrolls_its_body() {
    fits_and_scrolls(false);
}

/// The same with a body that is one collection, which owns the scroll (the shortcuts dialog).
#[trial]
fn a_dialog_whose_collection_is_taller_than_the_window_lies_inside_it_and_scrolls_it() {
    fits_and_scrolls(true);
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
    let (fixture, popup) = anchored_popover(Rect::from_min_size(Pos2::new(20.0, 550.0), Vec2::new(200.0, 30.0)), 5);
    let opener = fixture.by_label("Open").expect("the opener shows").rect;
    let popup = popup.get();
    assert!(fixture.by_label(&row(0)).is_some(), "the popup shows");
    assert!(popup.bottom() <= opener.top() + EDGE_SLACK, "the popup opens above its anchor: {popup:?} over {opener:?}");
    assert!(window().contains_rect(popup), "the popup lies inside the window: {popup:?}");
}

/// A popup with too little room on either side of its anchor lies inside the window.
#[trial]
fn a_popup_with_too_little_room_either_side_lies_inside_the_window() {
    let (fixture, popup) = anchored_popover(Rect::from_min_size(Pos2::new(20.0, 285.0), Vec2::new(200.0, 30.0)), ROWS);
    let popup = popup.get();
    assert!(fixture.by_label(&row(0)).is_some(), "the popup shows");
    assert!(window().contains_rect(popup), "the popup lies inside the window: {popup:?}");
}

/// A container closed through its binding with motion off closes the popover opened from its body: shown again, the
/// container shows no popup, and the popup's binding is false.
#[trial]
fn a_container_closed_through_its_binding_leaves_no_child_popup_open() {
    let (outer, inner) = (Rc::new(Cell::new(true)), Rc::new(Cell::new(false)));
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
    inner.set(true);
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
