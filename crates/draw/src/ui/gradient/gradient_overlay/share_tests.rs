//! The overlay and a `GradientEditor` on one host gradient through one `GestureHost`.

use egui::accesskit::{Action, ActionRequest, Role};
use egui::{Context, Event, Id, Modifiers, Pos2, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::Point;

use super::GradientOverlay;
use super::story::english_names;
use super::tests::{History, button, gradient};
use crate::ui::gradient::{GradientEditor, GradientStops, StopRef};
use crate::ui::gradient::{GradientLineNames, LineGradient, LinePart};
use craft_ui::canvas::CanvasView;
use craft_ui::edit::{EditEvent, GestureHost};
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

/// The host: one gradient, one gesture host, and the events each control reported.
struct Shared {
    doc: History,
    host: GestureHost<History>,
    view: Option<CanvasView>,
    overlay: Option<Id>,
    editor: Option<Id>,
    /// The bar's response id.
    bar: Option<Id>,
    picked: Option<LinePart>,
    /// The control and kind of each event that reached the host, updates left out.
    log: Vec<(&'static str, &'static str)>,
}

fn kind<T>(event: &EditEvent<T>) -> Option<&'static str> {
    match event {
        EditEvent::Begin { .. } => Some("begin"),
        EditEvent::Update { .. } => None,
        EditEvent::Commit { .. } => Some("commit"),
        EditEvent::Cancel { .. } => Some("cancel"),
        EditEvent::Set { .. } => Some("set"),
        _ => None,
    }
}

fn harness() -> Harness<'static, Shared> {
    let state = Shared {
        doc: History { now: gradient(), undo: Vec::new() },
        host: GestureHost::new(),
        view: None,
        overlay: None,
        editor: None,
        bar: None,
        picked: None,
        log: Vec::new(),
    };
    let mut h = Harness::<Shared>::builder()
        .with_size([480.0, 560.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, s: &mut Shared| {
                let ctx = ui.ctx().clone();
                let numbers = LocaleNumbers::from_tag("en");
                let editor = GradientEditor::id_in(ui, "bar");
                let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
                let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
                let value = s.doc.now.clone();
                let names = GradientLineNames { group: "Canvas gradient".into(), ..english_names() };
                let r = GradientOverlay::new("gradient", &value.line, &value.stops, names).selection_of(editor).show(ui, &canvas, &view);
                if let Some(event) = r.event {
                    s.log.extend(kind(&event).map(|k| ("canvas", k)));
                    s.host.handle(&ctx, &mut s.doc, event, History::apply);
                }
                // The editor draws what the overlay's update left, in the same frame.
                let stops = s.doc.now.stops.clone();
                let response = GradientEditor::new("bar", "Gradient", &stops, &numbers).show(ui);
                if let Some(event) = response.edit.event {
                    s.log.extend(kind(&event).map(|k| ("bar", k)));
                    s.host.handle(&ctx, &mut s.doc, event, |doc: &mut History, stops: GradientStops| {
                        let line = doc.now.line;
                        doc.apply(LineGradient { line, stops });
                    });
                }
                (s.view, s.overlay, s.editor, s.picked) = (Some(view), Some(r.id), Some(editor), r.picked);
                s.bar = Some(response.edit.response.id);
            },
            state,
        );
    h.run();
    h
}

fn at(h: &Harness<'_, Shared>, t: f64) -> Pos2 {
    h.state().view.expect("drawn").to_screen(Point::new(100.0 + 200.0 * t, 150.0))
}

fn locations(h: &Harness<'_, Shared>) -> Vec<f32> {
    h.state().doc.now.stops.color.iter().map(|s| s.location).collect()
}

fn editor_selected(h: &Harness<'_, Shared>) -> Option<StopRef> {
    GradientEditor::selected(&h.ctx, h.state().editor.expect("drawn"))
}

fn press(h: &mut Harness<'_, Shared>, t: f64) {
    let to = at(h, t);
    h.event(Event::PointerMoved(to));
    h.event(button(to, true, Modifiers::NONE));
    h.run();
}

fn move_to(h: &mut Harness<'_, Shared>, t: f64) {
    let to = at(h, t);
    h.event(Event::PointerMoved(to));
    h.run();
}

fn release(h: &mut Harness<'_, Shared>, t: f64) {
    let to = at(h, t);
    h.event(Event::PointerMoved(to));
    h.event(button(to, false, Modifiers::NONE));
    h.run();
}

fn click(h: &mut Harness<'_, Shared>, t: f64) {
    press(h, t);
    release(h, t);
}

fn click_at(h: &mut Harness<'_, Shared>, to: Pos2) {
    h.event(Event::PointerMoved(to));
    h.event(button(to, true, Modifiers::NONE));
    h.event(button(to, false, Modifiers::NONE));
    h.run();
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

/// The colour stops and midpoints under the group named `group`: name, value, `selected`.
fn markers(h: &Harness<'_, Shared>, group: &str) -> Vec<(String, String, bool)> {
    descendants(h.get_by_label(group).accesskit_node())
        .into_iter()
        .filter(|n| n.role() == Role::GraphicsObject)
        .map(|n| (n.label().unwrap_or_default(), n.value().unwrap_or_default(), n.data().is_selected() == Some(true)))
        .filter(|(name, ..)| name.starts_with("Color stop") || name.starts_with("Midpoint"))
        .collect()
}

#[test]
fn a_canvas_drag_moves_the_bars_marker_in_the_same_frame_and_makes_one_undo_step() {
    let mut h = harness();
    press(&mut h, 0.3);
    move_to(&mut h, 0.5);
    let bar = markers(&h, "Gradient");
    assert_eq!((bar[2].0.as_str(), bar[2].1.as_str()), ("Color stop 2 of 4", "50%, #FFFF00"), "the bar follows the drag");
    move_to(&mut h, 0.45);
    release(&mut h, 0.45);
    assert_eq!(locations(&h), [0.0, 0.45, 0.6, 1.0]);
    assert_eq!(h.state().doc.undo.len(), 1, "one undo step");
    assert_eq!(h.state().log, [("canvas", "begin"), ("canvas", "commit")]);
}

#[test]
fn a_stop_picked_on_the_canvas_is_the_editors_selection_and_the_other_way_round() {
    let mut h = harness();
    assert_eq!(h.state().editor, h.state().bar, "`id_in` names the bar the selection is kept under");
    click(&mut h, 0.6);
    assert_eq!(editor_selected(&h), Some(StopRef::Color(2)));
    assert_eq!(GradientOverlay::selected(&h.ctx, h.state().overlay.expect("drawn")), None, "the overlay keeps none of its own");
    let location = h.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Location"));
    assert_eq!(location.value().as_deref(), Some("60%"), "the field under the bar is the picked stop's");

    GradientEditor::select(&h.ctx, h.state().editor.expect("drawn"), Some(StopRef::Color(3)));
    h.run();
    assert_eq!(h.state().picked, Some(LinePart::Stop(3)), "the keys act on the stop the panel selected");
    let canvas = markers(&h, "Canvas gradient");
    assert_eq!(canvas.iter().filter(|m| m.2).map(|m| m.0.as_str()).collect::<Vec<_>>(), ["Color stop 4 of 4"]);

    // A selection the canvas doesn't show shows nothing there; a press on a handle keeps it.
    GradientEditor::select(&h.ctx, h.state().editor.expect("drawn"), Some(StopRef::Opacity(0)));
    h.run();
    assert!(markers(&h, "Canvas gradient").iter().all(|m| !m.2));
    click(&mut h, 1.0);
    assert_eq!(h.state().picked, Some(LinePart::End));
    assert_eq!(editor_selected(&h), Some(StopRef::Opacity(0)));
    click(&mut h, 0.3);
    assert_eq!(editor_selected(&h), Some(StopRef::Color(1)), "a stop press selects it");
}

#[test]
fn the_overlays_stops_and_midpoints_read_as_the_bars_markers_do() {
    let mut h = harness();
    click(&mut h, 0.3);
    let (canvas, bar) = (markers(&h, "Canvas gradient"), markers(&h, "Gradient"));
    assert_eq!(canvas.len(), 7);
    assert_eq!(canvas, bar);
}

#[test]
fn a_press_on_a_canvas_stop_during_the_editors_location_run_commits_the_run_first_and_drags_from_the_typed_value() {
    let mut h = harness();
    click(&mut h, 0.3);
    let field = h.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Location")).rect().center();
    click_at(&mut h, field);
    h.event(Event::Text("40".into()));
    h.run();
    assert_eq!(h.state().log, [("bar", "begin")], "the run is open");
    press(&mut h, 0.6);
    move_to(&mut h, 0.7);
    release(&mut h, 0.7);
    assert_eq!(h.state().log, [("bar", "begin"), ("bar", "commit"), ("canvas", "begin"), ("canvas", "commit")]);
    assert_eq!(locations(&h), [0.0, 0.4, 0.7, 1.0], "the typed value stands and the drag goes on from it");
    assert_eq!(h.state().doc.undo.len(), 2, "the run and the drag are a step each");
}

#[test]
fn a_screen_readers_step_on_the_editor_during_a_canvas_drag_ends_the_drag_and_it_goes_on_in_a_gesture_of_its_own_with_the_selection_following_the_stop()
 {
    let mut h = harness();
    press(&mut h, 0.3);
    move_to(&mut h, 0.5);
    assert_eq!(locations(&h), [0.0, 0.5, 0.6, 1.0]);
    let (target_node, target_tree) = h.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Location")).accesskit_node().locate();
    h.input_mut().events.push(Event::AccessKitActionRequest(ActionRequest { action: Action::Increment, target_tree, target_node, data: None }));
    h.run();
    assert!((locations(&h)[1] - 0.5).abs() < 1e-4, "the pointer wins for the dragged stop's location: {:?}", locations(&h));
    assert_eq!(
        h.state().log,
        [("canvas", "begin"), ("bar", "set"), ("canvas", "begin")],
        "the host ended the drag for the panel's Set and the drag goes on in a gesture of its own"
    );
    move_to(&mut h, 0.7);
    release(&mut h, 0.7);
    assert_eq!(locations(&h), [0.0, 0.6, 0.7, 1.0], "the drag goes on past its neighbour");
    assert_eq!(h.state().doc.now.stops.color[2].tag, 7, "with its stop");
    let log = &h.state().log;
    assert_eq!(log[3..], [("canvas", "commit")], "{log:?}");
    assert_eq!(h.state().doc.undo.len(), 2, "the panel's Set and the drag are a step each");
    assert_eq!(editor_selected(&h), Some(StopRef::Color(2)), "the shared selection follows the stop");
    let field = h.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Location"));
    assert_eq!(field.value().as_deref(), Some("70%"));
}
