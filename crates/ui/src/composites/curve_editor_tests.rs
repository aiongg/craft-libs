//! Curve editor tests: pointer and key editing through the edit lifecycle, the fields, focus,
//! AccessKit, the direction, and the key claim.

use egui::accesskit::Role;
use egui::{Context, Event, Id, Key, KeyboardShortcut, Modifiers, PointerButton, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::*;
use crate::behavior::KeyClaim;
use crate::direction::{self, Direction};
use crate::edit::{CancelReason, EditEvent};
use crate::localized::LocaleNumbers;
use crate::testing::a11y;
use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, install};

/// A host of one curve that applies every value and restores the start value on a cancel.
struct Host {
    curve: Curve,
    before: Option<Curve>,
    events: Vec<EditEvent<Curve>>,
    /// The graph's id and rect, from the last frame.
    id: Id,
    rect: Rect,
    pad: f32,
    /// `line.focus`.
    focus: egui::Color32,
    channel: usize,
    max_points: usize,
    /// Shown inside `edit::scope`.
    scoped: bool,
    /// An Esc press reached the host after the editor.
    escape_seen: bool,
    width: f32,
    pencil: bool,
}

impl Host {
    fn new(points: &[[f32; 2]]) -> Self {
        Self {
            curve: Curve::Points(points.to_vec()),
            before: None,
            events: Vec::new(),
            id: Id::NULL,
            rect: Rect::NOTHING,
            pad: 0.0,
            focus: egui::Color32::TRANSPARENT,
            channel: 0,
            max_points: 16,
            scoped: false,
            escape_seen: false,
            width: 300.0,
            pencil: false,
        }
    }

    fn show(&mut self, ui: &mut Ui) {
        let numbers = LocaleNumbers::from_tag("en");
        let channels = [CurveChannel::new("RGB", ChannelColor::Neutral), CurveChannel::new("Red", ChannelColor::Red)];
        let tokens = Tokens::of(ui);
        self.pad = CurveEditor::style(&tokens).pad;
        self.focus = tokens.color.line.focus.to_egui();
        let curve = self.curve.clone();
        let editor = CurveEditor::new("curve", "Curve", &curve, &numbers)
            .channels(&channels)
            .channel(self.channel)
            .max_points(self.max_points)
            .pencil(self.pencil);
        let width = self.width;
        let scoped = self.scoped;
        let edit = ui
            .allocate_ui(vec2(width, 0.0), |ui| {
                ui.set_max_width(width);
                match scoped {
                    true => crate::edit::scope(ui, |ui| editor.show(ui)).inner,
                    false => editor.show(ui),
                }
            })
            .inner;
        self.escape_seen |= ui.input(|i| i.key_pressed(Key::Escape));
        self.id = edit.response.id;
        self.rect = edit.response.rect;
        if let Some(event) = edit.event {
            match &event {
                EditEvent::Begin { .. } => self.before = Some(self.curve.clone()),
                EditEvent::Cancel { reason, .. } if *reason != CancelReason::External => {
                    if let Some(before) = self.before.take() {
                        self.curve = before;
                    }
                }
                other => {
                    if let Some(value) = other.value() {
                        self.curve = value.clone();
                    }
                }
            }
            self.events.push(event);
        }
    }

    /// The drawn curve's outputs.
    fn drawn(&self) -> Vec<f32> {
        match &self.curve {
            Curve::Map(map) => map.clone(),
            Curve::Points(_) => Vec::new(),
        }
    }

    fn points(&self) -> Vec<[f32; 2]> {
        self.curve.points().map(<[_]>::to_vec).unwrap_or_default()
    }

    /// Where level `q` is on screen.
    fn at(&self, q: [f32; 2]) -> Pos2 {
        let a = self.rect.shrink(self.pad);
        pos2(a.left() + q[0] / 255.0 * a.width(), a.bottom() - q[1] / 255.0 * a.height())
    }

    /// The events so far as short names.
    fn kinds(&self) -> Vec<&'static str> {
        self.events
            .iter()
            .map(|e| match e {
                EditEvent::Begin { .. } => "Begin",
                EditEvent::Update { .. } => "Update",
                EditEvent::Commit { changed: true, .. } => "Commit",
                EditEvent::Commit { changed: false, .. } => "Commit unchanged",
                EditEvent::Cancel { .. } => "Cancel",
                EditEvent::Set { .. } => "Set",
            })
            .collect()
    }
}

const THREE: [[f32; 2]; 3] = [[0.0, 0.0], [128.0, 128.0], [255.0, 255.0]];

fn harness(host: Host, dir: Direction) -> Harness<'static, Host> {
    let mut harness = Harness::builder()
        .with_size([480.0, 480.0])
        .with_context_setup(move |ctx: &Context| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(|ui, host: &mut Host| host.show(ui), host);
    harness.run();
    harness
}

fn rig(points: &[[f32; 2]]) -> Harness<'static, Host> {
    harness(Host::new(points), Direction::Ltr)
}

fn pointer(harness: &mut Harness<'_, Host>, pos: Pos2, button: PointerButton, pressed: bool, modifiers: Modifiers) {
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(Event::PointerButton { pos, button, pressed, modifiers });
    harness.step();
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
}

fn move_to(harness: &mut Harness<'_, Host>, pos: Pos2) {
    harness.hover_at(pos);
    harness.step();
}

fn press(harness: &mut Harness<'_, Host>, pos: Pos2) {
    move_to(harness, pos);
    pointer(harness, pos, PointerButton::Primary, true, Modifiers::NONE);
}

fn release(harness: &mut Harness<'_, Host>, pos: Pos2) {
    pointer(harness, pos, PointerButton::Primary, false, Modifiers::NONE);
    harness.run();
}

fn click_with(harness: &mut Harness<'_, Host>, pos: Pos2, button: PointerButton, modifiers: Modifiers) {
    move_to(harness, pos);
    pointer(harness, pos, button, true, modifiers);
    pointer(harness, pos, button, false, modifiers);
    harness.run();
}

fn key(harness: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

/// Cmd on a Mac, Ctrl elsewhere, as egui-winit reports them.
const CMD: Modifiers = Modifiers { ctrl: !cfg!(target_os = "macos"), mac_cmd: cfg!(target_os = "macos"), command: true, alt: false, shift: false };

fn graph<'a>(harness: &'a Harness<'_, Host>) -> Node<'a> {
    harness.get_by(|n| n.role() == Role::Group && n.label().as_deref() == Some("Curve"))
}

/// Whether the graph has keyboard focus. AccessKit reports the selected point as focused, the
/// graph's active descendant.
fn graph_focused(harness: &Harness<'_, Host>) -> bool {
    harness.ctx.memory(|m| m.has_focus(harness.state().id))
}

/// Focuses the graph from the keyboard, without a change.
fn focus(harness: &mut Harness<'_, Host>) {
    harness.key_press(Key::Tab);
    harness.run();
    assert!(graph_focused(harness), "Tab reaches the graph first");
}

fn selected(harness: &Harness<'_, Host>) -> Option<usize> {
    CurveEditor::selected(&harness.ctx, harness.state().id)
}

#[test]
fn a_press_on_the_empty_graph_adds_a_point_and_its_drag_is_one_gesture() {
    let mut harness = rig(&THREE);
    let at = harness.state().at([64.0, 200.0]);
    press(&mut harness, at);
    for y in [180.0, 160.0, 140.0] {
        let at = harness.state().at([64.0, y]);
        move_to(&mut harness, at);
    }
    let at = harness.state().at([64.0, 140.0]);
    release(&mut harness, at);
    let kinds = harness.state().kinds();
    assert_eq!(kinds.first(), Some(&"Begin"));
    assert_eq!(kinds.last(), Some(&"Commit"));
    assert!(kinds[1..kinds.len() - 1].iter().all(|k| *k == "Update"), "{kinds:?}");
    assert_eq!(harness.state().points(), vec![[0.0, 0.0], [64.0, 140.0], [128.0, 128.0], [255.0, 255.0]]);
    assert_eq!(selected(&harness), Some(1));
}

#[test]
fn a_press_on_a_point_without_moving_commits_unchanged_and_selects_it() {
    let mut harness = rig(&THREE);
    let point = harness.state().at([128.0, 128.0]) + vec2(3.0, 2.0);
    press(&mut harness, point);
    release(&mut harness, point);
    assert_eq!(harness.state().kinds(), ["Begin", "Commit unchanged"]);
    assert_eq!(harness.state().points(), THREE.to_vec(), "the grab offset keeps the point where it was");
    assert_eq!(selected(&harness), Some(1));
}

#[test]
fn esc_during_a_drag_cancels_it_and_the_before_curve_comes_back() {
    let mut harness = rig(&THREE);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    let at = harness.state().at([128.0, 40.0]);
    move_to(&mut harness, at);
    let at = harness.state().at([128.0, 40.0]);
    move_to(&mut harness, at);
    assert_eq!(harness.state().points()[1], [128.0, 40.0]);
    harness.key_press(Key::Escape);
    harness.step();
    assert_eq!(harness.state().kinds().last(), Some(&"Cancel"));
    let at = harness.state().at([128.0, 20.0]);
    move_to(&mut harness, at);
    let at = harness.state().at([128.0, 20.0]);
    release(&mut harness, at);
    assert_eq!(harness.state().points(), THREE.to_vec());
    assert_eq!(harness.state().kinds(), ["Begin", "Update", "Cancel"], "the rest of the drag does nothing");
    assert_eq!(graph(&harness).value().as_deref(), Some("RGB, 3 points"));
}

#[test]
fn an_esc_press_without_a_drag_is_left_to_the_host() {
    let mut harness = rig(&THREE);
    focus(&mut harness);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().escape_seen, "the press stays in the input");
    assert!(harness.state().events.is_empty());
}

#[test]
fn a_point_dragged_off_the_graph_goes_and_comes_back_within_one_gesture() {
    let mut harness = rig(&THREE);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    let below = harness.state().rect.bottom() + 40.0;
    let at = pos2(harness.state().at([128.0, 0.0]).x, below);
    move_to(&mut harness, at);
    let at = pos2(harness.state().at([128.0, 0.0]).x, below);
    move_to(&mut harness, at);
    assert_eq!(harness.state().points().len(), 2, "removed off the graph");
    let at = harness.state().at([100.0, 90.0]);
    move_to(&mut harness, at);
    let at = harness.state().at([100.0, 90.0]);
    move_to(&mut harness, at);
    assert_eq!(harness.state().points(), vec![[0.0, 0.0], [100.0, 90.0], [255.0, 255.0]], "back on re-entry");
    let at = harness.state().at([100.0, 90.0]);
    release(&mut harness, at);
    let kinds = harness.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")));
    assert_eq!(kinds.iter().filter(|k| **k == "Begin").count(), 1, "{kinds:?}");
}

#[test]
fn end_points_stay() {
    let mut harness = rig(&THREE);
    let far = pos2(harness.state().rect.left() - 60.0, harness.state().rect.bottom() + 60.0);
    let at = harness.state().at([0.0, 0.0]);
    press(&mut harness, at);
    move_to(&mut harness, far);
    release(&mut harness, far);
    assert_eq!(harness.state().points(), THREE.to_vec(), "an end point dragged off stays, at the corner");
    let at = harness.state().at([255.0, 255.0]);
    click_with(&mut harness, at, PointerButton::Secondary, Modifiers::NONE);
    key(&mut harness, Modifiers::NONE, Key::Minus);
    assert_eq!(selected(&harness), Some(2));
    key(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(harness.state().points(), THREE.to_vec());
    assert_eq!(harness.state().kinds(), ["Begin", "Commit unchanged"], "deleting an end point does nothing");
}

#[test]
fn a_full_curve_takes_no_point_and_a_near_input_selects_the_point_there() {
    let mut harness = harness(Host { max_points: 3, ..Host::new(&THREE) }, Direction::Ltr);
    let at = harness.state().at([60.0, 200.0]);
    press(&mut harness, at);
    let at = harness.state().at([60.0, 200.0]);
    release(&mut harness, at);
    assert!(harness.state().events.is_empty(), "{:?}", harness.state().kinds());
    assert_eq!(harness.state().points(), THREE.to_vec());

    let mut harness = rig(&THREE);
    let at = harness.state().at([129.0, 20.0]);
    press(&mut harness, at);
    let at = harness.state().at([129.0, 20.0]);
    release(&mut harness, at);
    assert_eq!(selected(&harness), Some(1));
    assert_eq!(harness.state().points().len(), 3, "no second point beside it");
}

#[test]
fn right_click_cmd_click_and_delete_each_remove_a_point_in_one_set() {
    let four = [[0.0, 0.0], [64.0, 80.0], [128.0, 128.0], [255.0, 255.0]];
    let mut harness = rig(&four);
    let at = harness.state().at([64.0, 80.0]);
    click_with(&mut harness, at, PointerButton::Secondary, Modifiers::NONE);
    assert_eq!(harness.state().kinds(), ["Set"]);
    assert_eq!(harness.state().points(), THREE.to_vec());

    let mut harness = rig(&four);
    let at = harness.state().at([64.0, 80.0]);
    click_with(&mut harness, at, PointerButton::Primary, CMD);
    assert_eq!(harness.state().kinds(), ["Set"]);
    assert_eq!(harness.state().points(), THREE.to_vec());

    let mut harness = rig(&four);
    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::Plus);
    key(&mut harness, Modifiers::NONE, Key::Plus);
    assert_eq!(selected(&harness), Some(1));
    key(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(harness.state().kinds(), ["Set"]);
    assert_eq!(harness.state().points(), THREE.to_vec());
    assert_eq!(selected(&harness), None);
}

#[test]
fn arrows_nudge_the_selected_point_in_one_set_each_between_its_neighbours() {
    let mut harness = rig(&[[0.0, 0.0], [120.0, 100.0], [130.0, 140.0], [255.0, 255.0]]);
    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::Plus);
    key(&mut harness, Modifiers::NONE, Key::Plus);
    key(&mut harness, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(harness.state().points()[1], [120.0, 101.0]);
    key(&mut harness, Modifiers::SHIFT, Key::ArrowDown);
    assert_eq!(harness.state().points()[1], [120.0, 91.0]);
    key(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
    assert_eq!(harness.state().points()[1], [129.0, 91.0], "strictly before the next point's input");
    key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(harness.state().kinds(), ["Set", "Set", "Set"], "a nudge that moves nothing sends nothing");
    assert!(graph_focused(&harness), "arrows stay in the graph");
}

#[test]
fn plus_and_minus_select_points_in_order_and_stop_at_the_ends() {
    let mut harness = rig(&THREE);
    focus(&mut harness);
    let mut order = Vec::new();
    for k in [Key::Plus, Key::Equals, Key::Plus, Key::Plus, Key::Minus, Key::Minus, Key::Minus] {
        key(&mut harness, Modifiers::NONE, k);
        order.push(selected(&harness));
    }
    assert_eq!(order, [Some(0), Some(1), Some(2), Some(2), Some(1), Some(0), Some(0)]);
    CurveEditor::select(&harness.ctx, harness.state().id, None);
    key(&mut harness, Modifiers::NONE, Key::Minus);
    assert_eq!(selected(&harness), Some(2), "minus with nothing selected selects the last");
    assert!(harness.state().events.is_empty(), "selection is view state");
}

#[test]
fn insert_and_cmd_enter_add_a_point_halfway_to_the_next_and_select_it() {
    let mut harness = rig(&THREE);
    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::Insert);
    assert_eq!(harness.state().points()[1], [64.0, 64.0], "nothing selected: across the widest gap, on the curve");
    assert_eq!(selected(&harness), Some(1));
    key(&mut harness, Modifiers::NONE, Key::Plus);
    key(&mut harness, CMD, Key::Enter);
    assert_eq!(harness.state().points()[3], [192.0, 192.0], "halfway from the selected point to the next");
    assert_eq!(selected(&harness), Some(3));
    assert_eq!(harness.state().kinds(), ["Set", "Set"]);
}

#[test]
fn the_selection_clears_on_a_channel_change_and_the_host_can_set_it() {
    let mut harness = rig(&THREE);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    let at = harness.state().at([128.0, 128.0]);
    release(&mut harness, at);
    assert_eq!(selected(&harness), Some(1));
    harness.state_mut().channel = 1;
    harness.run();
    assert_eq!(selected(&harness), None);
    CurveEditor::select(&harness.ctx, harness.state().id, Some(2));
    harness.run();
    assert_eq!(selected(&harness), Some(2), "kept in the channel shown next");
}

#[test]
fn a_scrub_of_the_input_field_is_one_gesture_of_curves() {
    let mut harness = rig(&THREE);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    let at = harness.state().at([128.0, 128.0]);
    release(&mut harness, at);
    harness.state_mut().events.clear();
    let start = harness.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Input")).rect().center();
    press(&mut harness, start);
    for dx in [10.0, 20.0, 30.0] {
        move_to(&mut harness, start + vec2(dx, 0.0));
    }
    release(&mut harness, start + vec2(30.0, 0.0));
    let kinds = harness.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "{kinds:?}");
    assert_eq!(kinds.iter().filter(|k| **k == "Begin").count(), 1, "{kinds:?}");
    let points = harness.state().points();
    assert!(points[1][0] > 128.0 && points[1][1] == 128.0, "{points:?}");
}

#[test]
fn inside_a_dialog_scope_the_editor_reports_updates_only() {
    let mut harness = harness(Host { scoped: true, ..Host::new(&THREE) }, Direction::Ltr);
    let at = harness.state().at([64.0, 200.0]);
    press(&mut harness, at);
    let at = harness.state().at([64.0, 180.0]);
    move_to(&mut harness, at);
    let at = harness.state().at([64.0, 180.0]);
    release(&mut harness, at);
    assert!(!harness.state().events.is_empty());
    assert!(harness.state().kinds().iter().all(|k| *k == "Update"), "{:?}", harness.state().kinds());
}

#[test]
fn the_focus_ring_shows_for_keyboard_focus_only() {
    let mut harness = rig(&THREE);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    release(&mut harness, at);
    // With no point selected AccessKit reports the graph itself as focused.
    CurveEditor::select(&harness.ctx, harness.state().id, None);
    harness.run();
    assert!(graph_focused(&harness));
    let line_focus = harness.state().focus;
    let rings = a11y::focus_rings(&harness).map(|(_, rings)| rings).unwrap_or_default();
    assert!(rings.iter().all(|r| r.stroke.color != line_focus), "no ring after a pointer press: {rings:?}");
    let id = harness.state().id;
    harness.ctx.memory_mut(|m| m.surrender_focus(id));
    harness.run();
    focus(&mut harness);
    a11y::assert_focus_ring(&harness, line_focus);
}

#[test]
fn the_graph_and_its_points_are_in_the_accesskit_tree() {
    let mut harness = rig(&THREE);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    let at = harness.state().at([128.0, 128.0]);
    release(&mut harness, at);
    let graph = graph(&harness);
    assert_eq!(graph.value().as_deref(), Some("RGB, 3 points"));
    assert!(graph.accesskit_node().data().supports_action(egui::accesskit::Action::Focus));
    let points: Vec<_> = graph.children().filter(|n| n.accesskit_node().role() == Role::GraphicsObject).collect();
    let names: Vec<_> = points.iter().map(|n| n.accesskit_node().label().unwrap_or_default()).collect();
    assert_eq!(names, ["Point 1 of 3", "Point 2 of 3", "Point 3 of 3"]);
    assert_eq!(points[1].value().as_deref(), Some("Input 128, Output 128"));
    let selected: Vec<_> = points.iter().map(|n| n.accesskit_node().data().is_selected()).collect();
    assert_eq!(selected, [Some(false), Some(true), Some(false)]);
    assert_eq!(graph.accesskit_node().data().active_descendant(), Some(points[1].accesskit_node().locate().0));
    for name in ["Input", "Output"] {
        harness.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some(name));
    }
}

#[test]
fn right_to_left_keeps_the_graph_geometry() {
    let offsets = |dir| {
        let harness = harness(Host::new(&THREE), dir);
        let graph = graph(&harness);
        let origin = graph.rect().min;
        graph.children().filter(|n| n.accesskit_node().role() == Role::GraphicsObject).map(|n| n.rect().center() - origin).collect::<Vec<_>>()
    };
    let ltr = offsets(Direction::Ltr);
    assert_eq!(ltr.len(), 3, "{ltr:?}");
    assert!(ltr[0].x < ltr[2].x && ltr[0].y > ltr[2].y, "the curve rises to the right: {ltr:?}");
    assert_eq!(offsets(Direction::Rtl), ltr);
}

#[test]
fn the_graph_claims_its_keys_only_while_focused() {
    let mut harness = rig(&THREE);
    let delete = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
    assert!(!KeyClaim::claimed(&harness.ctx, &delete));
    focus(&mut harness);
    for k in [Key::Delete, Key::Backspace, Key::Plus, Key::Equals, Key::Minus] {
        assert!(KeyClaim::claimed(&harness.ctx, &KeyboardShortcut::new(Modifiers::NONE, k)), "{k:?}");
    }
    assert!(KeyClaim::claimed(&harness.ctx, &KeyboardShortcut::new(Modifiers::SHIFT, Key::Plus)), "+ is Shift+= on many keyboards");
    assert!(!KeyClaim::claimed(&harness.ctx, &KeyboardShortcut::new(CMD, Key::Minus)), "Cmd/Ctrl+- stays the app's");
    assert!(!KeyClaim::claimed(&harness.ctx, &KeyboardShortcut::new(Modifiers::NONE, Key::A)));
    let id = harness.state().id;
    harness.ctx.memory_mut(|m| m.surrender_focus(id));
    harness.run();
    assert!(!KeyClaim::claimed(&harness.ctx, &delete));
}

#[test]
fn the_stories_name_every_control() {
    for story in crate::story::registry().into_iter().filter(|s| s.id.starts_with("composites/curve-editor/")) {
        let mut args = (story.args)();
        let mut harness = Harness::builder()
            .with_size([480.0, 560.0])
            .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
            .build_ui(move |ui| (story.show)(ui, &mut args));
        harness.run();
        a11y::assert_labelled(&harness);
        assert!(harness.query_all_by(|n| n.role() == Role::GraphicsObject).count() >= 3, "{}: the points are in the tree", story.id);
    }
}

#[test]
fn a_press_and_release_in_one_frame_adds_a_point_in_one_gesture() {
    let mut harness = rig(&THREE);
    let at = harness.state().at([64.0, 200.0]);
    move_to(&mut harness, at);
    harness.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    harness.run();
    assert_eq!(harness.state().kinds(), ["Begin", "Commit"]);
    assert_eq!(harness.state().points()[1], [64.0, 200.0]);
}

fn pencil_rig() -> Harness<'static, Host> {
    let mut host = Host::new(&[]);
    host.curve = Curve::Map((0..256).map(|k| k as f32).collect());
    host.pencil = true;
    harness(host, Direction::Ltr)
}

#[test]
fn a_pencil_stroke_is_one_gesture_that_draws_straight_segments_between_samples() {
    let mut harness = pencil_rig();
    let from = harness.state().at([20.0, 200.0]);
    press(&mut harness, from);
    let mid = harness.state().at([60.0, 100.0]);
    move_to(&mut harness, mid);
    let to = harness.state().at([100.0, 100.0]);
    move_to(&mut harness, to);
    release(&mut harness, to);
    let kinds = harness.state().kinds();
    assert_eq!(kinds.first(), Some(&"Begin"));
    assert_eq!(kinds.last(), Some(&"Commit"));
    assert!(kinds[1..kinds.len() - 1].iter().all(|k| *k == "Update"), "{kinds:?}");
    let map = harness.state().drawn();
    assert_eq!((map[20], map[60], map[100]), (200.0, 100.0, 100.0));
    assert_eq!(map[40], 150.0, "straight between the samples");
    assert_eq!((map[10], map[150]), (10.0, 150.0), "outside the stroke stays");
}

#[test]
fn shift_press_continues_in_a_straight_line_from_the_previous_stroke() {
    let mut harness = pencil_rig();
    let start = harness.state().at([20.0, 200.0]);
    press(&mut harness, start);
    release(&mut harness, start);
    harness.state_mut().events.clear();
    let to = harness.state().at([120.0, 100.0]);
    move_to(&mut harness, to);
    pointer(&mut harness, to, PointerButton::Primary, true, Modifiers::SHIFT);
    release(&mut harness, to);
    assert_eq!(harness.state().kinds(), ["Begin", "Update", "Commit"], "its own gesture");
    let map = harness.state().drawn();
    assert_eq!((map[20], map[70], map[120]), (200.0, 150.0, 100.0));
}

#[test]
fn esc_cancels_a_pencil_stroke_and_the_before_curve_comes_back() {
    let mut harness = pencil_rig();
    let before = harness.state().drawn();
    let from = harness.state().at([20.0, 200.0]);
    press(&mut harness, from);
    let to = harness.state().at([80.0, 40.0]);
    move_to(&mut harness, to);
    move_to(&mut harness, to);
    assert_ne!(harness.state().drawn(), before);
    harness.key_press(Key::Escape);
    harness.step();
    let rest = harness.state().at([120.0, 10.0]);
    move_to(&mut harness, rest);
    release(&mut harness, rest);
    assert_eq!(harness.state().kinds().last(), Some(&"Cancel"));
    assert_eq!(harness.state().drawn(), before);
}

#[test]
fn pencil_mode_hides_the_points_ignores_keys_and_reads_as_drawn() {
    let mut harness = pencil_rig();
    let at = harness.state().at([100.0, 100.0]);
    click_with(&mut harness, at, PointerButton::Primary, Modifiers::NONE);
    harness.state_mut().events.clear();
    for k in [Key::ArrowUp, Key::Insert, Key::Delete, Key::Plus] {
        key(&mut harness, Modifiers::NONE, k);
    }
    assert!(harness.state().events.is_empty(), "{:?}", harness.state().kinds());
    assert_eq!(harness.query_all_by_role(Role::GraphicsObject).count(), 0);
    assert_eq!(graph(&harness).value().as_deref(), Some("RGB, drawn"));
    assert_eq!(harness.query_all_by_label("Input").count(), 0, "no fields for a drawn curve");
}

#[test]
fn other_channels_draw_behind_unless_active_or_unchanged() {
    let numbers = LocaleNumbers::from_tag("en");
    let channels = [
        CurveChannel::new("RGB", ChannelColor::Neutral),
        CurveChannel::new("Red", ChannelColor::Red),
        CurveChannel::new("Green", ChannelColor::Green),
        CurveChannel::new("Blue", ChannelColor::Blue),
    ];
    let bent = Curve::Points(vec![[0.0, 0.0], [128.0, 160.0], [255.0, 255.0]]);
    let curves = [bent.clone(), bent.clone(), Curve::identity(255.0), Curve::Map((0..256).map(|k| (k as f32 + 9.0).min(255.0)).collect())];
    let editor = |active: usize| CurveEditor::new("c", "Curve", &curves[active], &numbers).channels(&channels).channel(active).overlays(&curves);
    assert_eq!(editor(0).other_channels(), [1, 3], "the identity green and the active composite are skipped");
    assert_eq!(editor(1).other_channels(), [0, 3]);
    assert!(CurveEditor::new("c", "Curve", &curves[0], &numbers).channels(&channels).other_channels().is_empty(), "no overlays, none drawn");
}
