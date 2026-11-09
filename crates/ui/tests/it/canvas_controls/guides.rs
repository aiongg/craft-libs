//! `Guides`: keyboard order, nudges, drags, deletion and the ruler hand-off.

use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_behavior::{Axis, CanvasView, EditEvent, GesturePhase, RulerUnits, Ticks};
use craft_keymap::Mods;
use craft_ui::canvas::{Canvas, Guide, GuideDrag, GuideKey, RulerText, Snap};
use craft_ui::canvas_controls::{GuideCommands, Guides};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, Target, pointer, trial};
use egui::{Id, Key, Modifiers, Pos2, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The part key the tests address guides by.
const GUIDE: craft_behavior::PartKey = craft_behavior::PartKey::new("guide");

/// How far a drag moves a guide in the drag tests, in document units.
const SHIFT: f64 = 20.0;

/// The grid the canvas's snap rounds to in the snap test, in document units.
const SNAP_STEP: f64 = 25.0;

/// A distance no canvas reaches.
const FAR: f32 = 5000.0;

/// What one pass reported about the edit.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Seen {
    Begin,
    Update,
    Commit,
    Cancel,
    Set,
}

struct Harness {
    fixture: Fixture,
    id: Rc<Cell<Id>>,
    guides: Rc<RefCell<Vec<Guide>>>,
    selected: Rc<Cell<Option<GuideKey>>>,
    seen: Rc<RefCell<Vec<Seen>>>,
    pending: Rc<Cell<Option<GuideDrag>>>,
    bounds: Rc<Cell<Option<DocRect>>>,
    tool_click: Rc<Cell<Option<Pos2>>>,
    units: Rc<Cell<Option<RulerText>>>,
    snapping: Rc<Cell<bool>>,
}

fn key(id: u64) -> GuideKey {
    GuideKey::new(id)
}

fn vertical(id: u64, x: f64) -> Guide {
    Guide::new(key(id), Axis::Y, x)
}

fn horizontal(id: u64, y: f64) -> Guide {
    Guide::new(key(id), Axis::X, y)
}

fn harness(guides: Vec<Guide>) -> Harness {
    let id = Rc::new(Cell::new(Id::NULL));
    let guides = Rc::new(RefCell::new(guides));
    let selected = Rc::new(Cell::new(None));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let pending = Rc::new(Cell::new(None));
    let bounds = Rc::new(Cell::new(None));
    let tool_click = Rc::new(Cell::new(None));
    let units = Rc::new(Cell::new(None));
    let snapping = Rc::new(Cell::new(false));
    let (shown_id, shown_guides, shown_selected, shown_seen, shown_pending, shown_bounds, shown_click) =
        (Rc::clone(&id), Rc::clone(&guides), Rc::clone(&selected), Rc::clone(&seen), Rc::clone(&pending), Rc::clone(&bounds), Rc::clone(&tool_click));
    let (shown_units, shown_snapping) = (Rc::clone(&units), Rc::clone(&snapping));
    let mut fixture = Fixture::new(move |ui| {
        let room = ui.available_rect_before_wrap();
        let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
        let mut canvas = Canvas::new("Document").view(view);
        if let Some(text) = shown_units.get() {
            canvas = canvas.rulers(text, text, Point::ZERO);
        }
        if shown_snapping.get() {
            canvas = canvas.snap(Snap::new(|_, position, _| (position / SNAP_STEP).round() * SNAP_STEP));
        }
        let canvas = canvas.show(ui, |ui, body| {
            let mut held = shown_guides.borrow_mut();
            let mut control = Guides::new("Guides", Bind::bound(&mut *held)).interactive(true);
            if let Some(drag) = shown_pending.take() {
                control = control.pending(drag);
            }
            if let Some(selected) = shown_selected.get() {
                control = control.selected_guide(selected);
            }
            if let Some(bounds) = shown_bounds.get() {
                control = control.bounds(bounds);
            }
            control.show_at(ui, body.rect)
        });
        if let Some(output) = canvas.inner {
            shown_id.set(output.edit.response.id);
            if let Some(select) = output.select {
                shown_selected.set(select);
            }
            let phase = output.edit.event.as_ref().map(|event| match event {
                EditEvent::Begin { .. } => Seen::Begin,
                EditEvent::Update { .. } => Seen::Update,
                EditEvent::Commit { .. } => Seen::Commit,
                EditEvent::Cancel { .. } => Seen::Cancel,
                _ => Seen::Set,
            });
            shown_seen.borrow_mut().extend(phase);
        }
        if let Some(at) = canvas.output.click_at() {
            shown_click.set(Some(at));
        }
    });
    fixture.settle();
    Harness { fixture, id, guides, selected, seen, pending, bounds, tool_click, units, snapping }
}

impl Harness {
    fn guide(&self, id: u64) -> Target {
        self.fixture.part(self.id.get(), key(id).part(GUIDE)).expect("the guide has a part")
    }

    fn click(&mut self, id: u64) {
        let target = self.guide(id);
        self.fixture.click(&target);
    }

    fn position(&self, id: u64) -> Option<f64> {
        self.guides.borrow().iter().find(|guide| guide.key == key(id)).map(|guide| guide.position)
    }

    fn count(&self, phase: Seen) -> usize {
        self.seen.borrow().iter().filter(|seen| **seen == phase).count()
    }

    fn press(&mut self, id: u64, key: Key, mods: Modifiers) {
        self.click(id);
        self.fixture.key(key, mods);
    }
}

fn mixed() -> Vec<Guide> {
    let mut locked = vertical(13, 10.0);
    locked.locked = true;
    vec![horizontal(10, 37.0), vertical(11, 100.0), vertical(12, 30.0), locked]
}

#[trial]
fn page_keys_select_vertical_guides_left_to_right_then_horizontal_ones_and_skip_locked_guides() {
    let mut harness = harness(mixed());
    harness.click(12);
    assert_eq!(harness.selected.get(), Some(key(12)), "a press selects the guide");
    let mut order = Vec::new();
    for _ in 0..3 {
        harness.fixture.key(Key::PageDown, Modifiers::NONE);
        order.push(harness.selected.get());
    }
    assert_eq!(order, [Some(key(11)), Some(key(10)), Some(key(10))], "Page Down walks the keyboard order and stops at its end");
    harness.fixture.key(Key::PageUp, Modifiers::NONE);
    assert_eq!(harness.selected.get(), Some(key(11)), "Page Up goes back");
    assert!(harness.fixture.part(harness.id.get(), key(13).part(GUIDE)).is_none(), "a locked guide has no part");
}

#[trial]
fn arrows_nudge_the_selected_guide_across_its_axis_and_shift_takes_the_large_step() {
    let mut harness = harness(mixed());
    harness.press(11, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(harness.position(11), Some(101.0), "an arrow moves a guide one unit");
    harness.fixture.key(Key::ArrowRight, Modifiers::SHIFT);
    assert_eq!(harness.position(11), Some(111.0), "Shift moves it ten");
    harness.fixture.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(harness.position(11), Some(111.0), "an arrow along the guide moves nothing");
    assert_eq!(harness.position(12), Some(30.0), "the other guides stay");
}

#[trial]
fn a_nudge_stops_at_the_bounds() {
    let mut harness = harness(mixed());
    harness.bounds.set(Some(DocRect::new(0.0, 0.0, 105.0, 200.0)));
    harness.fixture.settle();
    harness.press(11, Key::ArrowRight, Modifiers::SHIFT);
    harness.fixture.key(Key::ArrowRight, Modifiers::SHIFT);
    assert_eq!(harness.position(11), Some(105.0), "a guide stays inside the bounds");
}

#[trial]
fn delete_removes_the_focused_guide_and_selects_the_one_after_it() {
    let mut harness = harness(mixed());
    harness.press(12, Key::Delete, Modifiers::NONE);
    assert_eq!(harness.position(12), None, "Delete removes the focused guide");
    assert_eq!(harness.selected.get(), Some(key(11)), "the next guide in keyboard order is selected");
    harness.fixture.key(Key::Backspace, Modifiers::NONE);
    assert_eq!(harness.position(11), None, "Backspace removes it too");
    assert_eq!(harness.selected.get(), Some(key(10)));
}

#[trial]
fn a_drag_moves_the_guide_in_one_gesture() {
    let mut harness = harness(mixed());
    let target = harness.guide(11);
    harness.fixture.drag(&target, Vec2::new(SHIFT as f32, 0.0));
    assert_eq!(harness.position(11), Some(100.0 + SHIFT), "the guide follows the pointer");
    assert_eq!(harness.count(Seen::Commit), 1, "one drag commits once: {:?}", harness.seen.borrow());
    assert_eq!(harness.count(Seen::Cancel), 0);
}

#[trial]
fn a_release_outside_the_canvas_removes_the_guide() {
    let mut harness = harness(mixed());
    let target = harness.guide(11);
    harness.fixture.drag(&target, Vec2::new(FAR, 0.0));
    assert_eq!(harness.position(11), None, "a guide released outside is removed");
    assert_eq!(harness.count(Seen::Commit), 1);
}

#[trial]
fn a_release_outside_the_bounds_removes_the_guide() {
    let mut harness = harness(mixed());
    harness.bounds.set(Some(DocRect::new(0.0, 0.0, 110.0, 200.0)));
    harness.fixture.settle();
    let target = harness.guide(11);
    harness.fixture.drag(&target, Vec2::new(SHIFT as f32 * 2.0, 0.0));
    assert_eq!(harness.position(11), None, "a guide dropped past the document edge is removed");
}

#[trial]
fn escape_cancels_a_drag_once_and_restores_the_guide() {
    let mut harness = harness(mixed());
    let target = harness.guide(11);
    let from = target.center();
    harness.fixture.frame(vec![pointer::moved(from)]);
    harness.fixture.frame(vec![pointer::press(from)]);
    harness.fixture.frame(vec![pointer::moved(from + Vec2::new(8.0, 0.0))]);
    harness.fixture.frame(vec![pointer::moved(from + Vec2::new(16.0, 0.0))]);
    harness.fixture.key(Key::Escape, Modifiers::NONE);
    harness.fixture.frame(vec![pointer::moved(from + Vec2::new(24.0, 0.0))]);
    harness.fixture.frame(vec![pointer::release(from + Vec2::new(24.0, 0.0))]);
    harness.fixture.frame(Vec::new());
    assert_eq!(harness.count(Seen::Cancel), 1, "Escape cancels once: {:?}", harness.seen.borrow());
    assert_eq!(harness.count(Seen::Commit), 0, "the release after Escape commits nothing");
    assert_eq!(harness.position(11), Some(100.0), "the guide is where it was");
}

#[trial]
fn a_press_away_from_the_guides_reaches_the_tool_and_a_press_on_one_does_not() {
    let mut harness = harness(mixed());
    harness.click(11);
    assert_eq!(harness.tool_click.get(), None, "a click on a guide is the guides'");
    let canvas = harness.fixture.by_role(craft_ui::contract::Role::CANVAS).into_iter().next().expect("the canvas has a node");
    let away = Pos2::new(canvas.rect.left() + 150.0, canvas.rect.top() + 120.0);
    harness.fixture.click_at(away);
    let at = harness.tool_click.get().expect("a click away from the guides reaches the tool");
    assert!(canvas.rect.contains(at));
}

fn dragged(phase: GesturePhase, at: Option<f64>) -> GuideDrag {
    GuideDrag::new_guide(Axis::Y, phase, at, Mods::NONE)
}

#[trial]
fn a_guide_dragged_out_of_a_ruler_is_added_on_release_over_the_canvas() {
    let mut harness = harness(mixed());
    harness.pending.set(Some(dragged(GesturePhase::Begin, Some(60.0))));
    harness.fixture.frame(Vec::new());
    harness.pending.set(Some(dragged(GesturePhase::Update, Some(70.0))));
    harness.fixture.frame(Vec::new());
    assert_eq!(harness.guides.borrow().len(), 5, "the new guide shows while it is dragged");
    harness.pending.set(Some(dragged(GesturePhase::Commit, Some(80.0))));
    harness.fixture.frame(Vec::new());
    harness.pending.set(None);
    harness.fixture.frame(Vec::new());
    let guides = harness.guides.borrow();
    assert_eq!(guides.len(), 5, "one guide was added");
    assert_eq!(guides.iter().filter(|guide| guide.position == 80.0 && guide.along == Axis::Y).count(), 1, "at the release position");
}

#[trial]
fn a_guide_released_over_a_ruler_is_not_added() {
    let mut harness = harness(mixed());
    harness.pending.set(Some(dragged(GesturePhase::Begin, Some(60.0))));
    harness.fixture.frame(Vec::new());
    harness.pending.set(Some(dragged(GesturePhase::Commit, None)));
    harness.fixture.frame(Vec::new());
    harness.pending.set(None);
    harness.fixture.frame(Vec::new());
    assert_eq!(harness.guides.borrow().len(), 4, "a guide dropped on a ruler is discarded");
}

#[trial]
fn the_guide_commands_edit_the_guides() {
    let mut guides = mixed();
    assert_eq!(GuideCommands::Delete.apply(&mut guides, Some(key(11))), Some(None), "Delete clears the selection");
    assert!(guides.iter().all(|guide| guide.key != key(11)));
    assert_eq!(GuideCommands::Delete.apply(&mut guides, Some(key(13))), None, "a locked guide is not deleted");
    GuideCommands::Lock.apply(&mut guides, None);
    assert!(guides.iter().all(|guide| guide.locked));
    GuideCommands::Unlock.apply(&mut guides, None);
    assert!(guides.iter().all(|guide| !guide.locked));
    assert_eq!(GuideCommands::Clear.apply(&mut guides, None), Some(None));
    assert!(guides.is_empty());
}

#[trial]
fn a_press_on_the_canvas_background_clears_the_selection_without_app_code() {
    let mut harness = harness(mixed());
    harness.click(11);
    assert_eq!(harness.selected.get(), Some(key(11)));
    let canvas = harness.fixture.by_role(craft_ui::contract::Role::CANVAS).into_iter().next().expect("the canvas has a node");
    harness.fixture.click_at(Pos2::new(canvas.rect.left() + 150.0, canvas.rect.top() + 120.0));
    assert_eq!(harness.selected.get(), None, "a press that reaches the tool deselects the guide");
}

#[trial]
fn guides_read_the_canvas_units_with_no_unit_props() {
    let inches = RulerText::new(RulerUnits::new(1.0 / 96.0, Ticks::decimal(false)), UnitKind::Inches);
    let mut harness = harness(mixed());
    harness.units.set(Some(inches));
    harness.fixture.settle();
    let from = harness.guide(11).center();
    harness.fixture.frame(vec![pointer::moved(from)]);
    harness.fixture.frame(vec![pointer::press(from)]);
    harness.fixture.frame(vec![pointer::moved(from + Vec2::new(8.0, 0.0))]);
    harness.fixture.frame(vec![pointer::moved(from + Vec2::new(SHIFT as f32, 0.0))]);
    let texts: Vec<String> = harness.fixture.texts().iter().map(|text| text.text.to_string()).collect();
    assert!(texts.iter().any(|text| text.contains("1.25")), "the readout of a guide at 120 pixels is in the canvas's inches: {texts:?}");
    assert!(texts.iter().all(|text| !text.contains("px")), "no readout falls back to pixels: {texts:?}");
}

#[trial]
fn a_drag_snaps_the_guide_through_the_canvas_snap() {
    let mut harness = harness(mixed());
    harness.snapping.set(true);
    harness.fixture.settle();
    let target = harness.guide(11);
    harness.fixture.drag(&target, Vec2::new(SHIFT as f32 + 3.0, 0.0));
    assert_eq!(harness.position(11), Some(125.0), "the guide lands on the grid the canvas snaps to, not where the pointer is");
}
