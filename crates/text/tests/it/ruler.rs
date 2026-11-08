//! The text ruler's gestures: placing, moving and clearing stops, the indent markers, mirroring, keys and column
//! borders.

use craft_behavior::GesturePhase;
use craft_text::ui::DocUnit;
use craft_text::ui::ruler::{ColumnDrag, RulerValues, TabAlign, TabLeader, TabStop, TextRuler};
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The column's width, points.
const COLUMN: f32 = 468.0;
/// A drag along the strip, screen points.
const ALONG: f32 = 40.0;
/// A drag far enough off the strip to take a stop off it, screen points.
const OFF: f32 = 80.0;

fn left_stop(position: f32) -> TabStop {
    TabStop { position, align: TabAlign::Left, leader: TabLeader::None }
}

struct Shown {
    fixture: Fixture,
    values: Rc<RefCell<RulerValues>>,
    columns: Rc<RefCell<Vec<ColumnDrag>>>,
}

fn show(start: RulerValues, kind: Option<TabAlign>, right_to_left: bool, borders: &'static [f32]) -> Shown {
    let values = Rc::new(RefCell::new(start));
    let columns = Rc::new(RefCell::new(Vec::new()));
    let (held, seen) = (values.clone(), columns.clone());
    let fixture = Fixture::new(move |ui| {
        let mut shown = held.borrow_mut();
        let mut ruler =
            TextRuler::new("Ruler", Bind::bound(&mut *shown), COLUMN).unit(DocUnit::new(UnitKind::Inches, 96.0)).right_to_left(right_to_left);
        ruler = ruler.columns(borders);
        if let Some(kind) = kind {
            ruler = ruler.kind(kind);
        }
        let out = ruler.show(ui);
        seen.borrow_mut().extend(out.column);
    });
    Shown { fixture, values, columns }
}

fn indents() -> RulerValues {
    RulerValues { stops: Vec::new(), left_indent: 72.0, first_line_indent: 36.0, right_indent: 0.0 }
}

#[trial]
fn click_sets_a_stop_of_the_selectors_kind() {
    let mut shown = show(RulerValues::default(), Some(TabAlign::Center), false, &[]);
    shown.fixture.settle();
    let at = shown.fixture.shown().center();
    shown.fixture.click_at(at);
    let values = shown.values.borrow();
    assert_eq!(values.stops.len(), 1);
    assert_eq!(values.stops[0].align, TabAlign::Center);
}

#[trial]
fn dragging_a_stop_off_the_ruler_clears_it() {
    let mut shown = show(RulerValues { stops: vec![left_stop(144.0)], ..RulerValues::default() }, None, false, &[]);
    shown.fixture.settle();
    let stop = shown.fixture.by_label("Left Tab").expect("the stop is a node");
    shown.fixture.drag(&stop, Vec2::new(0.0, OFF));
    assert!(shown.values.borrow().stops.is_empty());
}

#[trial]
fn indent_markers_move_as_word_does() {
    let mut shown = show(indents(), None, false, &[]);
    shown.fixture.settle();
    let first_line = 72.0 + 36.0;
    let hanging = shown.fixture.by_label("Hanging Indent").expect("the hanging marker is a node");
    shown.fixture.drag(&hanging, Vec2::new(ALONG, 0.0));
    let values = shown.values.borrow().clone();
    assert!(values.left_indent > 72.0, "the hanging marker moves the left indent");
    assert!((values.left_indent + values.first_line_indent - first_line).abs() < 1e-3, "and leaves the first line where it was");
    drop(values);
    let left = shown.fixture.by_label("Left Indent").expect("the left indent box is a node");
    let before = shown.values.borrow().clone();
    shown.fixture.drag(&left, Vec2::new(ALONG, 0.0));
    let after = shown.values.borrow().clone();
    assert!(after.left_indent > before.left_indent);
    assert!((after.first_line_indent - before.first_line_indent).abs() < 1e-3, "the box moves first line and left indent together");
}

#[trial]
fn right_to_left_mirrors_positions() {
    let start = RulerValues { stops: vec![left_stop(144.0)], ..RulerValues::default() };
    let mut ltr = show(start.clone(), None, false, &[]);
    let mut rtl = show(start, None, true, &[]);
    for shown in [&mut ltr, &mut rtl] {
        shown.fixture.settle();
        let stop = shown.fixture.by_label("Left Tab").expect("the stop is a node");
        shown.fixture.drag(&stop, Vec2::new(ALONG, 0.0));
    }
    assert!(ltr.values.borrow().stops[0].position > 144.0);
    assert!(rtl.values.borrow().stops[0].position < 144.0);
}

#[trial]
fn delete_and_backspace_clear_the_picked_stop() {
    for key in [Key::Delete, Key::Backspace] {
        let mut shown = show(RulerValues { stops: vec![left_stop(72.0), left_stop(216.0)], ..RulerValues::default() }, None, false, &[]);
        shown.fixture.settle();
        let stop = shown.fixture.by_label("Left Tab").expect("the stop is a node");
        shown.fixture.click(&stop);
        shown.fixture.key(key, Modifiers::NONE);
        assert_eq!(shown.values.borrow().stops.len(), 1, "{key:?}");
    }
}

#[trial]
fn a_table_column_border_drags_as_one_gesture() {
    static BORDERS: [f32; 3] = [0.0, 156.0, 312.0];
    let mut shown = show(RulerValues::default(), None, false, &BORDERS);
    shown.fixture.settle();
    let border = shown.fixture.by_label("Table Column Width").expect("the border is a node");
    shown.fixture.drag(&border, Vec2::new(ALONG, 0.0));
    let phases: Vec<GesturePhase> = shown.columns.borrow().iter().map(|drag| drag.phase).collect();
    assert_eq!(phases.iter().filter(|phase| **phase == GesturePhase::Begin).count(), 1);
    assert_eq!(phases.iter().filter(|phase| **phase == GesturePhase::Commit).count(), 1);
    assert_eq!(phases.last(), Some(&GesturePhase::Commit));
}
