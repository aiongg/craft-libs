//! `GraphEditor`: the grid and range rules, key and handle drags, the bottom bar, the wheel and the keys.

use craft_behavior::{Selection, SelectionChange};
use craft_timeline::ui::graph::{
    GraphAction, GraphCurve, GraphEditor, GraphEditorOutput, GraphKey, GraphMode, GraphRange, HandleSide, auto_range, nice_step,
};
use craft_timeline::ui::{DragEvent, KeyframeId, KeyframeInterpolation, TimelineView};
use craft_timeline::{FrameRate, Tick, TimeFormat, TimeView};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The editor's room, in points.
const ROOM: Vec2 = Vec2::new(560.0, 320.0);
/// The time area around the editor is taller than the editor: the wheel's zoom share goes to the innermost taker, and
/// equal rects would leave the choice to the order they register in.
const VIEW_ROOM: Vec2 = Vec2::new(ROOM.x, 2.0 * ROOM.y);
/// Points per second of the editor's time axis.
const POINTS_PER_SECOND: f64 = 100.0;
/// The editor's name, which names its AccessKit node.
const GRAPH: &str = "Graph";
/// The curve's name, which starts the names of its keys.
const CURVE: &str = "Opacity";
/// The tooltip words of the bottom bar buttons.
const SPEED: &str = "Edit Speed Graph";
const REFERENCE: &str = "Show Reference Graph";

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

fn format() -> TimeFormat {
    TimeFormat::timecode(FrameRate::new(24, 1))
}

fn key(id: u64, at: f64, value: f64) -> GraphKey {
    GraphKey::new(KeyframeId::new(id), secs(at), value, KeyframeInterpolation::AutoBezier)
}

fn curve(keys: Vec<GraphKey>) -> GraphCurve {
    GraphCurve::new(1, CURVE, keys.iter().map(|k| (k.time, k.value)).collect(), keys)
}

#[trial]
fn nice_steps_are_one_two_five_or_ten() {
    let cases = [(1.4, 1.0), (1.5, 2.0), (3.4, 2.0), (3.5, 5.0), (7.4, 5.0), (7.5, 10.0), (14.0, 10.0), (0.035, 0.05), (-3.6, 5.0)];
    for (raw, step) in cases {
        let got = nice_step(raw);
        assert!((got - step).abs() < step * 1e-9, "{raw}: {got} != {step}");
    }
    assert!(nice_step(0.0) > 0.0, "zero stays positive");
}

#[trial]
fn auto_range_pads_eight_percent() {
    assert_eq!(auto_range(&[], GraphMode::Value), (0.0, 100.0));
    let with_handle = key(1, 1.0, 40.0).handles((secs(0.0), 40.0), (secs(2.0), 60.0));
    let curves = [curve(vec![key(2, 0.0, 10.0), with_handle])];
    let (low, high) = auto_range(&curves, GraphMode::Value);
    assert!((low - (10.0 - 50.0 * 0.08)).abs() < 1e-9 && (high - (60.0 + 50.0 * 0.08)).abs() < 1e-9, "{low}..{high}");

    let speeds = [curve(vec![key(1, 0.0, 20.0), key(2, 1.0, 40.0)])];
    let (low, high) = auto_range(&speeds, GraphMode::Speed);
    assert!((low + 40.0 * 0.08).abs() < 1e-9 && (high - 43.2).abs() < 1e-9, "speed starts at 0: {low}..{high}");

    let flat = [curve(vec![key(1, 0.0, 5.0)])];
    let (low, high) = auto_range(&flat, GraphMode::Value);
    assert!((high - low - 1.0 * 1.16).abs() < 1e-9, "{low}..{high}");
}

/// A shown editor and what it reported.
struct Shown {
    fixture: Fixture,
    range: Rc<RefCell<GraphRange>>,
    selection: Rc<RefCell<Selection<KeyframeId>>>,
    moved: Rc<RefCell<Vec<DragEvent<craft_timeline::ui::GraphKeyMove>>>>,
    handles: Rc<RefCell<Vec<DragEvent<craft_timeline::ui::HandleMove>>>>,
    actions: Rc<RefCell<Vec<GraphAction>>>,
}

fn selected(ids: &[u64]) -> Selection<KeyframeId> {
    let keys: Vec<KeyframeId> = ids.iter().map(|id| KeyframeId::new(*id)).collect();
    Selection::new(keys.clone(), keys.first().copied(), keys.first().copied())
}

/// The editor over `curves` with `selection`, over a fixed value range of 0 to 100.
fn shown(curves: Vec<GraphCurve>, selection: Selection<KeyframeId>, auto_zoom: bool) -> Shown {
    let mut start = GraphRange::default();
    start.auto_zoom = auto_zoom;
    let range = Rc::new(RefCell::new(start));
    let selection = Rc::new(RefCell::new(selection));
    let moved: Rc<RefCell<Vec<_>>> = Rc::default();
    let handles: Rc<RefCell<Vec<_>>> = Rc::default();
    let actions: Rc<RefCell<Vec<GraphAction>>> = Rc::default();
    let (held_range, held_selection, seen_moved, seen_handles, seen_actions) =
        (Rc::clone(&range), Rc::clone(&selection), Rc::clone(&moved), Rc::clone(&handles), Rc::clone(&actions));
    let mut fixture = Fixture::new(move |ui| {
        let mut view = TimeView::new(Tick::ZERO, POINTS_PER_SECOND);
        let mut edited = *held_range.borrow();
        let chosen = held_selection.borrow().clone();
        TimelineView::new("Time", Bind::bound(&mut view), format(), secs(10.0)).show(ui, |ui, _cx| {
            let full = ui.available_rect_before_wrap();
            let rect = Rect::from_min_size(full.min, ROOM);
            let out: GraphEditorOutput = GraphEditor::new(GRAPH, Bind::bound(&mut edited), &curves, &chosen).show_at(ui, rect);
            seen_moved.borrow_mut().extend(out.keys_moved);
            seen_handles.borrow_mut().extend(out.handle_moved);
            seen_actions.borrow_mut().extend(out.actions);
            if let Some(change) = out.selection {
                let mut held = held_selection.borrow_mut();
                match change {
                    SelectionChange::Set(items) => *held = Selection::new(items.clone(), items.first().copied(), items.first().copied()),
                    SelectionChange::Clear => *held = Selection::default(),
                    _ => {}
                }
            }
        });
        *held_range.borrow_mut() = edited;
    });
    fixture.room(VIEW_ROOM);
    fixture.settle();
    Shown { fixture, range, selection, moved, handles, actions }
}

/// Press at `from`, move to `to` in four steps with `modifiers` held, release.
fn drag_between(fixture: &mut Fixture, from: Pos2, to: Pos2, modifiers: Modifiers) {
    fixture.hold(modifiers);
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    for step in 1..=4 {
        fixture.frame(vec![pointer::moved(from + (to - from) * (step as f32 / 4.0))]);
    }
    fixture.frame(vec![pointer::release(to)]);
    fixture.frame(Vec::new());
    fixture.hold(Modifiers::NONE);
}

/// How many points the plot's value range of 0 to 100 spans: the editor's height less its bar.
fn plot_height(fixture: &Fixture) -> f32 {
    let editor = fixture.by_label(GRAPH).expect("the editor has a node").rect;
    let bar = fixture.by_label(SPEED).expect("the bar has a button").rect;
    editor.height() - bar.height()
}

/// The span of the value range the editors are shown over, in value units.
const VALUE_SPAN: f32 = 100.0;
/// How far a drag result may be from the dragged distance, in seconds and in value units.
const TIME_SLACK: f64 = 0.02;
const VALUE_SLACK: f64 = 1.0;
/// The part of a dragged time a Shift-constrained drag must still report.
const MOST_OF_THE_TIME: f64 = 0.8;

/// The pointer travel that moves a point by `seconds` along the time axis and up by `value` units of a plot `plot` points tall.
fn travel(seconds: f64, value: f32, plot: f32) -> Vec2 {
    Vec2::new((seconds * POINTS_PER_SECOND) as f32, -value / VALUE_SPAN * plot)
}

#[trial]
fn dragging_keys_reports_time_and_value() {
    let (seconds, rise) = (0.5, 20.0);
    let mut shown = shown(vec![curve(vec![key(1, 2.0, 50.0)])], Selection::default(), false);
    let tall = plot_height(&shown.fixture);
    let name = format!("{CURVE} 00:00:02:00");
    let at = shown.fixture.by_label(&name).expect("the key has a node").center();
    drag_between(&mut shown.fixture, at, at + travel(seconds, rise, tall), Modifiers::NONE);
    {
        let moved = shown.moved.borrow();
        let Some(DragEvent::Commit(value)) = moved.last().cloned() else { panic!("{moved:?}") };
        let (time_error, value_error) = ((value.delta_time.seconds() - seconds).abs(), (value.delta_value - f64::from(rise)).abs());
        assert!(time_error < TIME_SLACK && value_error < VALUE_SLACK, "{value:?}");
    }
    assert!(shown.selection.borrow().contains(&KeyframeId::new(1)), "grabbing a key selects it");

    let mut constrained = self::shown(vec![curve(vec![key(1, 2.0, 50.0)])], Selection::default(), false);
    let at = constrained.fixture.by_label(&name).expect("the key has a node").center();
    drag_between(&mut constrained.fixture, at, at + travel(seconds, rise / 4.0, tall), Modifiers::SHIFT);
    let moved = constrained.moved.borrow();
    let Some(DragEvent::Commit(value)) = moved.last().cloned() else { panic!("{moved:?}") };
    let keeps_time = value.delta_time.seconds() > seconds * MOST_OF_THE_TIME;
    assert!(keeps_time && value.delta_value == 0.0, "Shift keeps the time axis: {value:?}");
}

#[trial]
fn dragging_a_handle_reports_its_position() {
    let (handle_at, seconds, rise) = (3.0, 0.5, 30.0);
    let with_handle = key(1, 2.0, 50.0).handles((secs(1.0), 50.0), (secs(handle_at), 50.0));
    let mut shown = shown(vec![curve(vec![with_handle])], selected(&[1]), false);
    let tall = plot_height(&shown.fixture);
    let at = shown.fixture.by_label(&format!("{CURVE} 00:00:02:00 outgoing handle")).expect("the handle has a node").center();
    drag_between(&mut shown.fixture, at, at + travel(seconds, rise, tall), Modifiers::ALT);
    let handles = shown.handles.borrow();
    let Some(DragEvent::Commit(value)) = handles.last().cloned() else { panic!("{handles:?}") };
    assert_eq!((value.key, value.side, value.broken), (KeyframeId::new(1), HandleSide::Out, true));
    let (time_error, value_error) = ((value.time.seconds() - (handle_at + seconds)).abs(), (value.value - (50.0 + f64::from(rise))).abs());
    assert!(time_error < TIME_SLACK && value_error < VALUE_SLACK, "{value:?}");
}

#[trial]
fn bottom_bar_buttons_report_actions() {
    let mut shown = shown(vec![curve(vec![key(1, 2.0, 50.0)])], selected(&[1]), false);
    for (label, action) in [
        ("Convert selected keyframes to Hold", GraphAction::Hold),
        ("Convert selected keyframes to Linear", GraphAction::Linear),
        ("Convert selected keyframes to Auto Bezier", GraphAction::AutoBezier),
        ("Easy Ease (F9)", GraphAction::EasyEase),
        ("Easy Ease In (Shift+F9)", GraphAction::EasyEaseIn),
        ("Easy Ease Out (Ctrl+Shift+F9)", GraphAction::EasyEaseOut),
        ("Fit all graphs to view", GraphAction::FitAll),
        ("Fit selection to view", GraphAction::FitSelection),
    ] {
        shown.actions.borrow_mut().clear();
        let button = shown.fixture.by_label(label).expect(label);
        shown.fixture.click(&button);
        assert_eq!(*shown.actions.borrow(), [action], "{label}");
    }
    let speed = shown.fixture.by_label(SPEED).expect("the speed button");
    shown.fixture.click(&speed);
    assert_eq!(shown.range.borrow().mode, GraphMode::Speed);

    shown.actions.borrow_mut().clear();
    let editor = shown.fixture.by_label(GRAPH).expect("the editor has a node");
    shown.fixture.access(AccessActionKind::Focus, &editor);
    shown.fixture.key(Key::F9, Modifiers::NONE);
    shown.fixture.key(Key::F9, Modifiers::SHIFT);
    assert_eq!(*shown.actions.borrow(), [GraphAction::EasyEase, GraphAction::EasyEaseIn]);
}

#[trial]
fn ctrl_wheel_zooms_the_value_range_about_its_middle() {
    let mut shown = shown(vec![curve(vec![key(1, 2.0, 50.0)])], Selection::default(), true);
    let editor = shown.fixture.by_label(GRAPH).expect("the editor has a node").rect;
    shown.fixture.frame(vec![pointer::moved(editor.center())]);
    shown.fixture.frame(vec![pointer::wheel(100.0)]);
    shown.fixture.idle(2);
    let before = *shown.range.borrow();
    assert!(before.auto_zoom, "a plain wheel leaves the range: {before:?}");

    shown.fixture.frame(vec![pointer::zoom_wheel(100.0)]);
    shown.fixture.idle(2);
    let range = *shown.range.borrow();
    assert!(!range.auto_zoom, "zooming ends auto-zoom");
    let (low, high) = auto_range(&[curve(vec![key(1, 2.0, 50.0)])], GraphMode::Value);
    let middle = |low: f64, high: f64| (low + high) / 2.0;
    assert!((middle(range.low, range.high) - middle(low, high)).abs() < 1e-9, "the middle stays: {range:?}");
    assert!(range.high - range.low < high - low, "wheel up zooms in: {range:?}");
}

#[trial]
fn delete_and_backspace_report_delete_and_chords_are_exact() {
    let mut shown = shown(vec![curve(vec![key(1, 2.0, 50.0)])], selected(&[1]), false);
    let editor = shown.fixture.by_label(GRAPH).expect("the editor has a node");
    shown.fixture.access(AccessActionKind::Focus, &editor);
    for (key, modifiers) in [(Key::Delete, Modifiers::SHIFT), (Key::ArrowRight, Modifiers::SHIFT), (Key::ArrowUp, Modifiers::ALT)] {
        shown.fixture.key(key, modifiers);
    }
    assert!(shown.actions.borrow().is_empty() && shown.moved.borrow().is_empty(), "{:?}", shown.actions.borrow());
    shown.fixture.key(Key::Backspace, Modifiers::NONE);
    assert_eq!(*shown.actions.borrow(), [GraphAction::Delete], "Backspace deletes");
    shown.fixture.key(Key::Delete, Modifiers::NONE);
    assert_eq!(*shown.actions.borrow(), [GraphAction::Delete, GraphAction::Delete], "Delete deletes");
}

#[trial]
fn the_reference_button_toggles_the_reference_graph() {
    let mut shown = shown(vec![curve(vec![key(1, 2.0, 50.0)])], Selection::default(), false);
    let button = shown.fixture.by_label(REFERENCE).expect("the reference button");
    shown.fixture.click(&button);
    assert!(shown.range.borrow().reference);
}
