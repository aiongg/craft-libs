//! Curve editor tests: pointer and key editing through the edit lifecycle, the fields, focus,
//! AccessKit, the direction, and the key claim.

use egui::accesskit::{Action, ActionRequest, Role};
use egui::{Context, Event, Id, Key, KeyboardShortcut, Modifiers, PointerButton, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::*;
use crate::behavior::KeyClaim;
use crate::controls::NumberField;
use crate::direction::{self, Direction};
use crate::edit::{CancelReason, EditEvent, EditPolicy, GestureHost, UndoMarks};
use crate::localized::LocaleNumbers;
use crate::testing::a11y;
use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, current, install};

/// A host of one curve that applies every value and restores the start value on a cancel.
struct Host {
    curve: Curve,
    before: Option<Curve>,
    events: Vec<EditEvent<Curve>>,
    /// The pass of each event.
    passes: Vec<u64>,
    policy: EditPolicy,
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
    percent: bool,
    corners: bool,
    /// Hands the events to a [`GestureHost`] over a document with undo steps instead.
    undo: Option<Undo>,
    /// An app's field of the drawn curve's output at level 128, outside the editor: drawn before
    /// the editor when true, else after it.
    outside: Option<bool>,
    /// The outside field's events by kind, with their passes.
    outside_events: Vec<(&'static str, u64)>,
    /// Draws a host row between the graph and the fields; its rect is kept in `between_rect`.
    between: bool,
    bars: bool,
    between_rect: Option<Rect>,
}

/// A document of one curve with undo steps, under a [`GestureHost`].
struct Undo {
    host: GestureHost<CurveDoc>,
    doc: CurveDoc,
}

struct CurveDoc {
    now: Curve,
    /// The curve before each step.
    undo: Vec<Curve>,
}

impl UndoMarks for CurveDoc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(before) = self.undo.get(mark).cloned() {
            self.now = before;
        }
        self.undo.truncate(mark);
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

impl Host {
    fn new(points: &[[f32; 2]]) -> Self {
        Self {
            curve: Curve::smooth(points.to_vec()),
            before: None,
            events: Vec::new(),
            passes: Vec::new(),
            policy: EditPolicy::default(),
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
            percent: false,
            corners: false,
            undo: None,
            outside: None,
            outside_events: Vec::new(),
            between: false,
            bars: true,
            between_rect: None,
        }
    }

    /// A host that hands the events to a [`GestureHost`].
    fn with_undo(points: &[[f32; 2]]) -> Self {
        let curve = Curve::smooth(points.to_vec());
        Self { undo: Some(Undo { host: GestureHost::new(), doc: CurveDoc { now: curve, undo: Vec::new() } }), ..Self::new(points) }
    }

    fn show(&mut self, ui: &mut Ui) {
        let numbers = LocaleNumbers::from_tag("en");
        let channels = [CurveChannel::new("RGB", ChannelColor::Neutral), CurveChannel::new("Red", ChannelColor::Red)];
        let tokens = Tokens::of(ui);
        self.pad = CurveEditor::style(&tokens).pad;
        self.focus = tokens.color.line.focus.to_egui();
        let curve = self.curve.clone();
        let mut editor = CurveEditor::new("curve", "Curve", &curve, &numbers)
            .channels(&channels)
            .channel(self.channel)
            .max_points(self.max_points)
            .policy(self.policy)
            .pencil(self.pencil)
            .corners(self.corners)
            .bars(self.bars);
        if self.percent {
            editor = editor.percent();
        }
        let mut row = None;
        if self.between {
            editor = editor.between(|ui| row = Some(ui.label("Name").rect));
        }
        let width = self.width;
        let scoped = self.scoped;
        if self.outside == Some(true) {
            self.show_outside(ui, &numbers);
        }
        let edit = ui
            .allocate_ui(vec2(width, 0.0), |ui| {
                ui.set_max_width(width);
                match scoped {
                    true => crate::edit::scope(ui, |ui| editor.show(ui)).inner,
                    false => editor.show(ui),
                }
            })
            .inner;
        if self.outside == Some(false) {
            self.show_outside(ui, &numbers);
        }
        self.between_rect = row;
        self.escape_seen |= ui.input(|i| i.key_pressed(Key::Escape));
        self.id = edit.response.id;
        self.rect = edit.response.rect;
        if edit.event.is_some() {
            self.passes.push(ui.ctx().cumulative_pass_nr());
        }
        if let Some(undo) = &mut self.undo {
            if let Some(event) = edit.event.clone() {
                undo.host.handle(ui.ctx(), &mut undo.doc, event, |doc, value| {
                    let before = std::mem::replace(&mut doc.now, value);
                    doc.undo.push(before);
                });
            }
            let _ = undo.host.end_frame(ui.ctx(), &mut undo.doc);
            self.curve = undo.doc.now.clone();
            self.events.extend(edit.event);
            return;
        }
        if let Some(event) = edit.event {
            match &event {
                EditEvent::Begin { value, .. } => {
                    self.before = Some(self.curve.clone());
                    if let Some(value) = value {
                        self.curve = value.clone();
                    }
                }
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

    /// The outside field ([`Host::outside`]), applied to the drawn curve.
    fn show_outside(&mut self, ui: &mut Ui, numbers: &LocaleNumbers) {
        let value = self.drawn().get(128).copied().unwrap_or_default();
        let edit = NumberField::new("outside", "Midtone", f64::from(value), numbers).range(0.0..=255.0).decimals(0..=0).show(ui);
        let Some(event) = edit.event else { return };
        let kind = match &event {
            EditEvent::Begin { .. } => "Begin",
            EditEvent::Update { .. } => "Update",
            EditEvent::Commit { .. } => "Commit",
            EditEvent::Cancel { .. } => "Cancel",
            EditEvent::Set { .. } => "Set",
        };
        self.outside_events.push((kind, ui.ctx().cumulative_pass_nr()));
        if let (Some(v), Curve::Map(map)) = (event.value(), &mut self.curve) {
            map[128] = *v as f32;
        }
    }

    /// Forgets the events so far.
    fn clear(&mut self) {
        self.events.clear();
        self.passes.clear();
    }

    /// The pass of the first event of `kind` ([`kinds`](Host::kinds)) at or after event `from`.
    fn pass_of(&self, kind: &str, from: usize) -> u64 {
        let i = self.kinds().iter().skip(from).position(|k| *k == kind);
        assert!(i.is_some(), "no {kind} in {:?}", self.kinds());
        self.passes[from + i.unwrap_or_default()]
    }

    /// The drawn curve's outputs.
    fn drawn(&self) -> Vec<f32> {
        match &self.curve {
            Curve::Map(map) => map.clone(),
            Curve::Points { .. } => Vec::new(),
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

    // PhotoCraft's `point_curve`: the press takes the point there to the pointer, one gesture.
    let mut harness = rig(&THREE);
    let at = harness.state().at([129.0, 20.0]);
    press(&mut harness, at);
    release(&mut harness, at);
    assert_eq!(selected(&harness), Some(1));
    assert_eq!(harness.state().points(), vec![[0.0, 0.0], [129.0, 20.0], [255.0, 255.0]], "no second point beside it");
    let kinds = harness.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "{kinds:?}");
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

    // A gap under four levels has no whole level two away from both ends.
    let mut harness = rig(&[[0.0, 0.0], [100.0, 100.0], [103.0, 103.0], [255.0, 255.0]]);
    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::Plus);
    key(&mut harness, Modifiers::NONE, Key::Plus);
    key(&mut harness, Modifiers::NONE, Key::Insert);
    assert_eq!((harness.state().points().len(), selected(&harness)), (4, Some(1)), "nothing added, the selection kept");
    assert!(harness.state().events.is_empty());
}

#[test]
fn shift_insert_and_alt_or_cmd_arrows_are_left_to_the_app() {
    let mut harness = rig(&THREE);
    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::Plus);
    key(&mut harness, Modifiers::NONE, Key::Plus);
    key(&mut harness, Modifiers::SHIFT, Key::Insert);
    key(&mut harness, Modifiers::ALT, Key::ArrowUp);
    key(&mut harness, CMD, Key::ArrowUp);
    key(&mut harness, Modifiers::SHIFT, Key::Delete);
    assert_eq!(harness.state().points(), THREE.to_vec());
    assert!(harness.state().events.is_empty(), "{:?}", harness.state().kinds());
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
fn a_host_selection_during_a_drag_leaves_the_drag_to_commit() {
    let mut harness = rig(&THREE);
    let at = harness.state().at([64.0, 200.0]);
    press(&mut harness, at);
    let to = harness.state().at([64.0, 180.0]);
    move_to(&mut harness, to);
    CurveEditor::select(&harness.ctx, harness.state().id, Some(0));
    let to = harness.state().at([64.0, 160.0]);
    move_to(&mut harness, to);
    release(&mut harness, to);
    let kinds = harness.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "{kinds:?}");
    assert_eq!(harness.state().points()[1], [64.0, 160.0]);
}

#[test]
fn a_scrub_of_the_input_field_is_one_gesture_of_curves() {
    let mut harness = rig(&THREE);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    let at = harness.state().at([128.0, 128.0]);
    release(&mut harness, at);
    harness.state_mut().clear();
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

/// Typing into the Input field and pressing the graph without Enter: the typed value goes out
/// first (on the Begin of the run) and stays, and the graph's gesture begins in the pass after the commit, from the typed
/// value, with a plain host and under a [`GestureHost`].
#[test]
fn a_press_on_the_graph_during_a_typing_run_keeps_the_typed_value_and_starts_from_it() {
    for host in [Host::new(&THREE), Host::with_undo(&THREE)] {
        let undo = host.undo.is_some();
        let mut harness = harness(host, Direction::Ltr);
        let point = harness.state().at([128.0, 128.0]);
        press(&mut harness, point);
        release(&mut harness, point);
        harness.state_mut().clear();
        type_into_input(&mut harness, "100");

        let at = harness.state().at([200.0, 60.0]);
        press(&mut harness, at);
        let to = harness.state().at([200.0, 80.0]);
        move_to(&mut harness, to);
        release(&mut harness, to);
        let kinds = harness.state().kinds();
        assert_eq!(kinds[..3], ["Begin", "Commit", "Begin"], "undo {undo}: {kinds:?}");
        assert_eq!(kinds.last(), Some(&"Commit"), "undo {undo}: {kinds:?}");
        assert!(kinds[3..kinds.len() - 1].iter().all(|k| *k == "Update"), "undo {undo}: {kinds:?}");
        assert_eq!(
            harness.state().pass_of("Begin", 2),
            harness.state().pass_of("Commit", 0) + 1,
            "undo {undo}: the press runs the pass after the commit"
        );
        assert_eq!(harness.state().points(), vec![[0.0, 0.0], [100.0, 128.0], [200.0, 80.0], [255.0, 255.0]], "undo {undo}");
        if let Some(u) = &harness.state().undo {
            assert_eq!(u.doc.undo.len(), 2, "the typing run and the drag, one step each");
            assert_eq!(u.doc.undo[1].points(), Some(&[[0.0, 0.0], [100.0, 128.0], [255.0, 255.0]][..]));
        }
    }
}

/// A right-click and a Cmd/Ctrl-click on a point during the Input field's typing run each wait
/// for the run's commit and then delete the point as their own Set: the held press keeps its
/// button and Cmd/Ctrl.
#[test]
fn a_delete_click_on_the_graph_during_a_typing_run_waits_for_its_commit() {
    for (button, modifiers) in [(PointerButton::Secondary, Modifiers::NONE), (PointerButton::Primary, CMD)] {
        let mut harness = rig(&[[0.0, 0.0], [64.0, 80.0], [128.0, 128.0], [255.0, 255.0]]);
        let point = harness.state().at([64.0, 80.0]);
        press(&mut harness, point);
        release(&mut harness, point);
        harness.state_mut().clear();
        type_into_input(&mut harness, "70");
        let at = harness.state().at([128.0, 128.0]);
        click_with(&mut harness, at, button, modifiers);
        assert_eq!(harness.state().kinds(), ["Begin", "Commit", "Set"], "{button:?}");
        assert_eq!(harness.state().pass_of("Set", 0), harness.state().pass_of("Commit", 0) + 1, "{button:?}");
        assert_eq!(harness.state().points(), vec![[0.0, 0.0], [70.0, 80.0], [255.0, 255.0]], "{button:?}");
    }
}

/// Under an on-release policy the host's curve stays during a scrub of the Input field, and the
/// graph's point follows the scrub all the same: the graph draws the editor's shown curve.
#[test]
fn a_field_scrub_moves_the_point_under_an_on_release_policy() {
    let mut harness = harness(Host { policy: EditPolicy::default().on_release_only(true), ..Host::new(&THREE) }, Direction::Ltr);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    release(&mut harness, at);
    harness.state_mut().clear();
    let start = input_field(&harness);
    press(&mut harness, start);
    for dx in [10.0, 20.0, 30.0] {
        move_to(&mut harness, start + vec2(dx, 0.0));
    }
    harness.step();
    assert_eq!(harness.state().points(), THREE.to_vec(), "no updates reach the host");
    let value = point_value(&harness, "Point 2 of 3");
    assert_ne!(value, "Input 128, Output 128", "the point follows the scrub");
    release(&mut harness, start + vec2(30.0, 0.0));
    let points = harness.state().points();
    assert!(points[1][0] > 128.0, "{points:?}");
    assert_eq!(value, format!("Input {}, Output 128", points[1][0]), "the point showed the value the host got");
}

/// A screen reader's step on the Input field while a point is dragged doesn't join the drag: it
/// waits for the release and goes out as its own Set in the next pass, mapped onto the curve the
/// drag left.
#[test]
fn a_screen_readers_step_on_a_field_during_a_graph_drag_is_its_own_set_after_the_release() {
    let mut harness = harness(Host::with_undo(&THREE), Direction::Ltr);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    for y in [110.0, 100.0] {
        let to = harness.state().at([128.0, y]);
        move_to(&mut harness, to);
    }
    let (target_node, target_tree) =
        harness.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Input")).accesskit_node().locate();
    harness.event(Event::AccessKitActionRequest(ActionRequest { action: Action::Increment, target_tree, target_node, data: None }));
    let to = harness.state().at([128.0, 90.0]);
    move_to(&mut harness, to);
    assert_eq!(harness.state().points()[1], [128.0, 90.0], "the step waits");
    let to = harness.state().at([128.0, 90.0]);
    release(&mut harness, to);
    let kinds = harness.state().kinds();
    assert_eq!(kinds.first(), Some(&"Begin"), "{kinds:?}");
    assert_eq!(kinds[kinds.len() - 2..], ["Commit", "Set"], "{kinds:?}");
    assert!(kinds[1..kinds.len() - 2].iter().all(|k| *k == "Update"), "{kinds:?}");
    assert_eq!(harness.state().pass_of("Set", 0), harness.state().pass_of("Commit", 0) + 1);
    assert_eq!(harness.state().points(), vec![[0.0, 0.0], [129.0, 90.0], [255.0, 255.0]]);
    let undo = harness.state().undo.as_ref().map(|u| u.doc.undo.len());
    assert_eq!(undo, Some(2), "the drag and the step, one step each");
}

/// Screen-reader steps on the Input and Output fields in one frame both count: the Output step
/// applies to the curve the Input step left.
#[test]
fn steps_on_both_fields_in_one_frame_both_reach_the_host() {
    let mut harness = harness(Host::with_undo(&THREE), Direction::Ltr);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    release(&mut harness, at);
    harness.state_mut().clear();
    let events = ["Input", "Output"].map(|name| {
        let (target_node, target_tree) =
            harness.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some(name)).accesskit_node().locate();
        Event::AccessKitActionRequest(ActionRequest { action: Action::Increment, target_tree, target_node, data: None })
    });
    harness.input_mut().events.extend(events);
    harness.run();
    assert_eq!(harness.state().kinds(), ["Set"]);
    assert_eq!(harness.state().points(), vec![[0.0, 0.0], [129.0, 129.0], [255.0, 255.0]]);
}

/// An arrow on the graph during a scrub of the selected point's Input field waits for the
/// release and moves only the output: the graph's curve from the arrow's pass doesn't bring back
/// the input the scrub moved on from.
#[test]
fn an_arrow_on_the_graph_during_a_field_scrub_keeps_the_scrubs_value() {
    let mut harness = harness(Host::with_undo(&THREE), Direction::Ltr);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    release(&mut harness, at);
    harness.state_mut().clear();
    let start = input_field(&harness);
    press(&mut harness, start);
    for dx in [10.0, 20.0] {
        move_to(&mut harness, start + vec2(dx, 0.0));
    }
    assert!(graph_focused(&harness), "the scrub leaves the focus on the graph");
    harness.event(Event::Key { key: Key::ArrowUp, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    harness.step();
    let stepped = harness.state().points()[1][0];
    assert_eq!(harness.state().points()[1][1], 128.0, "the arrow waits");
    for dx in [30.0, 40.0] {
        move_to(&mut harness, start + vec2(dx, 0.0));
    }
    release(&mut harness, start + vec2(40.0, 0.0));
    let kinds = harness.state().kinds();
    assert_eq!(kinds[kinds.len() - 2..], ["Commit", "Set"], "{kinds:?}");
    assert_eq!(harness.state().pass_of("Set", 0), harness.state().pass_of("Commit", 0) + 1, "the arrow goes out the pass after the release");
    let committed = harness.state().events.iter().find_map(|e| match e {
        EditEvent::Commit { value, .. } => value.points().map(|p| p[1][0]),
        _ => None,
    });
    assert!(committed.is_some_and(|x| x > stepped), "the scrub moved on after the arrow: {committed:?} vs {stepped}");
    let points = harness.state().points();
    assert_eq!(Some(points[1][0]), committed, "the arrow keeps the input the scrub committed");
    assert_eq!(points[1][1], 129.0, "the arrow applies");
    let undo = harness.state().undo.as_ref().map(|u| u.doc.undo.len());
    assert_eq!(undo, Some(2), "the scrub and the arrow, one step each");
}

/// An arrow on the graph in the same pass as an update of the Input field's scrub doesn't go out
/// with the update: the fields draw from the editor's shown curve, not the graph's, so the
/// update carries only the scrub's input and the arrow waits for the release.
#[test]
fn an_arrow_on_the_graph_in_a_field_scrubs_update_pass_waits_for_the_release() {
    let mut harness = harness(Host::with_undo(&THREE), Direction::Ltr);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    release(&mut harness, at);
    harness.state_mut().clear();
    let start = input_field(&harness);
    press(&mut harness, start);
    move_to(&mut harness, start + vec2(10.0, 0.0));
    let to = start + vec2(20.0, 0.0);
    harness.input_mut().events.extend([
        Event::PointerMoved(to),
        Event::Key { key: Key::ArrowUp, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
    ]);
    harness.step();
    let points = harness.state().points();
    assert!(points[1][0] > 128.0, "the scrub's update went out: {points:?}");
    assert_eq!(points[1][1], 128.0, "the arrow waits: {:?}", harness.state().kinds());
    release(&mut harness, to);
    let kinds = harness.state().kinds();
    assert_eq!(kinds[kinds.len() - 2..], ["Commit", "Set"], "{kinds:?}");
    assert_eq!(harness.state().points()[1][1], 129.0, "the arrow applies after the release");
    let undo = harness.state().undo.as_ref().map(|u| u.doc.undo.len());
    assert_eq!(undo, Some(2), "the scrub and the arrow, one step each");
}

/// An Insert on the graph during a scrub of the selected point's Input field waits for the
/// release, keeping the selection on the scrubbed point, then adds its point halfway from where
/// the scrub left that point to the next, and selects it.
#[test]
fn a_held_insert_selects_the_inserted_point_on_the_curve_of_its_turn() {
    let mut harness = harness(Host::with_undo(&THREE), Direction::Ltr);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    release(&mut harness, at);
    harness.state_mut().clear();
    let start = input_field(&harness);
    press(&mut harness, start);
    move_to(&mut harness, start + vec2(10.0, 0.0));
    harness.event(Event::Key { key: Key::Insert, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    harness.step();
    assert_eq!(harness.state().points().len(), 3, "the insert waits");
    assert_eq!(selected(&harness), Some(1), "the selection stays on the scrubbed point while it waits");
    for dx in [20.0, 30.0] {
        move_to(&mut harness, start + vec2(dx, 0.0));
    }
    release(&mut harness, start + vec2(30.0, 0.0));
    let kinds = harness.state().kinds();
    assert_eq!(kinds[kinds.len() - 2..], ["Commit", "Set"], "{kinds:?}");
    let points = harness.state().points();
    assert_eq!(points.len(), 4, "{points:?}");
    let x = points[1][0];
    assert!(x > 128.0, "the scrub moved the point: {points:?}");
    assert_eq!(points[2][0], ((x + 255.0) / 2.0).round(), "halfway from where the scrub left the point: {points:?}");
    assert_eq!(selected(&harness), Some(2), "the inserted point is selected");
}

/// The Input field's centre.
fn input_field(harness: &Harness<'_, Host>) -> Pos2 {
    harness.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Input")).rect().center()
}

/// Clicks into the Input field and types `text`, without Enter.
fn type_into_input(harness: &mut Harness<'_, Host>, text: &str) {
    let input = input_field(harness);
    click_with(harness, input, PointerButton::Primary, Modifiers::NONE);
    harness.event(Event::Text(text.into()));
    harness.run();
}

/// The AccessKit value of the point named `name`.
fn point_value(harness: &Harness<'_, Host>, name: &str) -> String {
    harness.get_by(|n| n.role() == Role::GraphicsObject && n.label().as_deref() == Some(name)).value().unwrap_or_default()
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
    let claimed = |harness: &Harness<'_, Host>, modifiers, key| KeyClaim::claimed(&harness.ctx, &KeyboardShortcut::new(modifiers, key));
    for k in [Key::Delete, Key::Backspace, Key::Plus, Key::Equals, Key::Minus, Key::Insert] {
        assert!(claimed(&harness, Modifiers::NONE, k), "{k:?}");
    }
    assert!(claimed(&harness, CMD, Key::Enter), "Cmd/Ctrl+Enter adds a point");
    assert!(claimed(&harness, Modifiers::SHIFT, Key::Plus), "+ is Shift+= on many keyboards");
    assert!(!claimed(&harness, CMD, Key::Minus), "Cmd/Ctrl+- stays the app's");
    assert!(!claimed(&harness, Modifiers::SHIFT, Key::Insert), "Shift+Insert (paste) stays the app's");
    assert!(!claimed(&harness, Modifiers::SHIFT, Key::Delete), "Shift+Delete (cut) stays the app's");
    assert!(!claimed(&harness, Modifiers::NONE, Key::A));
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
        if story.id.ends_with("/preview") {
            assert_eq!(harness.query_all_by(|n| n.role() == Role::Image).count(), 3, "{}: the thumbnails are images", story.id);
            continue;
        }
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

/// Runs `host` for one frame of a bare context: the pointer arrives, the primary button goes down
/// and comes up inside the frame (a fast native click, an automation click). The test harness
/// spreads such events over two frames, so it can't show this.
fn same_frame_click(ctx: &Context, host: &mut Host, pos: Pos2, time: f64) {
    let button = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    let raw = egui::RawInput {
        time: Some(time),
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(480.0, 480.0))),
        events: vec![Event::PointerMoved(pos), button(true), button(false)],
        ..Default::default()
    };
    ctx.run_ui(raw, |ui| host.show(ui)).textures_delta.clear();
}

fn bare_context() -> Context {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx
}

fn idle_frame(ctx: &Context, host: &mut Host, time: f64) {
    let raw = egui::RawInput { time: Some(time), screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(480.0, 480.0))), ..Default::default() };
    ctx.run_ui(raw, |ui| host.show(ui)).textures_delta.clear();
}

#[test]
fn a_press_and_release_in_one_frame_selects_a_point_and_adds_one_on_the_empty_plot() {
    let ctx = bare_context();
    let mut host = Host::new(&THREE);
    for t in 0..3 {
        idle_frame(&ctx, &mut host, f64::from(t) * 0.1);
    }
    let at = host.at(THREE[1]);
    same_frame_click(&ctx, &mut host, at, 0.4);
    idle_frame(&ctx, &mut host, 0.5);
    assert_eq!((CurveEditor::selected(&ctx, host.id), host.curve.points().map(<[_]>::len)), (Some(1), Some(3)));
    host.events.clear();
    let at = host.at([64.0, 200.0]);
    same_frame_click(&ctx, &mut host, at, 1.0);
    idle_frame(&ctx, &mut host, 1.1);
    assert_eq!(host.kinds(), ["Set"]);
    assert_eq!(host.points()[1], [64.0, 200.0]);
}

#[test]
fn a_press_and_release_in_one_frame_draws_a_point_in_pencil_mode() {
    let ctx = bare_context();
    let mut host = Host { pencil: true, ..Host::new(&THREE) };
    for t in 0..3 {
        idle_frame(&ctx, &mut host, f64::from(t) * 0.1);
    }
    let at = host.at([64.0, 200.0]);
    same_frame_click(&ctx, &mut host, at, 0.4);
    idle_frame(&ctx, &mut host, 0.5);
    assert_eq!(host.kinds(), ["Set"]);
    assert_eq!(host.drawn()[64], 200.0);
}

#[test]
fn a_graph_in_pencil_mode_keeps_no_claim_from_points_mode() {
    let mut harness = rig(&THREE);
    let delete = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
    focus(&mut harness);
    assert!(KeyClaim::claimed(&harness.ctx, &delete));
    // The focus goes to the host's pencil toggle; a stroke brings it back.
    let id = harness.state().id;
    harness.ctx.memory_mut(|m| m.surrender_focus(id));
    harness.state_mut().pencil = true;
    harness.run();
    let at = harness.state().at([100.0, 100.0]);
    click_with(&mut harness, at, PointerButton::Primary, Modifiers::NONE);
    assert!(graph_focused(&harness));
    assert!(!KeyClaim::claimed(&harness.ctx, &delete));
}

#[test]
fn percent_fields_show_whole_percents_and_take_them() {
    let mut harness = harness(Host { percent: true, ..Host::new(&THREE) }, Direction::Ltr);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    release(&mut harness, at);
    let points: Vec<_> = graph(&harness).children().filter(|n| n.accesskit_node().role() == Role::GraphicsObject).collect();
    assert_eq!(points[1].value().as_deref(), Some("Input 50%, Output 50%"));
    let field = |harness: &Harness<'_, Host>, name: &str| {
        let node = harness.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some(name));
        let data = node.accesskit_node().data().clone();
        (data.numeric_value(), data.min_numeric_value(), data.max_numeric_value(), node.rect().center())
    };
    let (value, min, max, _) = field(&harness, "Output");
    assert_eq!((value, min, max), (Some(50.0), Some(0.0), Some(100.0)));
    let (value, ..) = field(&harness, "Input");
    assert_eq!(value, Some(50.0));
    harness.state_mut().clear();
    let input = field(&harness, "Input").3;
    click_with(&mut harness, input, PointerButton::Primary, Modifiers::NONE);
    harness.event(Event::Text("25".into()));
    harness.run();
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().points()[1], [64.0, 128.0], "25% of 255 levels, rounded");
}

fn pencil_rig() -> Harness<'static, Host> {
    let mut host = Host::new(&[]);
    host.curve = Curve::Map((0..256).map(|k| k as f32).collect());
    host.pencil = true;
    harness(host, Direction::Ltr)
}

/// A pencil stroke pressed during the typing run of an app's field of the same curve waits for
/// the run's commit, whether the field is drawn before or after the editor, and begins in the pass
/// after it from the curve the run left.
#[test]
fn a_pencil_stroke_during_an_outside_fields_typing_run_begins_the_pass_after_its_commit() {
    for first in [false, true] {
        let mut host = Host::new(&[]);
        host.curve = Curve::Map((0..256).map(|k| k as f32).collect());
        host.pencil = true;
        host.outside = Some(first);
        let mut harness = harness(host, Direction::Ltr);
        let field = harness.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Midtone")).rect().center();
        click_with(&mut harness, field, PointerButton::Primary, Modifiers::NONE);
        harness.event(Event::Text("40".into()));
        harness.run();
        assert!(harness.state().events.is_empty(), "field first {first}");

        let from = harness.state().at([20.0, 200.0]);
        press(&mut harness, from);
        let to = harness.state().at([60.0, 200.0]);
        move_to(&mut harness, to);
        release(&mut harness, to);
        let commit = harness.state().outside_events.iter().find(|(kind, _)| *kind == "Commit").map(|(_, pass)| *pass);
        assert!(commit.is_some(), "field first {first}: {:?}", harness.state().outside_events);
        assert_eq!(Some(harness.state().pass_of("Begin", 0)), commit.map(|pass| pass + 1), "field first {first}");
        assert_eq!(harness.state().kinds().last(), Some(&"Commit"), "field first {first}");
        let map = harness.state().drawn();
        assert_eq!((map[20], map[60], map[128]), (200.0, 200.0, 40.0), "field first {first}: the stroke keeps the typed value");
    }
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

/// A drawn curve holding a NaN level (the host's sampler gave none there) is still the value the
/// stroke reported: the stroke is one gesture to its Commit, not an outside change, and the
/// level stays as it was.
#[test]
fn a_pencil_stroke_on_a_curve_with_a_nan_level_is_not_an_outside_change() {
    let mut harness = pencil_rig();
    if let Curve::Map(map) = &mut harness.state_mut().curve {
        map[200] = f32::NAN;
    }
    harness.run();
    let from = harness.state().at([20.0, 200.0]);
    press(&mut harness, from);
    for x in [60.0, 100.0] {
        let at = harness.state().at([x, 100.0]);
        move_to(&mut harness, at);
    }
    let to = harness.state().at([100.0, 100.0]);
    release(&mut harness, to);
    let kinds = harness.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "{kinds:?}");
    let map = harness.state().drawn();
    assert_eq!((map[20], map[100]), (200.0, 100.0));
    assert!(map[200].is_nan());
}

#[test]
fn shift_press_continues_in_a_straight_line_from_the_previous_stroke() {
    let mut harness = pencil_rig();
    let start = harness.state().at([20.0, 200.0]);
    press(&mut harness, start);
    release(&mut harness, start);
    harness.state_mut().clear();
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
    harness.state_mut().clear();
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
    let bent = Curve::smooth(vec![[0.0, 0.0], [128.0, 160.0], [255.0, 255.0]]);
    let curves = [bent.clone(), bent.clone(), Curve::identity(255.0), Curve::Map((0..256).map(|k| (k as f32 + 9.0).min(255.0)).collect())];
    let editor = |active: usize| CurveEditor::new("c", "Curve", &curves[active], &numbers).channels(&channels).channel(active).overlays(&curves);
    assert_eq!(editor(0).other_channels(), [1, 3], "the identity green and the active composite are skipped");
    assert_eq!(editor(1).other_channels(), [0, 3]);
    assert!(CurveEditor::new("c", "Curve", &curves[0], &numbers).channels(&channels).other_channels().is_empty(), "no overlays, none drawn");
}

#[test]
fn shift_press_starts_afresh_once_the_previous_stroke_s_curve_is_gone() {
    let identity = Curve::Map((0..256).map(|k| k as f32).collect());
    let stroke_then = |change: &dyn Fn(&mut Harness<'_, Host>)| {
        let mut harness = pencil_rig();
        let start = harness.state().at([20.0, 200.0]);
        press(&mut harness, start);
        release(&mut harness, start);
        change(&mut harness);
        harness.state_mut().clear();
        let to = harness.state().at([120.0, 100.0]);
        move_to(&mut harness, to);
        pointer(&mut harness, to, PointerButton::Primary, true, Modifiers::SHIFT);
        release(&mut harness, to);
        harness.state().drawn()
    };
    // The host resets the curve (an undo, Reset).
    let map = stroke_then(&|h| {
        h.state_mut().curve = identity.clone();
        h.run();
    });
    assert_eq!((map[70], map[120]), (70.0, 100.0), "no line from the old end");
    // Pencil off and on again.
    let map = stroke_then(&|h| {
        for pencil in [false, true] {
            h.state_mut().pencil = pencil;
            h.run();
        }
    });
    assert_eq!(map[120], 100.0);
    assert_ne!(map[70], 150.0, "no line from the old end");
}

/// A `+` and an arrow in one frame during a scrub of the selected point's Input field: the arrow
/// names the point `+` selected, and the selection stays there while the arrow waits.
#[test]
fn a_selection_key_before_a_waiting_arrow_keeps_its_selection() {
    const FOUR: [[f32; 2]; 4] = [[0.0, 0.0], [100.0, 100.0], [200.0, 200.0], [255.0, 255.0]];
    let mut harness = harness(Host::with_undo(&FOUR), Direction::Ltr);
    let at = harness.state().at([100.0, 100.0]);
    press(&mut harness, at);
    release(&mut harness, at);
    assert_eq!(selected(&harness), Some(1));
    harness.state_mut().clear();
    let start = input_field(&harness);
    press(&mut harness, start);
    for dx in [10.0, 20.0] {
        move_to(&mut harness, start + vec2(dx, 0.0));
    }
    let key = |key| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE };
    harness.input_mut().events.extend([key(Key::Plus), key(Key::ArrowUp)]);
    harness.step();
    assert_eq!(harness.state().points()[2][1], 200.0, "the arrow waits");
    assert_eq!(selected(&harness), Some(2), "the selection stays on the point `+` selected");
    release(&mut harness, start + vec2(20.0, 0.0));
    assert_eq!(harness.state().points()[2][1], 201.0, "the arrow moved the point `+` selected");
    assert_eq!(selected(&harness), Some(2));
}

const BENT: [[f32; 2]; 5] = [[0.0, 0.0], [64.0, 40.0], [128.0, 200.0], [192.0, 210.0], [255.0, 255.0]];

fn corner_rig() -> Harness<'static, Host> {
    harness(Host { corners: true, ..Host::new(&BENT) }, Direction::Ltr)
}

/// Selects the point at `q` with a click.
fn select_point(harness: &mut Harness<'_, Host>, q: [f32; 2]) {
    let at = harness.state().at(q);
    press(harness, at);
    release(harness, at);
}

fn corner_box<'a>(harness: &'a Harness<'_, Host>) -> Node<'a> {
    harness.get_by(|n| n.role() == Role::CheckBox && n.label().as_deref() == Some("Corner"))
}

#[test]
fn corner_points_split_the_curve_and_the_corner_checkbox_toggles() {
    let mut harness = corner_rig();
    assert!(corner_box(&harness).accesskit_node().is_disabled(), "no point selected");
    select_point(&mut harness, BENT[2]);
    assert_eq!(corner_box(&harness).accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
    harness.state_mut().events.clear();
    corner_box(&harness).click();
    harness.run();
    assert_eq!(harness.state().curve.corner_flags(), [false, false, true, false, false]);
    assert_eq!(harness.state().points(), BENT, "the points stay where they were");
    assert_eq!(selected(&harness), Some(2));
    assert_eq!(harness.state().kinds(), ["Set"]);
    assert_eq!(corner_box(&harness).accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));

    // The curve is split at the corner into two runs, each the sampler's own curve.
    let samples = Curve::through(&BENT, &harness.state().curve.corner_flags(), |p| Curve::monotone(p, 255.0));
    let (before, after) = (Curve::monotone(&BENT[..=2], 255.0), Curve::monotone(&BENT[2..], 255.0));
    assert!(samples[..128] == before[..128] && samples[128..] == after[128..], "a run on each side of the corner");

    assert_eq!(point_value(&harness, "Point 3 of 5"), "Input 128, Output 200, corner");
    assert_eq!(point_value(&harness, "Point 2 of 5"), "Input 64, Output 40");

    corner_box(&harness).click();
    harness.run();
    assert_eq!(harness.state().curve, Curve::smooth(BENT.to_vec()), "unchecked: smooth again");
    assert_eq!(point_value(&harness, "Point 3 of 5"), "Input 128, Output 200");
}

#[test]
fn without_corners_there_is_no_checkbox() {
    let harness = rig(&BENT);
    assert!(harness.query_all_by(|n| n.role() == Role::CheckBox).next().is_none());
}

#[test]
fn a_corner_dragged_off_the_plot_and_back_keeps_its_flag() {
    let mut harness = corner_rig();
    select_point(&mut harness, BENT[2]);
    corner_box(&harness).click();
    harness.run();
    let at = harness.state().at(BENT[2]);
    press(&mut harness, at);
    let off = harness.state().rect.center() + vec2(0.0, harness.state().rect.height());
    move_to(&mut harness, off);
    assert_eq!(harness.state().points().len(), 4, "dragged off, the point is gone");
    let back = harness.state().at([140.0, 150.0]);
    move_to(&mut harness, back);
    release(&mut harness, back);
    assert_eq!(harness.state().points()[2], [140.0, 150.0]);
    assert_eq!(harness.state().curve.corner_flags(), [false, false, true, false, false]);
}

#[test]
fn a_corner_keeps_its_flag_when_dragged_and_a_new_point_is_smooth() {
    let mut harness = corner_rig();
    select_point(&mut harness, BENT[2]);
    corner_box(&harness).click();
    harness.run();
    let at = harness.state().at(BENT[2]);
    let to = harness.state().at([140.0, 150.0]);
    press(&mut harness, at);
    move_to(&mut harness, to);
    release(&mut harness, to);
    assert_eq!(harness.state().points()[2], [140.0, 150.0]);
    assert_eq!(harness.state().curve.corner_flags(), [false, false, true, false, false]);
    let new = harness.state().at([20.0, 100.0]);
    press(&mut harness, new);
    release(&mut harness, new);
    assert_eq!(harness.state().curve.corner_flags(), [false, false, false, true, false, false], "the flag stays on its point");
}

#[test]
fn outer_width_is_the_width_the_editor_allocates_at_each_size() {
    use crate::theme::scope;
    use craft_theme::Size;
    for size in [Size::Compact, Size::Comfortable] {
        let seen = std::rc::Rc::new(std::cell::Cell::new((0.0_f32, 0.0_f32)));
        let out = seen.clone();
        let mut harness =
            Harness::builder().with_size(vec2(600.0, 600.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(move |ui| {
                scope(
                    ui,
                    |o| o.size = Some(size),
                    |ui| {
                        let numbers = LocaleNumbers::from_tag("en");
                        let curve = Curve::identity(255.0);
                        let width = CurveEditor::outer_width(ui.ctx(), true, 267.0);
                        let shown = ui.scope(|ui| CurveEditor::new("width", "Width", &curve, &numbers).side(width).fields(false).show(ui));
                        out.set((width, shown.response.rect.width()));
                    },
                );
            });
        harness.run();
        let (width, allocated) = seen.get();
        assert_eq!(width, allocated, "{size:?}");
        assert!(width > 267.0, "{size:?}");
    }
}

#[test]
fn the_state_maps_the_corners_of_the_value_area_and_follows_the_pointer() {
    let mut harness = rig(&THREE);
    let ctx = harness.ctx.clone();
    let state = CurveEditor::state(&ctx, harness.state().id).expect("state after a frame");
    let host = harness.state();
    assert_eq!(state.plot, host.rect.shrink(host.pad), "inside the inset");
    assert_eq!(state.screen([0.0, 0.0]), state.plot.left_bottom());
    assert_eq!(state.screen([255.0, 255.0]), state.plot.right_top());
    assert_eq!(state.value_at(state.plot.left_bottom()), [0.0, 0.0]);
    assert_eq!((state.dragged, state.hovered, state.pending_delete), (None, None, false));

    let on = state.screen([128.0, 128.0]);
    move_to(&mut harness, on);
    move_to(&mut harness, on);
    let id = harness.state().id;
    assert_eq!(CurveEditor::state(&ctx, id).and_then(|s| s.hovered), Some(1));
    press(&mut harness, on);
    assert_eq!(CurveEditor::state(&ctx, id).and_then(|s| s.dragged), Some(1));
    let off = pos2(on.x, harness.state().rect.bottom() + 40.0);
    move_to(&mut harness, off);
    move_to(&mut harness, off);
    let held = CurveEditor::state(&ctx, id).expect("state");
    assert_eq!((held.dragged, held.pending_delete), (None, true), "off the graph");
    release(&mut harness, off);
    assert!(!CurveEditor::state(&ctx, id).is_some_and(|s| s.pending_delete));
}

#[test]
fn a_host_row_between_shows_between_the_graph_and_the_fields_and_the_fields_still_edit() {
    let mut host = Host::new(&THREE);
    host.between = true;
    let mut harness = harness(host, Direction::Ltr);
    let at = harness.state().at([128.0, 128.0]);
    press(&mut harness, at);
    release(&mut harness, at);
    let row = harness.state().between_rect.expect("the host row drew");
    assert!(row.top() >= harness.state().rect.bottom(), "below the graph");
    assert!(row.bottom() <= input_field(&harness).y, "above the fields");
    harness.state_mut().clear();
    type_into_input(&mut harness, "100");
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().points(), vec![[0.0, 0.0], [100.0, 128.0], [255.0, 255.0]]);
}

#[test]
fn a_plain_curve_drops_the_bars_and_their_gap_and_the_state_follows() {
    let bars = rig(&THREE);
    let plain = harness(Host { bars: false, ..Host::new(&THREE) }, Direction::Ltr);
    let (with, without) = (bars.state(), plain.state());
    let style = CurveEditor::style(&current(&plain.ctx));
    assert_eq!(without.rect.width() - with.rect.width(), style.bar + style.bar_gap);
    let state = CurveEditor::state(&plain.ctx, without.id).expect("state after a frame");
    assert_eq!(state.plot, without.rect.shrink(without.pad), "the plot is the whole widget less the inset");
    assert_eq!(without.rect.width(), without.rect.height(), "a square");
    assert_eq!(state.screen([0.0, 0.0]), state.plot.left_bottom());
}

#[test]
fn a_plain_curve_outer_width_is_the_side_and_two_insets() {
    let harness = harness(Host::new(&THREE), Direction::Ltr);
    let style = CurveEditor::style(&current(&harness.ctx));
    assert_eq!(CurveEditor::outer_width(&harness.ctx, true, 260.0), 260.0 + style.bar + style.bar_gap + 2.0 * style.pad);
    assert_eq!(CurveEditor::outer_width(&harness.ctx, false, 260.0), 260.0 + 2.0 * style.pad);
}
