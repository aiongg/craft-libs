//! The graph editor (egui_kittest).

use craft_ui::behavior::Selection;
use craft_ui::edit::EditEvent;
use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::accesskit::Role;
use egui::{Key, KeyboardShortcut, Modifiers, Pos2, Rect, Ui, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::graph::{
    GraphAction, GraphCurve, GraphEditor, GraphEditorLabels, GraphKey, GraphKeyMove, GraphMode, GraphRange, HandleMove, HandleSide, auto_range,
    nice_step,
};
use super::keyframes::{KeyframeId, KeyframeInterpolation};
use crate::time::{Tick, TimeView};

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

fn key(id: u64, at: f64, value: f64) -> GraphKey {
    GraphKey { id: KeyframeId(id), time: secs(at), value, interpolation: KeyframeInterpolation::AutoBezier, in_handle: None, out_handle: None }
}

fn curve(keys: Vec<GraphKey>) -> GraphCurve {
    GraphCurve { id: 1, name: "Opacity".into(), color: None, samples: keys.iter().map(|k| (k.time, k.value)).collect(), keys }
}

// nice_step cut-offs 1.5, 3.5, 7.5.
#[test]
fn nice_steps_are_one_two_five_or_ten() {
    let cases = [(1.4, 1.0), (1.5, 2.0), (3.4, 2.0), (3.5, 5.0), (7.4, 5.0), (7.5, 10.0), (14.0, 10.0), (0.035, 0.05), (-3.6, 5.0)];
    for (raw, step) in cases {
        let got = nice_step(raw);
        assert!((got - step).abs() < step * 1e-9, "{raw}: {got} != {step}");
    }
    assert!(nice_step(0.0) > 0.0, "zero stays positive");
}

// auto_range of samples, keys and handles, speed starts at 0, flat curves one unit high.
#[test]
fn auto_range_pads_eight_percent() {
    assert_eq!(auto_range(&[], GraphMode::Value), (0.0, 100.0));
    let mut with_handle = key(1, 1.0, 40.0);
    with_handle.out_handle = Some((secs(2.0), 60.0));
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

struct Graph {
    curves: Vec<GraphCurve>,
    selection: Selection<KeyframeId>,
    range: GraphRange,
    moved: Vec<EditEvent<GraphKeyMove>>,
    handles: Vec<EditEvent<HandleMove>>,
    actions: Vec<GraphAction>,
}

const AREA: Rect = Rect { min: pos2(0.0, 0.0), max: pos2(560.0, 320.0) };

fn show_graph(ui: &mut Ui, app: &mut Graph) {
    let view = TimeView::new(Tick::ZERO, 100.0);
    let out = GraphEditor::new("graph", &app.curves, &mut app.selection, &view, &mut app.range).show(ui, AREA);
    app.moved.extend(out.keys_moved);
    app.handles.extend(out.handle_moved);
    app.actions.extend(out.actions);
}

fn graph(curves: Vec<GraphCurve>, selection: Selection<KeyframeId>) -> Harness<'static, Graph> {
    let range = GraphRange { low: 0.0, high: 100.0, auto_zoom: false, ..GraphRange::default() };
    let mut harness = Harness::<Graph>::builder()
        .with_size([560.0, 320.0])
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show_graph, Graph { curves, selection, range, moved: Vec::new(), handles: Vec::new(), actions: Vec::new() });
    harness.run();
    harness
}

/// The point of `(seconds, value)` on the plot.
fn point(seconds: f64, value: f64) -> Pos2 {
    let mut tokens = None;
    egui::__run_test_ui(|ui| tokens = Some(Tokens::of(ui)));
    let tokens = tokens.unwrap();
    let top = AREA.top() + tokens.space.gap.inline;
    let bottom = AREA.bottom() - tokens.size.control.md;
    pos2(AREA.left() + seconds as f32 * 100.0, bottom - (value / 100.0) as f32 * (bottom - top))
}

fn selected(ids: &[u64]) -> Selection<KeyframeId> {
    Selection {
        items: ids.iter().map(|id| KeyframeId(*id)).collect(),
        lead: ids.first().map(|id| KeyframeId(*id)),
        anchor: ids.first().map(|id| KeyframeId(*id)),
    }
}

// a key drag reports GraphKeyMove; Shift keeps one axis.
#[test]
fn dragging_keys_reports_time_and_value() {
    let mut harness = graph(vec![curve(vec![key(1, 2.0, 50.0)])], Selection::new());
    harness.drag(point(2.0, 50.0), point(2.5, 70.0), Modifiers::NONE);
    let Some(EditEvent::Commit { value, changed: true, .. }) = harness.state().moved.last().cloned() else { panic!("{:?}", harness.state().moved) };
    assert!((value.dt.seconds() - 0.5).abs() < 0.02 && (value.dv - 20.0).abs() < 1.0, "{value:?}");
    assert!(harness.state().selection.contains(&KeyframeId(1)), "grabbing a key selects it");

    let mut harness = graph(vec![curve(vec![key(1, 2.0, 50.0)])], Selection::new());
    harness.drag(point(2.0, 50.0), point(2.5, 55.0), Modifiers::SHIFT);
    let Some(EditEvent::Commit { value, .. }) = harness.state().moved.last().cloned() else { panic!("{:?}", harness.state().moved) };
    assert!(value.dt.seconds() > 0.4 && value.dv == 0.0, "Shift keeps the time axis: {value:?}");
}

// a handle drag reports HandleMove; Alt breaks it.
#[test]
fn dragging_a_handle_reports_its_position() {
    let mut with_handle = key(1, 2.0, 50.0);
    with_handle.out_handle = Some((secs(3.0), 50.0));
    let mut harness = graph(vec![curve(vec![with_handle])], selected(&[1]));
    harness.drag(point(3.0, 50.0), point(3.5, 80.0), Modifiers::ALT);
    let Some(EditEvent::Commit { value, .. }) = harness.state().handles.last().cloned() else { panic!("{:?}", harness.state().handles) };
    assert_eq!((value.key, value.side, value.broken), (KeyframeId(1), HandleSide::Out, true));
    assert!((value.time.seconds() - 3.5).abs() < 0.02 && (value.value - 80.0).abs() < 1.0, "{value:?}");
}

// each bar button and F9 report their GraphAction.
#[test]
fn bottom_bar_buttons_report_actions() {
    let mut harness = graph(vec![curve(vec![key(1, 2.0, 50.0)])], selected(&[1]));
    let labels = GraphEditorLabels::default();
    for (label, action) in [
        (labels.hold.as_str(), GraphAction::Hold),
        (labels.linear.as_str(), GraphAction::Linear),
        (labels.auto_bezier.as_str(), GraphAction::AutoBezier),
        (labels.easy_ease.as_str(), GraphAction::EasyEase),
        (labels.easy_ease_in.as_str(), GraphAction::EasyEaseIn),
        (labels.easy_ease_out.as_str(), GraphAction::EasyEaseOut),
        (labels.fit_all.as_str(), GraphAction::FitAll),
        (labels.fit_selection.as_str(), GraphAction::FitSelection),
    ] {
        harness.state_mut().actions.clear();
        harness.get_by_label(label).click();
        harness.run();
        assert_eq!(harness.state().actions, [action], "{label}");
    }
    harness.get_by_label(&labels.speed_graph).click();
    harness.run();
    assert_eq!(harness.state().range.mode, GraphMode::Speed);

    harness.state_mut().actions.clear();
    harness.get_by_role_and_label(Role::Group, &labels.title).focus();
    harness.run();
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::F9));
    harness.press(KeyboardShortcut::new(Modifiers::SHIFT, Key::F9));
    assert_eq!(harness.state().actions, [GraphAction::EasyEase, GraphAction::EasyEaseIn]);
}

// Ctrl/Cmd+wheel reaches the editor as egui's zoom delta: the range scales about its middle, a
// plain wheel leaves it, and auto-zoom ends.
#[test]
fn ctrl_wheel_zooms_the_value_range_about_its_middle() {
    let wheel = |modifiers| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 100.0),
        phase: egui::TouchPhase::Move,
        modifiers,
        source: egui::MouseWheelSource::Unknown,
    };
    let mut harness = graph(vec![curve(vec![key(1, 2.0, 50.0)])], Selection::new());
    harness.state_mut().range = GraphRange { auto_zoom: true, ..GraphRange::default() };
    harness.hover_at(point(1.0, 50.0));
    harness.event(wheel(Modifiers::NONE));
    harness.run();
    let before = harness.state().range;
    assert!(before.auto_zoom, "a plain wheel leaves the range: {before:?}");

    harness.event(wheel(Modifiers::COMMAND));
    harness.run();
    let range = harness.state().range;
    assert!(!range.auto_zoom, "zooming ends auto-zoom");
    assert!(((range.low + range.high) / 2.0 - (before.low + before.high) / 2.0).abs() < 1e-9, "the middle stays: {range:?}");
    assert!(range.high - range.low < before.high - before.low, "wheel up zooms in: {range:?}");
}

// Backspace deletes as Delete does; an extra Shift or Alt makes another chord, which neither
// deletes nor nudges.
#[test]
fn delete_and_backspace_report_delete_and_chords_are_exact() {
    let mut harness = graph(vec![curve(vec![key(1, 2.0, 50.0)])], selected(&[1]));
    let title = GraphEditorLabels::default().title;
    harness.get_by_role_and_label(Role::Group, &title).focus();
    harness.run();
    for shortcut in [
        KeyboardShortcut::new(Modifiers::SHIFT, Key::Delete),
        KeyboardShortcut::new(Modifiers::SHIFT, Key::ArrowRight),
        KeyboardShortcut::new(Modifiers::ALT, Key::ArrowUp),
    ] {
        harness.press(shortcut);
    }
    assert!(harness.state().actions.is_empty() && harness.state().moved.is_empty(), "{:?}", harness.state().actions);
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Backspace));
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Delete));
    assert_eq!(harness.state().actions, [GraphAction::Delete, GraphAction::Delete]);
}

/// How many Tab presses bring the focus from the plot back to it.
fn tab_stops(harness: &mut Harness<'static, Graph>) -> usize {
    let title = GraphEditorLabels::default().title;
    harness.get_by_role_and_label(Role::Group, &title).focus();
    harness.run();
    for presses in 1..=60 {
        harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Tab));
        if harness.get_by_role_and_label(Role::Group, &title).is_focused() {
            return presses;
        }
    }
    panic!("the focus never came back to the plot");
}

// The plot is the graph's one Tab stop: keys and handles add none.
#[test]
fn keys_and_handles_are_not_tab_stops() {
    let mut with_handle = key(1, 2.0, 50.0);
    with_handle.out_handle = Some((secs(3.0), 50.0));
    let mut bare = graph(Vec::new(), Selection::new());
    let mut busy = graph(vec![curve(vec![with_handle, key(2, 1.0, 20.0), key(3, 4.0, 80.0)])], selected(&[1]));
    assert_eq!(tab_stops(&mut busy), tab_stops(&mut bare));
}

// The bar's reference toggle flips `GraphRange::reference`.
#[test]
fn the_reference_button_toggles_the_reference_graph() {
    let mut harness = graph(vec![curve(vec![key(1, 2.0, 50.0)])], Selection::new());
    let label = GraphEditorLabels::default().show_reference;
    harness.get_by_label(&label).click();
    harness.run();
    assert!(harness.state().range.reference);
}
