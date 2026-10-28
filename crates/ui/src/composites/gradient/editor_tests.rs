//! Gradient editor tests: pointer and key editing through the edit lifecycle, the press that
//! waits for a typing run, colour requests, drops, AccessKit, focus, the direction and the key
//! claim.

use craft_theme::Srgba;
use egui::accesskit::Role;
use egui::{Context, Event, Id, Key, KeyboardShortcut, Modifiers, PointerButton, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::editor::{GradientEditor, GradientEditorStyle, StopDrop, StopRef};
use super::stops::{self, StopRules};
use super::value::{ColorStop, GradientStops, OpacityStop, StopRow, StopRows};
use crate::behavior::KeyClaim;
use crate::controls::NumberField;
use crate::direction::{self, Direction};
use crate::edit::{CancelReason, EditEvent};
use crate::localized::LocaleNumbers;
use crate::testing::a11y;
use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, install};

/// What the test drops on the bar.
#[derive(Debug, PartialEq)]
struct Payload(u32);

/// A host of one gradient that applies every value and restores the start value on a cancel.
struct Host {
    stops: GradientStops,
    before: Option<GradientStops>,
    events: Vec<EditEvent<GradientStops>>,
    rows: StopRows,
    min: usize,
    /// Shown inside `edit::scope`.
    scoped: bool,
    /// A "Location" field of colour stop 1 above the editor, as DesignCraft's panel header.
    field: bool,
    field_events: Vec<EditEvent<f64>>,
    requested: Vec<usize>,
    dropped: Option<(StopDrop, u32)>,
    escape_seen: bool,
    /// The bar's id and rect, the ramp, the style and `line.focus`, from the last frame.
    id: Id,
    rect: Rect,
    ramp: Rect,
    style: Option<GradientEditorStyle>,
    focus: egui::Color32,
}

impl Host {
    fn new(stops: GradientStops) -> Self {
        Self {
            stops,
            before: None,
            events: Vec::new(),
            rows: StopRows::Split,
            min: 2,
            scoped: false,
            field: false,
            field_events: Vec::new(),
            requested: Vec::new(),
            dropped: None,
            escape_seen: false,
            id: Id::NULL,
            rect: Rect::NOTHING,
            ramp: Rect::NOTHING,
            style: None,
            focus: egui::Color32::TRANSPARENT,
        }
    }

    fn show(&mut self, ui: &mut Ui) {
        let numbers = LocaleNumbers::from_tag("en");
        let tokens = Tokens::of(ui);
        let style = GradientEditor::style(&tokens);
        self.focus = tokens.color.line.focus.to_egui();
        if self.field {
            let value = f64::from(self.stops.color[1].location) * 100.0;
            let field = NumberField::new("location", "Location", value, &numbers).range(0.0..=100.0).decimals(0..=0).show(ui);
            if let Some(event) = field.event {
                if let Some(v) = event.value()
                    && let Some((stops, _)) = stops::move_to(&self.stops, StopRow::Color, 1, *v as f32 / 100.0, &StopRules::default())
                {
                    self.stops = stops;
                }
                self.field_events.push(event);
            }
        }
        let stops = self.stops.clone();
        let editor = GradientEditor::new("gradient", "Gradient", &stops, &numbers).rows(self.rows).stops(self.min, 1024).drop_target::<Payload>();
        let response = ui
            .allocate_ui(vec2(300.0, 0.0), |ui| {
                ui.set_max_width(300.0);
                match self.scoped {
                    true => crate::edit::scope(ui, |ui| editor.show(ui)).inner,
                    false => editor.show(ui),
                }
            })
            .inner;
        self.escape_seen |= ui.input(|i| i.key_pressed(Key::Escape));
        let edit = response.edit;
        self.id = edit.response.id;
        self.rect = edit.response.rect;
        let top = match self.rows {
            StopRows::Split => style.pointer + style.marker + 1.0,
            StopRows::Combined => 3.0 * style.diamond + 1.0,
        };
        self.ramp = Rect::from_min_size(self.rect.min + vec2(style.inset, top), vec2(self.rect.width() - 2.0 * style.inset, style.ramp_height));
        self.style = Some(style);
        self.requested.extend(response.color_requested);
        if let Some((place, payload)) = response.dropped {
            self.dropped = Some((place, payload.0));
        }
        if let Some(event) = edit.event {
            match &event {
                EditEvent::Begin { .. } => self.before = Some(self.stops.clone()),
                EditEvent::Cancel { reason, .. } if *reason != CancelReason::External => {
                    if let Some(before) = self.before.take() {
                        self.stops = before;
                    }
                }
                other => {
                    if let Some(value) = other.value() {
                        self.stops = value.clone();
                    }
                }
            }
            self.events.push(event);
        }
    }

    fn style(&self) -> GradientEditorStyle {
        self.style.expect("shown")
    }

    fn x(&self, location: f32) -> f32 {
        self.ramp.left() + location * self.ramp.width()
    }

    /// The middle of the colour stop marker at `location`.
    fn color_marker(&self, location: f32) -> Pos2 {
        let s = self.style();
        pos2(self.x(location), self.ramp.bottom() + s.pointer + s.marker / 2.0)
    }

    /// The middle of the opacity stop marker at `location`.
    fn opacity_marker(&self, location: f32) -> Pos2 {
        let s = self.style();
        pos2(self.x(location), self.ramp.top() - s.pointer - s.marker / 2.0)
    }

    /// The colour midpoint diamond at `location` of a split bar.
    fn diamond(&self, location: f32) -> Pos2 {
        pos2(self.x(location), self.ramp.bottom() + 2.0 * self.style().diamond)
    }

    fn locations(&self) -> Vec<f32> {
        self.stops.color.iter().map(|s| s.location).collect()
    }

    fn kinds(&self) -> Vec<&'static str> {
        kinds(&self.events)
    }
}

fn kinds<T>(events: &[EditEvent<T>]) -> Vec<&'static str> {
    events
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

fn stop(location: f32, tag: u64) -> ColorStop {
    ColorStop { tag, ..ColorStop::new(location, Srgba::new(location, 0.2, 1.0 - location, 1.0)) }
}

/// Three colour stops at 0, 50 and 100 %, tagged 1, 2 and 3.
fn three() -> GradientStops {
    GradientStops::new(vec![stop(0.0, 1), stop(0.5, 2), stop(1.0, 3)])
}

fn harness(host: Host, dir: Direction) -> Harness<'static, Host> {
    let mut harness = Harness::builder()
        .with_size([400.0, 240.0])
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(move |ctx: &Context| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(|ui, host: &mut Host| host.show(ui), host);
    harness.run();
    harness
}

fn rig(stops: GradientStops) -> Harness<'static, Host> {
    harness(Host::new(stops), Direction::Ltr)
}

fn harness_with(host: Host) -> Harness<'static, Host> {
    harness(host, Direction::Ltr)
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

fn click(harness: &mut Harness<'_, Host>, pos: Pos2, button: PointerButton) {
    move_to(harness, pos);
    pointer(harness, pos, button, true, Modifiers::NONE);
    pointer(harness, pos, button, false, Modifiers::NONE);
    harness.run();
}

fn key(harness: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

/// Cmd on a Mac, Ctrl elsewhere, as egui-winit reports them.
const CMD: Modifiers = Modifiers { ctrl: !cfg!(target_os = "macos"), mac_cmd: cfg!(target_os = "macos"), command: true, alt: false, shift: false };

fn bar<'a>(harness: &'a Harness<'_, Host>) -> Node<'a> {
    harness.get_by(|n| n.role() == Role::Group && n.label().as_deref() == Some("Gradient"))
}

fn markers<'a>(harness: &'a Harness<'_, Host>) -> Vec<Node<'a>> {
    bar(harness).children().filter(|n| n.accesskit_node().role() == Role::GraphicsObject).collect()
}

fn bar_focused(harness: &Harness<'_, Host>) -> bool {
    harness.ctx.memory(|m| m.has_focus(harness.state().id))
}

/// Focuses the bar from the keyboard, without a change.
fn focus(harness: &mut Harness<'_, Host>) {
    harness.key_press(Key::Tab);
    harness.run();
    assert!(bar_focused(harness), "Tab reaches the bar first");
}

/// Gives the bar keyboard focus whatever has it.
fn focus_from_scratch(harness: &mut Harness<'_, Host>) {
    let id = harness.state().id;
    harness.ctx.memory_mut(|m| m.surrender_focus(id));
    harness.run();
    focus(harness);
}

fn selected(harness: &Harness<'_, Host>) -> Option<StopRef> {
    GradientEditor::selected(&harness.ctx, harness.state().id)
}

fn select(harness: &mut Harness<'_, Host>, marker: Option<StopRef>) {
    GradientEditor::select(&harness.ctx, harness.state().id, marker);
    harness.run();
}

#[test]
fn a_press_selects_a_stop_in_its_frame_and_a_drag_past_a_neighbour_is_one_gesture_the_selection_follows() {
    let mut harness = rig(three());
    select(&mut harness, None);
    let at = harness.state().color_marker(0.0) + vec2(3.0, 0.0);
    press(&mut harness, at);
    assert_eq!(selected(&harness), Some(StopRef::Color(0)), "selected in the press frame");
    assert_eq!(harness.state().kinds(), ["Begin"]);
    for to in [0.3, 0.6, 0.7] {
        let at = harness.state().color_marker(to) + vec2(3.0, 0.0);
        move_to(&mut harness, at);
    }
    let at = harness.state().color_marker(0.7) + vec2(3.0, 0.0);
    release(&mut harness, at);
    let kinds = harness.state().kinds();
    assert_eq!(kinds.last(), Some(&"Commit"), "{kinds:?}");
    assert!(kinds[1..kinds.len() - 1].iter().all(|k| *k == "Update"), "{kinds:?}");
    assert_eq!(harness.state().locations(), [0.5, 0.7, 1.0], "the grab offset keeps the stop under the pointer");
    let tags: Vec<_> = harness.state().stops.color.iter().map(|s| s.tag).collect();
    assert_eq!(tags, [2, 1, 3], "the tag travels with the stop");
    assert_eq!(selected(&harness), Some(StopRef::Color(1)));

    // A press without movement commits unchanged.
    harness.state_mut().events.clear();
    let at = harness.state().color_marker(1.0) + vec2(-2.0, 1.0);
    press(&mut harness, at);
    release(&mut harness, at);
    assert_eq!(harness.state().kinds(), ["Begin", "Commit unchanged"]);
    assert_eq!(selected(&harness), Some(StopRef::Color(2)));
}

#[test]
fn a_stop_torn_off_goes_and_comes_back_within_one_gesture_and_stays_at_the_minimum() {
    let mut harness = rig(three());
    let at = harness.state().color_marker(0.5);
    press(&mut harness, at);
    let tear_off = harness.state().style().tear_off;
    let off = pos2(at.x, harness.state().ramp.bottom() + tear_off + 4.0);
    move_to(&mut harness, off);
    move_to(&mut harness, off);
    assert_eq!(harness.state().locations(), [0.0, 1.0], "the torn-off stop is gone while off");
    move_to(&mut harness, at);
    move_to(&mut harness, at);
    assert_eq!(harness.state().stops, three(), "back on the bar it is restored");
    move_to(&mut harness, off);
    release(&mut harness, off);
    let kinds = harness.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "{kinds:?}");
    assert!(!kinds.contains(&"Set"), "{kinds:?}");
    assert_eq!(harness.state().locations(), [0.0, 1.0]);

    // Two stops are the minimum: dragged off, the stop stays.
    let at = harness.state().color_marker(1.0);
    press(&mut harness, at);
    let off = pos2(at.x - 30.0, harness.state().ramp.bottom() + tear_off + 4.0);
    move_to(&mut harness, off);
    release(&mut harness, off);
    assert_eq!(harness.state().stops.color.len(), 2);
}

#[test]
fn an_alt_drag_moves_a_copy_with_the_stop_s_tag_that_cant_be_torn_off() {
    let mut harness = rig(three());
    let at = harness.state().color_marker(0.5);
    move_to(&mut harness, at);
    pointer(&mut harness, at, PointerButton::Primary, true, Modifiers::ALT);
    let to = harness.state().color_marker(0.75);
    move_to(&mut harness, to);
    let off = pos2(to.x, harness.state().ramp.bottom() + harness.state().style().tear_off + 4.0);
    move_to(&mut harness, off);
    release(&mut harness, off);
    let kinds = harness.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "{kinds:?}");
    assert_eq!(harness.state().locations(), [0.0, 0.5, 0.75, 1.0]);
    assert_eq!(harness.state().stops.color[2].tag, 2);
    assert_eq!(selected(&harness), Some(StopRef::Color(2)), "the copy is selected");
}

#[test]
fn a_click_adds_a_stop_in_the_row_by_where_it_is_and_a_drag_on_empty_space_does_nothing() {
    let mut harness = rig(three());
    let ramp = harness.state().ramp;
    let above = pos2(harness.state().x(0.25), ramp.top() + 3.0);
    click(&mut harness, above, PointerButton::Primary);
    assert_eq!(harness.state().kinds(), ["Set"]);
    let opacity: Vec<_> = harness.state().stops.opacity.iter().map(|s| (s.location, s.opacity)).collect();
    assert_eq!(opacity, [(0.0, 1.0), (0.25, 1.0), (1.0, 1.0)], "the implicit stops are written out");
    assert_eq!(selected(&harness), Some(StopRef::Opacity(1)));

    let below = pos2(harness.state().x(0.75), ramp.bottom() - 3.0);
    click(&mut harness, below, PointerButton::Primary);
    assert_eq!(harness.state().kinds(), ["Set", "Set"]);
    assert_eq!(harness.state().locations(), [0.0, 0.5, 0.75, 1.0]);
    assert_eq!(selected(&harness), Some(StopRef::Color(2)));

    let from = pos2(harness.state().x(0.1), ramp.center().y);
    press(&mut harness, from);
    move_to(&mut harness, from + vec2(40.0, 0.0));
    release(&mut harness, from + vec2(40.0, 0.0));
    assert_eq!(harness.state().kinds(), ["Set", "Set"], "a drag on empty space does nothing");

    let mut harness = harness_with(Host { rows: StopRows::Combined, ..Host::new(three()) });
    let above = pos2(harness.state().x(0.25), harness.state().ramp.top() + 3.0);
    click(&mut harness, above, PointerButton::Primary);
    assert_eq!(harness.state().locations(), [0.0, 0.25, 0.5, 1.0], "one row: a colour stop");
    assert!(harness.state().stops.opacity.is_empty());
}

#[test]
fn esc_during_a_drag_cancels_it_and_the_start_comes_back() {
    let mut harness = rig(three());
    let at = harness.state().color_marker(0.5);
    press(&mut harness, at);
    let to = harness.state().color_marker(0.8);
    move_to(&mut harness, to);
    move_to(&mut harness, to);
    assert_eq!(harness.state().locations()[1], 0.8);
    harness.key_press(Key::Escape);
    harness.step();
    assert_eq!(harness.state().kinds().last(), Some(&"Cancel"));
    let to = harness.state().color_marker(0.9);
    move_to(&mut harness, to);
    release(&mut harness, to);
    assert_eq!(harness.state().stops, three());
    assert_eq!(harness.state().kinds(), ["Begin", "Update", "Cancel"], "the rest of the drag does nothing");
    assert!(!harness.state().escape_seen, "the press is consumed");

    focus_from_scratch(&mut harness);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().escape_seen, "without a drag Esc is the host's");
}

#[test]
fn keys_move_delete_and_add_stops_one_set_each() {
    let mut harness = rig(three());
    focus(&mut harness);
    select(&mut harness, Some(StopRef::Color(1)));
    key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    key(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
    key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(harness.state().locations(), [0.0, 0.6, 1.0]);
    assert_eq!(harness.state().kinds(), ["Set", "Set", "Set"]);
    assert!(bar_focused(&harness), "arrows keep the focus");

    key(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(harness.state().locations(), [0.0, 1.0]);
    assert_eq!(selected(&harness), Some(StopRef::Color(1)), "the stop that takes its index");
    key(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(harness.state().locations(), [0.0, 1.0], "not below the minimum");

    key(&mut harness, Modifiers::NONE, Key::Insert);
    assert_eq!(harness.state().locations(), [0.0, 0.5, 1.0]);
    assert_eq!(selected(&harness), Some(StopRef::Color(1)));
    key(&mut harness, CMD, Key::Enter);
    assert_eq!(harness.state().locations(), [0.0, 0.5, 0.75, 1.0]);
    assert_eq!(selected(&harness), Some(StopRef::Color(2)));
    assert_eq!(harness.state().kinds(), ["Set"; 6]);

    for (modifiers, k) in
        [(Modifiers::SHIFT, Key::Insert), (Modifiers::ALT, Key::ArrowRight), (CMD, Key::ArrowRight), (Modifiers::SHIFT, Key::Delete)]
    {
        key(&mut harness, modifiers, k);
    }
    assert_eq!(harness.state().kinds().len(), 6, "Shift+Insert, Alt and Cmd/Ctrl arrows and Shift+Delete are the app's");
}

#[test]
fn plus_and_minus_walk_the_row_with_its_midpoints_and_up_and_down_change_rows() {
    use StopRef::{Color, Midpoint, Opacity};
    let mut harness = rig(three());
    focus(&mut harness);
    assert_eq!(selected(&harness), Some(Color(0)), "the first colour stop by default");
    let mut walk = Vec::new();
    for k in [Key::Plus, Key::Equals, Key::Minus, Key::Minus, Key::Minus] {
        key(&mut harness, Modifiers::NONE, k);
        walk.push(selected(&harness).unwrap());
    }
    assert_eq!(walk, [Midpoint(StopRow::Color, 0), Color(1), Midpoint(StopRow::Color, 0), Color(0), Color(0)]);
    select(&mut harness, None);
    key(&mut harness, Modifiers::NONE, Key::Minus);
    assert_eq!(selected(&harness), Some(Color(2)), "the last with nothing selected");
    key(&mut harness, Modifiers::NONE, Key::Plus);
    assert_eq!(selected(&harness), Some(Color(2)), "it stops at the end");

    key(&mut harness, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(selected(&harness), Some(Opacity(1)), "the opacity stop nearest 100 %");
    key(&mut harness, Modifiers::NONE, Key::Minus);
    assert_eq!(selected(&harness), Some(Midpoint(StopRow::Opacity, 0)));
    key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(selected(&harness), Some(Color(1)), "the colour stop nearest the midpoint at 50 %");
    assert!(harness.state().events.is_empty());
    assert!(bar_focused(&harness));
}

#[test]
fn a_midpoint_drags_and_nudges_within_the_midpoint_range() {
    let mut harness = rig(three());
    let at = harness.state().diamond(0.25);
    press(&mut harness, at);
    assert_eq!(selected(&harness), Some(StopRef::Midpoint(StopRow::Color, 0)));
    let to = harness.state().diamond(0.0);
    move_to(&mut harness, to);
    release(&mut harness, to);
    assert_eq!(harness.state().kinds().last(), Some(&"Commit"));
    assert_eq!(harness.state().stops.color[0].midpoint, 0.05, "PhotoCraft's range");
    focus_from_scratch(&mut harness);
    key(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
    assert!((harness.state().stops.color[0].midpoint - 0.15).abs() < 1e-5);
    assert_eq!(harness.state().locations(), [0.0, 0.5, 1.0]);
}

#[test]
fn a_press_during_a_typing_run_waits_for_its_commit_and_edits_the_stops_it_leaves() {
    let mut harness = harness_with(Host { field: true, ..Host::new(three()) });
    let field = harness.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Location")).rect().center();
    click(&mut harness, field, PointerButton::Primary);
    harness.event(Event::Text("30".into()));
    harness.run();
    assert_eq!(kinds(&harness.state().field_events), ["Begin", "Update"]);

    let at = harness.state().color_marker(1.0);
    press(&mut harness, at);
    let to = harness.state().color_marker(0.9);
    move_to(&mut harness, to);
    release(&mut harness, to);
    assert_eq!(kinds(&harness.state().field_events), ["Begin", "Update", "Commit"]);
    let kinds = harness.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "{kinds:?}");
    assert!(!kinds.contains(&"Cancel"), "the drag starts from the typed value: {kinds:?}");
    assert_eq!(harness.state().locations(), [0.0, 0.3, 0.9]);
}

#[test]
fn inside_a_dialog_scope_the_editor_reports_updates_only() {
    let mut harness = harness_with(Host { scoped: true, ..Host::new(three()) });
    let at = harness.state().color_marker(0.5);
    press(&mut harness, at);
    let to = harness.state().color_marker(0.6);
    move_to(&mut harness, to);
    release(&mut harness, to);
    let below = pos2(harness.state().x(0.8), harness.state().ramp.bottom() - 3.0);
    click(&mut harness, below, PointerButton::Primary);
    assert!(harness.state().events.len() >= 2);
    assert!(harness.state().kinds().iter().all(|k| *k == "Update"), "{:?}", harness.state().kinds());
    assert_eq!(harness.state().locations(), [0.0, 0.6, 0.8, 1.0]);
}

#[test]
fn a_double_click_or_enter_on_a_color_stop_asks_for_its_color() {
    let mut harness = rig(three());
    let at = harness.state().color_marker(0.5);
    move_to(&mut harness, at);
    // One frame per event: the second click has to follow the first within egui's window.
    for _ in 0..2 {
        pointer(&mut harness, at, PointerButton::Primary, true, Modifiers::NONE);
        pointer(&mut harness, at, PointerButton::Primary, false, Modifiers::NONE);
    }
    harness.run();
    assert_eq!(harness.state().requested, [1]);
    assert_eq!(harness.state().stops, three());

    focus_from_scratch(&mut harness);
    select(&mut harness, Some(StopRef::Color(2)));
    key(&mut harness, Modifiers::NONE, Key::Enter);
    key(&mut harness, Modifiers::NONE, Key::Space);
    assert_eq!(harness.state().requested, [1, 2, 2]);
    select(&mut harness, Some(StopRef::Opacity(0)));
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().requested, [1, 2, 2], "an opacity stop has no colour");

    // A right-click selects a marker and changes nothing.
    harness.state_mut().events.clear();
    let at = harness.state().opacity_marker(1.0);
    click(&mut harness, at, PointerButton::Secondary);
    assert_eq!(selected(&harness), Some(StopRef::Opacity(1)));
    assert!(harness.state().events.is_empty());
}

#[test]
fn a_drop_reports_the_stop_or_the_location_it_lands_on() {
    let mut harness = rig(three());
    for (at, place) in [(0.5, StopDrop::OnStop(1)), (0.25, StopDrop::At(0.25))] {
        let pos = pos2(harness.state().x(at), harness.state().ramp.center().y);
        egui::DragAndDrop::set_payload(&harness.ctx, Payload(7));
        move_to(&mut harness, pos);
        harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
        harness.run();
        assert_eq!(harness.state().dropped, Some((place, 7)));
        harness.state_mut().dropped = None;
    }
    assert!(harness.state().events.is_empty(), "the host applies a drop");
}

#[test]
fn the_bar_and_its_markers_are_in_the_accesskit_tree() {
    let mut stops = three();
    stops.opacity = vec![OpacityStop::new(0.0, 1.0), OpacityStop::new(1.0, 0.4)];
    let mut harness = rig(stops);
    let at = harness.state().color_marker(0.5);
    click(&mut harness, at, PointerButton::Primary);
    let bar = bar(&harness);
    assert_eq!(bar.value().as_deref(), Some("3 color stops, 2 opacity stops"));
    assert!(bar.accesskit_node().data().supports_action(egui::accesskit::Action::Focus));
    let nodes = markers(&harness);
    let names: Vec<_> = nodes.iter().map(|n| n.accesskit_node().label().unwrap_or_default()).collect();
    assert_eq!(
        names,
        [
            "Opacity stop 1 of 2",
            "Opacity midpoint 1",
            "Opacity stop 2 of 2",
            "Color stop 1 of 3",
            "Midpoint 1",
            "Color stop 2 of 3",
            "Midpoint 2",
            "Color stop 3 of 3"
        ]
    );
    assert_eq!(nodes[2].value().as_deref(), Some("100%, opacity 40%"));
    assert_eq!(nodes[5].value().as_deref(), Some("50%, #803380"));
    assert_eq!(nodes[4].value().as_deref(), Some("50%"));
    let selected: Vec<_> = nodes.iter().map(|n| n.accesskit_node().data().is_selected()).collect();
    assert_eq!(selected.iter().filter(|s| **s == Some(true)).count(), 1);
    assert_eq!(selected[5], Some(true));
    assert_eq!(bar.accesskit_node().data().active_descendant(), Some(nodes[5].accesskit_node().locate().0));

    let harness = harness_with(Host { rows: StopRows::Combined, ..Host::new(three()) });
    assert_eq!(self::bar(&harness).value().as_deref(), Some("3 stops"));
    assert_eq!(markers(&harness)[0].value().as_deref(), Some("0%, #0033FF, opacity 100%"));
}

#[test]
fn the_focus_ring_shows_for_keyboard_focus_only() {
    let mut harness = rig(three());
    let at = harness.state().color_marker(0.5);
    click(&mut harness, at, PointerButton::Primary);
    select(&mut harness, None);
    assert!(bar_focused(&harness));
    let line_focus = harness.state().focus;
    let rings = a11y::focus_rings(&harness).map(|(_, rings)| rings).unwrap_or_default();
    assert!(rings.iter().all(|r| r.stroke.color != line_focus), "no ring after a pointer press: {rings:?}");
    focus_from_scratch(&mut harness);
    a11y::assert_focus_ring(&harness, line_focus);
}

#[test]
fn right_to_left_keeps_the_bar_geometry() {
    let offsets = |dir| {
        let harness = harness(Host::new(three()), dir);
        let origin = bar(&harness).rect().min;
        markers(&harness).iter().map(|n| n.rect().center() - origin).collect::<Vec<_>>()
    };
    let ltr = offsets(Direction::Ltr);
    assert!(ltr[0].x < ltr.last().unwrap().x, "location 0 at the left: {ltr:?}");
    assert_eq!(offsets(Direction::Rtl), ltr);

    let mut harness = harness(Host::new(three()), Direction::Rtl);
    focus(&mut harness);
    select(&mut harness, Some(StopRef::Color(1)));
    key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(harness.state().locations(), [0.0, 0.51, 1.0], "Right moves as drawn");
}

#[test]
fn the_bar_claims_its_keys_only_while_focused() {
    let mut harness = rig(three());
    let delete = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
    assert!(!KeyClaim::claimed(&harness.ctx, &delete));
    focus(&mut harness);
    let claimed = |harness: &Harness<'_, Host>, modifiers, key| KeyClaim::claimed(&harness.ctx, &KeyboardShortcut::new(modifiers, key));
    for k in [Key::Delete, Key::Backspace, Key::Plus, Key::Equals, Key::Minus, Key::Insert] {
        assert!(claimed(&harness, Modifiers::NONE, k), "{k:?}");
    }
    assert!(claimed(&harness, CMD, Key::Enter));
    assert!(claimed(&harness, Modifiers::SHIFT, Key::Plus), "+ is Shift+= on many keyboards");
    assert!(!claimed(&harness, Modifiers::SHIFT, Key::Insert), "Shift+Insert (paste) stays the app's");
    assert!(!claimed(&harness, CMD, Key::Minus));
    let id = harness.state().id;
    harness.ctx.memory_mut(|m| m.surrender_focus(id));
    harness.run();
    assert!(!KeyClaim::claimed(&harness.ctx, &delete));
}

#[test]
fn the_editor_stories_name_every_control() {
    for story in crate::story::registry().into_iter().filter(|s| s.id.starts_with("composites/gradient/editor")) {
        let mut args = (story.args)();
        let mut harness = Harness::builder()
            .with_size([520.0, 240.0])
            .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
            .build_ui(move |ui| (story.show)(ui, &mut args));
        harness.run();
        a11y::assert_labelled(&harness);
        assert!(harness.query_all_by(|n| n.role() == Role::GraphicsObject).count() >= 3, "{}: the markers are in the tree", story.id);
    }
}
