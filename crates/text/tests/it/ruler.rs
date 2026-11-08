//! The text ruler's gestures: placing, moving and clearing stops, the indent markers, mirroring, keys and column
//! borders.

use crate::set;
use craft_behavior::GesturePhase;
use craft_text::ui::DocUnit;
use craft_text::ui::ruler::{ColumnDrag, RulerValues, TabAlign, TabLeader, TabStop, TextRuler};
use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The column's width, points.
const COLUMN: f32 = 468.0;
/// A drag along the strip, as a share of the ruler's width.
const ALONG_SHARE: f32 = 0.1;
/// A drag far enough off the strip to take a stop off it, as a share of the ruler's width.
const OFF_SHARE: f32 = 0.25;

fn left_stop(position: f32) -> TabStop {
    TabStop::new(position, TabAlign::Left, TabLeader::None)
}

/// The stops' nodes in position order: a stop is named by its kind and its position.
fn stop_nodes(fixture: &Fixture) -> Vec<Target> {
    fixture
        .by_role(Role::SLIDER)
        .into_iter()
        .filter(|node| fixture.tree().name_of(node.node).is_some_and(|name| name.starts_with("Left Tab")))
        .collect()
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
    set(RulerValues::default(), |v| {
        v.left_indent = 72.0;
        v.first_line_indent = 36.0;
    })
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
    assert_eq!(shown.fixture.focused_stop().and_then(|(_, part)| part).map(|part| part.index), Some(Some(0)), "the new stop has the focus");
}

#[trial]
fn dragging_a_stop_off_the_ruler_clears_it() {
    let mut shown = show(set(RulerValues::default(), |v| v.stops = vec![left_stop(144.0)]), None, false, &[]);
    shown.fixture.settle();
    let stop = stop_nodes(&shown.fixture).remove(0);
    shown.fixture.drag(&stop, Vec2::Y * (shown.fixture.shown().width() * OFF_SHARE));
    assert!(shown.values.borrow().stops.is_empty());
}

#[trial]
fn indent_markers_move_as_word_does() {
    let mut shown = show(indents(), None, false, &[]);
    shown.fixture.settle();
    let start = shown.values.borrow().clone();
    let first_line = start.left_indent + start.first_line_indent;
    let hanging = shown.fixture.by_label("Hanging Indent").expect("the hanging marker is a node");
    shown.fixture.drag(&hanging, Vec2::X * (shown.fixture.shown().width() * ALONG_SHARE));
    let values = shown.values.borrow().clone();
    assert!(values.left_indent > start.left_indent, "the hanging marker moves the left indent");
    assert!((values.left_indent + values.first_line_indent - first_line).abs() < 1e-3, "and leaves the first line where it was");
    drop(values);
    let left = shown.fixture.by_label("Left Indent").expect("the left indent box is a node");
    let before = shown.values.borrow().clone();
    shown.fixture.drag(&left, Vec2::X * (shown.fixture.shown().width() * ALONG_SHARE));
    let after = shown.values.borrow().clone();
    assert!(after.left_indent > before.left_indent, "the left indent box moves the left indent: {before:?} then {after:?}");
    assert!((after.first_line_indent - before.first_line_indent).abs() < 1e-3, "the box moves first line and left indent together");
}

#[trial]
fn right_to_left_mirrors_positions() {
    let start = set(RulerValues::default(), |v| v.stops = vec![left_stop(144.0)]);
    let mut ltr = show(start.clone(), None, false, &[]);
    let mut rtl = show(start, None, true, &[]);
    for shown in [&mut ltr, &mut rtl] {
        shown.fixture.settle();
        let stop = stop_nodes(&shown.fixture).remove(0);
        shown.fixture.drag(&stop, Vec2::X * (shown.fixture.shown().width() * ALONG_SHARE));
    }
    let (a, b) = (ltr.values.borrow().stops[0].position, rtl.values.borrow().stops[0].position);
    assert!(a > b, "left to right moves a stop up, right to left down: {a} {b}");
}

#[trial]
fn delete_and_backspace_clear_the_picked_stop() {
    for key in [Key::Delete, Key::Backspace] {
        let (kept, picked) = (left_stop(72.0), left_stop(216.0));
        let mut shown = show(set(RulerValues::default(), |v| v.stops = vec![kept, picked]), None, false, &[]);
        shown.fixture.settle();
        let stops = stop_nodes(&shown.fixture);
        assert_eq!(stops.len(), 2, "the stops are named apart");
        shown.fixture.click(&stops[1]);
        shown.fixture.key(key, Modifiers::NONE);
        assert_eq!(shown.values.borrow().stops, vec![kept], "{key:?} clears the picked stop only");
    }
}

#[trial]
fn a_table_column_border_drags_as_one_gesture() {
    static BORDERS: [f32; 3] = [0.0, 156.0, 312.0];
    let mut shown = show(RulerValues::default(), None, false, &BORDERS);
    shown.fixture.settle();
    let border = shown.fixture.by_label("Table Column Width").expect("the border is a node");
    shown.fixture.drag(&border, Vec2::X * (shown.fixture.shown().width() * ALONG_SHARE));
    let phases: Vec<GesturePhase> = shown.columns.borrow().iter().map(|drag| drag.phase).collect();
    assert_eq!(phases.iter().filter(|phase| **phase == GesturePhase::Begin).count(), 1);
    assert_eq!(phases.iter().filter(|phase| **phase == GesturePhase::Commit).count(), 1);
    assert_eq!(phases.last(), Some(&GesturePhase::Commit));
}
