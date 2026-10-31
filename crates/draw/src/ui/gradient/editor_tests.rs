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
use craft_ui::behavior::KeyClaim;
use craft_ui::controls::NumberField;
use craft_ui::direction::{self, Direction};
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::localized::LocaleNumbers;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
use craft_ui::testing::a11y;
use craft_ui::theme::{Base, ToEgui as _, Tokens, TokensOf as _, install};

/// What the test drops on the bar.
#[derive(Debug, PartialEq)]
struct Payload(u32);

/// A host of one gradient that applies every value and restores the start value on a cancel.
struct Host {
    stops: GradientStops,
    before: Option<GradientStops>,
    events: Vec<EditEvent<GradientStops>>,
    /// The egui pass in which each of `events` and `field_events` was reported.
    passes: Vec<u64>,
    field_passes: Vec<u64>,
    rows: StopRows,
    min: usize,
    /// Shown inside `edit::scope`.
    scoped: bool,
    /// A "Location" field of colour stop 1 above the editor, as DesignCraft's panel header.
    field: bool,
    field_events: Vec<EditEvent<f64>>,
    /// The editor's own fields row, and its colour popover.
    stop_fields: bool,
    popover: bool,
    location_decimals: usize,
    opacity_midpoints: bool,
    /// The editor isn't drawn.
    hidden: bool,
    /// The popover's picker gets an eyedropper through `color_picker`; the editor's last report
    /// of it, and of the picker's id.
    eyedropper: bool,
    eyedropper_requested: bool,
    picker_id: Option<Id>,
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
            passes: Vec::new(),
            field_passes: Vec::new(),
            rows: StopRows::Split,
            min: 2,
            scoped: false,
            field: false,
            field_events: Vec::new(),
            stop_fields: true,
            popover: false,
            location_decimals: 0,
            opacity_midpoints: true,
            hidden: false,
            eyedropper: false,
            eyedropper_requested: false,
            picker_id: None,
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
        if self.hidden {
            return;
        }
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
                self.field_passes.push(ui.ctx().cumulative_pass_nr());
                self.field_events.push(event);
            }
        }
        let stops = self.stops.clone();
        let editor = GradientEditor::new("gradient", "Gradient", &stops, &numbers)
            .rows(self.rows)
            .stops(self.min, 1024)
            .fields(self.stop_fields)
            .color_popover(self.popover)
            .location_decimals(self.location_decimals)
            .opacity_midpoints(self.opacity_midpoints)
            .drop_target::<Payload>();
        let editor = match self.eyedropper {
            true => editor.color_picker(|picker| picker.eyedropper("Pick screen color")),
            false => editor,
        };
        let response = ui
            .allocate_ui(vec2(300.0, 0.0), |ui| {
                ui.set_max_width(300.0);
                match self.scoped {
                    true => craft_ui::edit::scope(ui, |ui| editor.show(ui)).inner,
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
            StopRows::Combined | StopRows::ColorOnly => 3.0 * style.diamond + 1.0,
        };
        self.ramp = Rect::from_min_size(self.rect.min + vec2(style.inset, top), vec2(self.rect.width() - 2.0 * style.inset, style.ramp_height));
        self.style = Some(style);
        self.requested.extend(response.color_requested);
        self.eyedropper_requested |= response.eyedropper_requested;
        self.picker_id = response.picker_id;
        if let Some((place, payload)) = response.dropped {
            self.dropped = Some((place, payload.0));
        }
        if let Some(event) = edit.event {
            match &event {
                EditEvent::Begin { value, .. } => {
                    self.before = Some(self.stops.clone());
                    if let Some(value) = value {
                        self.stops = value.clone();
                    }
                }
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
            self.passes.push(ui.ctx().cumulative_pass_nr());
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
            _ => "other",
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
    harness_sized(host, dir, [400.0, 240.0])
}

fn harness_sized(host: Host, dir: Direction, size: [f32; 2]) -> Harness<'static, Host> {
    let mut harness = Harness::builder()
        .with_size(size)
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
    let mut harness = harness_with(Host { field: true, stop_fields: false, ..Host::new(three()) });
    let field = harness.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Location")).rect().center();
    click(&mut harness, field, PointerButton::Primary);
    harness.event(Event::Text("30".into()));
    harness.run();
    assert_eq!(kinds(&harness.state().field_events), ["Begin"]);

    let at = harness.state().color_marker(1.0);
    press(&mut harness, at);
    let to = harness.state().color_marker(0.9);
    move_to(&mut harness, to);
    release(&mut harness, to);
    assert_eq!(kinds(&harness.state().field_events), ["Begin", "Commit"]);
    assert_eq!(harness.state().passes[0], harness.state().field_passes[1] + 1, "the drag begins in the pass after the run's commit");
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
fn colour_only_midpoints_leave_the_opacity_row_without_diamonds_or_focus_stops() {
    use StopRef::{Color, Opacity};
    let mut stops = three();
    stops.opacity = vec![OpacityStop::new(0.0, 1.0), OpacityStop::new(1.0, 0.4)];
    let mut host = Host::new(stops);
    host.opacity_midpoints = false;
    let mut harness = harness_with(host);
    let names: Vec<_> = markers(&harness).iter().map(|n| n.accesskit_node().label().unwrap_or_default()).collect();
    assert_eq!(
        names,
        ["Opacity stop 1 of 2", "Opacity stop 2 of 2", "Color stop 1 of 3", "Midpoint 1", "Color stop 2 of 3", "Midpoint 2", "Color stop 3 of 3"]
    );
    focus(&mut harness);
    select(&mut harness, Some(Opacity(0)));
    key(&mut harness, Modifiers::NONE, Key::Plus);
    assert_eq!(selected(&harness), Some(Opacity(1)), "Plus skips the absent opacity midpoint");
    select(&mut harness, Some(Color(0)));
    key(&mut harness, Modifiers::NONE, Key::Plus);
    assert_eq!(selected(&harness), Some(StopRef::Midpoint(StopRow::Color, 0)), "the colour row keeps its midpoints");
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

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
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

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn the_editor_stories_name_every_control() {
    for story in
        craft_ui::story::registry().into_iter().filter(|s| s.id.starts_with("composites/gradient/editor") || s.id == "composites/gradient/dialog")
    {
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

fn spin<'a>(harness: &'a Harness<'_, Host>, name: &str) -> Node<'a> {
    let name = name.to_owned();
    harness.get_by(move |n| n.role() == Role::SpinButton && n.label().as_deref() == Some(name.as_str()))
}

fn button<'a>(harness: &'a Harness<'_, Host>, name: &str) -> Node<'a> {
    let name = name.to_owned();
    harness.get_by(move |n| n.role() == Role::Button && n.label().as_deref() == Some(name.as_str()))
}

fn color_well<'a>(harness: &'a Harness<'_, Host>) -> Node<'a> {
    harness.get_by(|n| n.role() == Role::ColorWell && n.label().as_deref() == Some("Color"))
}

/// Types `text` into the field named `name` and commits it with Enter.
fn type_into(harness: &mut Harness<'_, Host>, name: &str, text: &str) {
    let at = spin(harness, name).rect().center();
    click(harness, at, PointerButton::Primary);
    harness.event(Event::Text(text.into()));
    harness.run();
}

#[test]
fn the_fields_edit_the_selected_marker_one_gesture_per_run_and_the_selection_follows_the_stop() {
    let mut harness = rig(three());
    let names: Vec<_> = harness
        .query_all_by(|n| matches!(n.role(), Role::SpinButton | Role::Button | Role::ColorWell))
        .filter_map(|n| n.accesskit_node().label())
        .filter(|l| ["Color", "Opacity", "Location", "Midpoint", "Delete stop"].contains(&l.as_str()))
        .collect();
    assert_eq!(names, ["Color", "Location", "Delete stop"], "a colour stop of a split bar");
    assert_eq!(color_well(&harness).value().as_deref(), Some("#0033ff"));

    // Location past a neighbour: one gesture, and the selection stays on the stop (tag 1).
    type_into(&mut harness, "Location", "70");
    assert_eq!(selected(&harness), Some(StopRef::Color(1)));
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().kinds().first(), Some(&"Begin"));
    assert_eq!(harness.state().kinds().last(), Some(&"Commit"));
    assert_eq!(harness.state().kinds().iter().filter(|k| **k == "Begin").count(), 1);
    assert_eq!(harness.state().locations(), [0.5, 0.7, 1.0]);
    assert_eq!(harness.state().stops.color[1].tag, 1);

    // Esc cancels a run, and the selection goes back with the stop.
    harness.state_mut().events.clear();
    type_into(&mut harness, "Location", "20");
    assert_eq!((harness.state().locations(), selected(&harness)), (vec![0.2, 0.5, 1.0], Some(StopRef::Color(0))));
    key(&mut harness, Modifiers::NONE, Key::Escape);
    assert_eq!(harness.state().kinds().last(), Some(&"Cancel"));
    assert_eq!((harness.state().locations(), selected(&harness)), (vec![0.5, 0.7, 1.0], Some(StopRef::Color(1))));

    // An opacity stop's Opacity: a scrub is one gesture, and writes out the implicit stops.
    harness.state_mut().events.clear();
    select(&mut harness, Some(StopRef::Opacity(0)));
    assert!(harness.query_by(|n| n.role() == Role::ColorWell && n.label().as_deref() == Some("Color")).is_none());
    let field = spin(&harness, "Opacity").rect().center();
    press(&mut harness, field);
    for dx in [5.0, 10.0, 20.0] {
        move_to(&mut harness, field - vec2(dx, 0.0));
    }
    release(&mut harness, field - vec2(20.0, 0.0));
    assert_eq!(harness.state().kinds().first(), Some(&"Begin"));
    assert_eq!(harness.state().kinds().last(), Some(&"Commit"));
    let opacity = &harness.state().stops.opacity;
    assert_eq!(opacity.len(), 2);
    assert!(opacity[0].opacity < 1.0 && opacity[1].opacity == 1.0, "{opacity:?}");

    // A midpoint's Midpoint, clamped to the host's range.
    select(&mut harness, Some(StopRef::Midpoint(StopRow::Color, 0)));
    assert_eq!(spin(&harness, "Midpoint").value().as_deref(), Some("50%"));
    type_into(&mut harness, "Midpoint", "99");
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().stops.color[0].midpoint, 0.95);
    assert!(harness.query_by(|n| n.label().as_deref() == Some("Delete stop")).is_none(), "a midpoint has no Delete");

    // Combined: a colour stop's opacity is its alpha.
    let mut harness = harness_with(Host { rows: StopRows::Combined, ..Host::new(three()) });
    type_into(&mut harness, "Opacity", "25");
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().stops.color[0].color.a, 0.25);
    assert!(harness.state().stops.opacity.is_empty());
}

/// An arrow on the bar during a scrub of the selected stop's Opacity field waits for the release
/// and moves only the stop: the bar's stops from the arrow's pass don't bring back the opacity
/// the scrub moved on from. The scrub's update in the arrow's pass doesn't carry the arrow's move:
/// the fields draw from the editor's shown stops, not the bar's.
#[test]
fn an_arrow_on_the_bar_during_a_field_scrub_keeps_the_scrubs_value() {
    let mut harness = harness_with(Host { rows: StopRows::Combined, ..Host::new(three()) });
    focus(&mut harness);
    select(&mut harness, Some(StopRef::Color(1)));
    harness.state_mut().events.clear();
    harness.state_mut().passes.clear();
    let field = spin(&harness, "Opacity").rect().center();
    press(&mut harness, field);
    for dx in [5.0, 10.0] {
        move_to(&mut harness, field - vec2(dx, 0.0));
    }
    assert!(bar_focused(&harness), "the scrub leaves the focus on the bar");
    harness.event(Event::Key { key: Key::ArrowRight, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    harness.step();
    let stepped = harness.state().stops.color[1].color.a;
    assert_eq!(harness.state().locations(), [0.0, 0.5, 1.0], "the arrow waits");
    for dx in [20.0, 30.0] {
        move_to(&mut harness, field - vec2(dx, 0.0));
    }
    release(&mut harness, field - vec2(30.0, 0.0));
    let state = harness.state();
    let kinds = state.kinds();
    assert_eq!(kinds[kinds.len() - 2..], ["Commit", "Set"], "{kinds:?}");
    assert_eq!(state.passes[kinds.len() - 1], state.passes[kinds.len() - 2] + 1, "the arrow goes out the pass after the release");
    let committed = state.events.iter().find_map(|e| match e {
        EditEvent::Commit { value, .. } => Some(value.color[1].color.a),
        _ => None,
    });
    assert!(committed.is_some_and(|a| a < stepped), "the scrub moved on after the arrow: {committed:?} vs {stepped}");
    assert_eq!(Some(state.stops.color[1].color.a), committed, "the arrow keeps the opacity the scrub committed");
    assert_eq!(state.locations(), [0.0, 0.51, 1.0], "the arrow applies");
}

/// Screen-reader steps on the Opacity and Location fields in one frame both count: the Location
/// step moves the stop the Opacity step left.
#[test]
fn steps_on_two_fields_in_one_frame_both_reach_the_host() {
    use egui::accesskit::{Action, ActionRequest};
    let mut harness = harness_with(Host { rows: StopRows::Combined, ..Host::new(three()) });
    focus(&mut harness);
    select(&mut harness, Some(StopRef::Color(1)));
    harness.state_mut().events.clear();
    let events = [("Opacity", Action::Decrement), ("Location", Action::Increment)].map(|(name, action)| {
        let (target_node, target_tree) = spin(&harness, name).accesskit_node().locate();
        Event::AccessKitActionRequest(ActionRequest { action, target_tree, target_node, data: None })
    });
    harness.input_mut().events.extend(events);
    harness.run();
    let state = harness.state();
    assert_eq!(state.kinds(), ["Set"]);
    assert_eq!(state.stops.color[1].color.a, 0.99, "the opacity step applies");
    assert_eq!(state.locations(), [0.0, 0.51, 1.0], "the location step applies");
}

#[test]
fn enter_on_an_opacity_stop_or_a_midpoint_focuses_its_first_field() {
    let mut harness = rig(three());
    focus(&mut harness);
    select(&mut harness, Some(StopRef::Opacity(1)));
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert!(spin(&harness, "Opacity").is_focused());
    harness.event(Event::Text("40".into()));
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().stops.opacity_stops()[1].opacity, 0.4);
    assert!(harness.state().requested.is_empty());

    focus_from_scratch(&mut harness);
    select(&mut harness, Some(StopRef::Midpoint(StopRow::Color, 1)));
    key(&mut harness, Modifiers::NONE, Key::Space);
    assert!(spin(&harness, "Midpoint").is_focused());
}

#[test]
fn delete_removes_the_selected_stop_after_a_typing_run_and_is_off_at_the_minimum() {
    let mut harness = rig(three());
    select(&mut harness, Some(StopRef::Color(1)));
    type_into(&mut harness, "Location", "30");
    let delete = button(&harness, "Delete stop").rect().center();
    click(&mut harness, delete, PointerButton::Primary);
    assert_eq!(harness.state().kinds(), ["Begin", "Commit", "Set"]);
    let passes = &harness.state().passes;
    assert_eq!(passes[2], passes[1] + 1, "the delete sets in the pass after the run's commit: {passes:?}");
    assert_eq!(harness.state().locations(), [0.0, 1.0]);
    assert_eq!(selected(&harness), Some(StopRef::Color(1)));
    assert!(button(&harness, "Delete stop").accesskit_node().is_disabled(), "two stops are the minimum");
}

#[test]
fn the_color_well_asks_for_the_color_or_opens_the_popover() {
    let mut harness = rig(three());
    let well = color_well(&harness).rect().center();
    click(&mut harness, well, PointerButton::Primary);
    assert_eq!(harness.state().requested, [0]);
    assert!(harness.query_by(|n| n.role() == Role::Dialog).is_none());

    let mut harness = harness_with(Host { popover: true, ..Host::new(three()) });
    let well = color_well(&harness).rect().center();
    click(&mut harness, well, PointerButton::Primary);
    assert!(harness.query_by(|n| n.role() == Role::Dialog).is_some(), "the popover opens");
    assert_eq!(color_well(&harness).accesskit_node().data().has_popup(), Some(egui::accesskit::HasPopup::Dialog));
    // The hex field has the focus with its text selected.
    harness.event(Event::Text("FF0000".into()));
    harness.run();
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().stops.color[0].color.to_rgba().to_hex(), "#ff0000");
    assert_eq!(harness.state().kinds().iter().filter(|k| **k == "Begin").count(), 1, "{:?}", harness.state().kinds());
    assert_eq!(harness.state().kinds().last(), Some(&"Commit"));
    key(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(harness.query_by(|n| n.role() == Role::Dialog).is_none());

    // A double-click on a colour stop opens it too, and nothing goes to the host.
    let at = harness.state().color_marker(1.0);
    move_to(&mut harness, at);
    for _ in 0..2 {
        pointer(&mut harness, at, PointerButton::Primary, true, Modifiers::NONE);
        pointer(&mut harness, at, PointerButton::Primary, false, Modifiers::NONE);
    }
    harness.run();
    assert!(harness.query_by(|n| n.role() == Role::Dialog).is_some());
    assert_eq!(selected(&harness), Some(StopRef::Color(2)));
    assert!(harness.state().requested.is_empty());
}

#[test]
fn a_color_only_editor_keeps_alpha_at_one_through_the_color_well() {
    let translucent = GradientStops::new(vec![
        ColorStop { tag: 1, ..ColorStop::new(0.0, Srgba::new(0.2, 0.2, 0.2, 0.5)) },
        ColorStop { tag: 2, ..ColorStop::new(1.0, Srgba::new(0.8, 0.8, 0.8, 0.5)) },
    ]);
    let mut harness = harness_with(Host { rows: StopRows::ColorOnly, popover: true, ..Host::new(translucent) });
    let well = color_well(&harness).rect().center();
    click(&mut harness, well, PointerButton::Primary);
    harness.event(Event::Text("FF0000".into()));
    harness.run();
    key(&mut harness, Modifiers::NONE, Key::Enter);
    let color = harness.state().stops.color[0].color;
    assert_eq!(color.to_rgba().to_hex(), "#ff0000");
    assert_eq!(color.a, 1.0, "the colour-only rows have no alpha");
    assert_eq!(harness.state().stops.color[1].color.a, 0.5, "the other stop is not edited");
}

#[test]
fn a_press_during_the_location_field_s_typing_run_waits_and_drags_from_the_committed_value() {
    let mut harness = rig(three());
    select(&mut harness, Some(StopRef::Color(1)));
    type_into(&mut harness, "Location", "30");
    assert_eq!(harness.state().kinds(), ["Begin"]);
    let at = harness.state().color_marker(1.0);
    press(&mut harness, at);
    let to = harness.state().color_marker(0.9);
    move_to(&mut harness, to);
    release(&mut harness, to);
    let kinds = harness.state().kinds();
    assert_eq!(kinds[..2], ["Begin", "Commit"], "the run commits first: {kinds:?}");
    assert_eq!((kinds[2], kinds.last()), ("Begin", Some(&"Commit")), "{kinds:?}");
    assert!(!kinds.contains(&"Cancel"));
    let passes = &harness.state().passes;
    assert_eq!(passes[2], passes[1] + 1, "the drag begins in the pass after the run's commit: {passes:?}");
    assert_eq!(harness.state().locations(), [0.0, 0.3, 0.9]);
}

/// Opens the popover on colour stop 0 and types `hex` into its hex field without Enter.
fn hex_run(harness: &mut Harness<'_, Host>, hex: &str) {
    let well = color_well(harness).rect().center();
    click(harness, well, PointerButton::Primary);
    assert!(harness.query_by(|n| n.role() == Role::Dialog).is_some(), "the popover opens");
    harness.event(Event::Text(hex.into()));
    harness.run();
    assert_eq!(harness.state().kinds().first(), Some(&"Begin"));
}

#[test]
fn the_popover_s_hex_run_ends_when_its_picker_goes_and_a_bar_drag_is_a_gesture_of_its_own() {
    // The selection moves to a midpoint: the well and the popover go mid-run.
    let mut harness = harness_with(Host { popover: true, ..Host::new(three()) });
    hex_run(&mut harness, "FF0000");
    select(&mut harness, Some(StopRef::Midpoint(StopRow::Color, 0)));
    assert!(harness.query_by(|n| n.role() == Role::Dialog).is_none());
    let kinds = harness.state().kinds();
    assert_eq!((kinds.iter().filter(|k| **k == "Begin").count(), kinds.last()), (1, Some(&"Commit")), "{kinds:?}");
    assert_eq!(harness.state().stops.color[0].color.to_rgba().to_hex(), "#ff0000");
    harness.state_mut().events.clear();
    let (from, to) = (harness.state().color_marker(1.0), harness.state().color_marker(0.9));
    press(&mut harness, from);
    move_to(&mut harness, to);
    release(&mut harness, to);
    let kinds = harness.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "{kinds:?}");
    assert_eq!(kinds.iter().filter(|k| **k == "Begin").count(), 1, "{kinds:?}");
    assert_eq!(harness.state().locations(), [0.0, 0.5, 0.9]);

    // A press on the bar mid-run closes the popover: the run commits before the drag begins.
    let mut harness = harness_with(Host { popover: true, ..Host::new(three()) });
    hex_run(&mut harness, "00FF00");
    let (from, to) = (harness.state().color_marker(1.0), harness.state().color_marker(0.9));
    press(&mut harness, from);
    move_to(&mut harness, to);
    release(&mut harness, to);
    assert!(harness.query_by(|n| n.role() == Role::Dialog).is_none());
    let kinds = harness.state().kinds();
    let commit = kinds.iter().position(|k| *k == "Commit").expect("the run commits");
    assert_eq!(kinds[..commit].iter().filter(|k| **k == "Begin").count(), 1, "{kinds:?}");
    assert_eq!((kinds[commit + 1], kinds.last()), ("Begin", Some(&"Commit")), "{kinds:?}");
    assert!(!kinds.contains(&"Cancel"), "{kinds:?}");
    assert_eq!(harness.state().stops.color[0].color.to_rgba().to_hex(), "#00ff00");
    assert_eq!(harness.state().locations(), [0.0, 0.5, 0.9]);

    // The whole editor goes mid-run: the host ends its gesture as the hex run would, a commit
    // with the typed colour.
    let mut harness = harness_with(Host { popover: true, ..Host::new(three()) });
    hex_run(&mut harness, "0000FF");
    let Some(EditEvent::Begin { id, .. }) = harness.state().events.first().cloned() else { panic!() };
    harness.state_mut().hidden = true;
    harness.run();
    let ctx = harness.ctx.clone();
    assert!(!craft_ui::edit::is_live(&ctx, id));
    assert_eq!(craft_ui::edit::ending(&ctx, id), craft_ui::edit::Ending::Commit { changed: true });
    let kept = craft_ui::edit::unreported::<GradientStops>(&ctx, id).unwrap_or_else(|| harness.state().stops.clone());
    assert_eq!(kept.color[0].color.to_rgba().to_hex(), "#0000ff");
}

#[test]
fn a_typed_location_keeps_its_decimals_and_a_drag_rounds_to_a_thousandth() {
    let mut harness = harness_with(Host { location_decimals: 3, ..Host::new(three()) });
    select(&mut harness, Some(StopRef::Color(1)));
    type_into(&mut harness, "Location", "33.333");
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().kinds(), ["Begin", "Commit"]);
    assert!((harness.state().stops.color[1].location - 0.33333).abs() < 1e-6, "{:?}", harness.state().locations());
    assert_eq!(spin(&harness, "Location").value().as_deref(), Some("33.333%"));

    let (from, to) = (harness.state().color_marker(1.0), harness.state().color_marker(0.9) + vec2(0.37, 0.0));
    press(&mut harness, from);
    move_to(&mut harness, to);
    release(&mut harness, to);
    let at = harness.state().stops.color[2].location;
    assert_eq!(at, stops::round_location(at), "{at}");
}

#[test]
fn the_popover_s_picker_takes_the_host_s_setup_and_reports_its_eyedropper() {
    // Tall enough for the popover with alpha and the eyedropper.
    let host = Host { popover: true, eyedropper: true, rows: StopRows::Combined, ..Host::new(three()) };
    let mut harness = harness_sized(host, Direction::Ltr, [400.0, 640.0]);
    let well = color_well(&harness).rect().center();
    click(&mut harness, well, PointerButton::Primary);
    let opacity = harness.query_all_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Opacity")).count();
    assert_eq!(opacity, 2, "the fields row's and the picker's alpha, set before the host's setup");
    let picker = harness.state().picker_id.expect("the picker is shown");
    let eyedropper = button(&harness, "Pick screen color").rect().center();
    click(&mut harness, eyedropper, PointerButton::Primary);
    assert!(harness.state().eyedropper_requested);
    harness.state_mut().events.clear();
    craft_color::ui::ColorPicker::sampled(&harness.ctx, picker, Srgba::rgb(1.0, 0.0, 0.0));
    harness.run();
    assert_eq!(harness.state().kinds(), ["Set"]);
    assert_eq!(harness.state().stops.color[0].color.to_rgba().to_hex(), "#ff0000");
}

#[test]
fn a_location_run_moves_the_stop_from_where_it_began() {
    // Two stops at 50 %: typing "5" then "50" on the second passes the first and comes back.
    let tied = GradientStops::new(vec![stop(0.0, 1), stop(0.5, 2), stop(0.5, 3), stop(1.0, 4)]);
    let mut harness = rig(tied.clone());
    select(&mut harness, Some(StopRef::Color(2)));
    type_into(&mut harness, "Location", "5");
    assert_eq!((harness.state().locations(), selected(&harness)), (vec![0.0, 0.05, 0.5, 1.0], Some(StopRef::Color(1))));
    harness.event(Event::Text("0".into()));
    harness.run();
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().kinds().last(), Some(&"Commit unchanged"), "{:?}", harness.state().kinds());
    assert_eq!(harness.state().stops, tied);
    assert_eq!(selected(&harness), Some(StopRef::Color(2)));
}

#[test]
fn a_screen_reader_step_in_the_frame_the_popover_s_run_ends_goes_onto_the_committed_stops() {
    // The host turns the popover off mid hex run, and a screen reader steps the Opacity field in
    // the same frame.
    let mut harness = harness_with(Host { popover: true, rows: StopRows::Combined, ..Host::new(three()) });
    hex_run(&mut harness, "FF0000");
    harness.state_mut().popover = false;
    let fields = harness.query_all_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Opacity"));
    let field = fields.min_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).expect("the fields row's Opacity");
    field.decrement_accesskit();
    harness.run();
    let kinds = harness.state().kinds();
    assert_eq!((kinds.first(), &kinds[kinds.len() - 2..]), (Some(&"Begin"), &["Commit", "Set"][..]), "{kinds:?}");
    let color = harness.state().stops.color[0].color;
    assert_eq!(color.to_rgba().to_hex(), "#ff0000fc", "the typed colour with the step's opacity");
}

/// A key press on the bar while the pointer scrubs a field.
fn bar_key_during_scrub(harness: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    harness.event(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers });
    harness.step();
}

/// The value of the editor's last Commit.
fn committed(harness: &Harness<'_, Host>) -> GradientStops {
    harness
        .state()
        .events
        .iter()
        .rev()
        .find_map(|e| match e {
            EditEvent::Commit { value, .. } => Some(value.clone()),
            _ => None,
        })
        .expect("a Commit")
}

#[test]
fn a_held_bar_arrow_during_a_location_scrub_of_the_same_stop_steps_from_the_committed_location() {
    let mut harness = rig(three());
    focus(&mut harness);
    select(&mut harness, Some(StopRef::Color(1)));
    harness.state_mut().events.clear();
    let field = spin(&harness, "Location").rect().center();
    press(&mut harness, field);
    for dx in [5.0, 10.0] {
        move_to(&mut harness, field + vec2(dx, 0.0));
    }
    assert!(bar_focused(&harness), "the scrub leaves the focus on the bar");
    bar_key_during_scrub(&mut harness, Modifiers::NONE, Key::ArrowRight);
    for dx in [15.0, 20.0] {
        move_to(&mut harness, field + vec2(dx, 0.0));
    }
    release(&mut harness, field + vec2(20.0, 0.0));
    let kinds = harness.state().kinds();
    assert_eq!(kinds[kinds.len() - 2..], ["Commit", "Set"], "{kinds:?}");
    let at = committed(&harness).color[1].location;
    assert!(at > 0.55, "the scrub moved on after the arrow: {at}");
    assert_eq!(harness.state().locations(), [0.0, at + 0.01, 1.0], "the arrow steps from the committed location");
}

#[test]
fn a_held_move_past_a_neighbour_keeps_the_selection_on_the_moved_stop() {
    let stops = GradientStops::new(vec![stop(0.0, 1), stop(0.5, 2), stop(0.55, 3), stop(1.0, 4)]);
    let mut harness = harness_with(Host { rows: StopRows::Combined, ..Host::new(stops) });
    focus(&mut harness);
    select(&mut harness, Some(StopRef::Color(1)));
    let field = spin(&harness, "Opacity").rect().center();
    press(&mut harness, field);
    for dx in [5.0, 10.0] {
        move_to(&mut harness, field - vec2(dx, 0.0));
    }
    bar_key_during_scrub(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
    assert_eq!(selected(&harness), Some(StopRef::Color(1)), "the selection waits with the move");
    move_to(&mut harness, field - vec2(15.0, 0.0));
    release(&mut harness, field - vec2(15.0, 0.0));
    let state = harness.state();
    assert_eq!(state.kinds()[state.kinds().len() - 2..], ["Commit", "Set"]);
    assert_eq!(state.locations(), [0.0, 0.55, 0.6, 1.0]);
    assert_eq!(state.stops.color[2].tag, 2);
    assert_eq!(selected(&harness), Some(StopRef::Color(2)), "the selection follows the moved stop");
}

#[test]
fn a_held_step_on_an_untagged_stop_follows_it_past_a_neighbour() {
    let untagged = GradientStops::new(vec![stop(0.0, 0), stop(0.4, 0), stop(0.6, 0), stop(1.0, 0)]);
    let mut harness = rig(untagged);
    focus(&mut harness);
    select(&mut harness, Some(StopRef::Color(1)));
    harness.state_mut().events.clear();
    let field = spin(&harness, "Location").rect().center();
    press(&mut harness, field);
    for dx in [5.0, 10.0] {
        move_to(&mut harness, field + vec2(dx, 0.0));
    }
    bar_key_during_scrub(&mut harness, Modifiers::NONE, Key::ArrowRight);
    for dx in [20.0, 35.0] {
        move_to(&mut harness, field + vec2(dx, 0.0));
    }
    release(&mut harness, field + vec2(35.0, 0.0));
    let kinds = harness.state().kinds();
    assert_eq!(kinds[kinds.len() - 2..], ["Commit", "Set"], "{kinds:?}");
    let at = committed(&harness).color[2].location;
    assert!(at > 0.6, "the scrub moved the stop past its neighbour: {at}");
    assert_eq!(harness.state().locations(), [0.0, 0.6, at + 0.01, 1.0], "the moved stop steps; the other stays");
}
