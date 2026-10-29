//! Levels editor tests: the rows' and fields' edits as `Levels`, the gamma mapping, field ranges,
//! a press during a typing run, the clipping report, AccessKit, the direction and the stories.

use egui::accesskit::Role;
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::*;
use crate::controls::NumberField;
use crate::direction::{self, Direction};
use crate::edit::{CancelReason, EditEvent, EditPolicy, GestureHost, UndoMarks};
use crate::localized::LocaleNumbers;
use crate::testing::a11y;
use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, install};

/// A host of one channel's levels that applies every value and restores the start value on a
/// cancel, or hands the events to a [`GestureHost`] with undo steps.
struct Host {
    levels: Levels,
    before: Option<Levels>,
    events: Vec<EditEvent<Levels>>,
    id: Id,
    /// `line.focus`.
    focus: egui::Color32,
    scoped: bool,
    enabled: bool,
    /// The host stops drawing the editor.
    hidden: bool,
    /// [`LevelsEditor::clipping`] after the last frame.
    clip: Option<LevelsClip>,
    undo: Option<Undo>,
    policy: EditPolicy,
    /// The pass of each event.
    passes: Vec<u64>,
    /// The pass of the last event, and whether two events came in one pass.
    event_pass: Option<u64>,
    doubled: bool,
    /// A gamma field of the same levels outside the editor, drawn before it, with its policy.
    outside: Option<EditPolicy>,
    outside_events: Vec<EditEvent<f64>>,
}

struct Undo {
    host: GestureHost<Doc>,
    doc: Doc,
}

struct Doc {
    now: Levels,
    undo: Vec<Levels>,
}

impl UndoMarks for Doc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(&before) = self.undo.get(mark) {
            self.now = before;
        }
        self.undo.truncate(mark);
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

impl Host {
    fn new(levels: Levels) -> Self {
        Self {
            levels,
            before: None,
            events: Vec::new(),
            id: Id::NULL,
            focus: egui::Color32::TRANSPARENT,
            scoped: false,
            enabled: true,
            hidden: false,
            clip: None,
            undo: None,
            policy: EditPolicy::default(),
            passes: Vec::new(),
            event_pass: None,
            doubled: false,
            outside: None,
            outside_events: Vec::new(),
        }
    }

    fn with_undo(levels: Levels) -> Self {
        Self { undo: Some(Undo { host: GestureHost::new(), doc: Doc { now: levels, undo: Vec::new() } }), ..Self::new(levels) }
    }

    fn show(&mut self, ui: &mut Ui) {
        if self.hidden {
            self.clip = LevelsEditor::clipping(ui.ctx(), self.id);
            return;
        }
        let numbers = LocaleNumbers::from_tag("en");
        self.focus = Tokens::of(ui).color.line.focus.to_egui();
        // The editor draws the value of the frame's start, as a host that applies the frame's
        // events after its controls shows it.
        let levels = self.levels;
        if let Some(policy) = self.outside {
            let edit = NumberField::new("outside", "Outside gamma", levels.gamma, &numbers).policy(policy).show(ui);
            if let Some(&gamma) = edit.event.as_ref().and_then(EditEvent::value) {
                self.levels.gamma = gamma;
            }
            self.outside_events.extend(edit.event);
        }
        let editor =
            LevelsEditor::new("levels", "Levels", &levels, &numbers).value("Red channel").width(300.0).enabled(self.enabled).policy(self.policy);
        let edit = match self.scoped {
            true => crate::edit::scope(ui, |ui| editor.show(ui)).inner,
            false => editor.show(ui),
        };
        self.id = edit.response.id;
        if edit.event.is_some() {
            let pass = ui.ctx().cumulative_pass_nr();
            self.doubled |= self.event_pass == Some(pass);
            self.event_pass = Some(pass);
            self.passes.push(pass);
        }
        self.clip = LevelsEditor::clipping(ui.ctx(), self.id);
        if let Some(undo) = &mut self.undo {
            if let Some(event) = edit.event.clone() {
                undo.host.handle(ui.ctx(), &mut undo.doc, event, |doc, value| {
                    let before = std::mem::replace(&mut doc.now, value);
                    doc.undo.push(before);
                });
            }
            let _ = undo.host.end_frame(ui.ctx(), &mut undo.doc);
            self.levels = undo.doc.now;
            self.events.extend(edit.event);
            return;
        }
        if let Some(event) = edit.event {
            match &event {
                EditEvent::Begin { .. } => self.before = Some(self.levels),
                EditEvent::Cancel { .. } => {
                    if let Some(before) = self.before.take() {
                        self.levels = before;
                    }
                }
                other => {
                    if let Some(value) = other.value() {
                        self.levels = *value;
                    }
                }
            }
            self.events.push(event);
        }
    }

    /// Forgets the events so far.
    fn clear(&mut self) {
        self.events.clear();
        self.passes.clear();
    }

    /// The pass of the first event of `kind` ([`kinds`](Host::kinds)).
    fn pass_of(&self, kind: &str) -> u64 {
        let i = self.kinds().iter().position(|k| *k == kind);
        assert!(i.is_some(), "no {kind} in {:?}", self.kinds());
        self.passes[i.unwrap_or_default()]
    }

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

const IDENTITY: Levels = Levels { in_black: 0.0, gamma: 1.0, in_white: 255.0, out_black: 0.0, out_white: 255.0 };

fn harness(host: Host, dir: Direction) -> Harness<'static, Host> {
    let mut harness = Harness::builder()
        .with_size([480.0, 520.0])
        .with_context_setup(move |ctx: &Context| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(|ui, host: &mut Host| host.show(ui), host);
    harness.run();
    harness
}

fn rig(levels: Levels) -> Harness<'static, Host> {
    harness(Host::new(levels), Direction::Ltr)
}

fn node<'a>(harness: &'a Harness<'_, Host>, role: Role, name: &str) -> Node<'a> {
    let name = name.to_owned();
    harness.get_by(move |n| n.role() == role && n.label().as_deref() == Some(name.as_str()))
}

/// The histogram's plot: it spans the thumbs' travel, level 0 at its left and 255 at its right.
fn plot(harness: &Harness<'_, Host>) -> Rect {
    node(harness, Role::Image, "Histogram").rect()
}

/// The point at `level` on the row of thumb `name`.
fn at(harness: &Harness<'_, Host>, name: &str, level: f64) -> Pos2 {
    let plot = plot(harness);
    let y = node(harness, Role::Slider, name).rect().center().y;
    pos2(plot.left() + plot.width() * (level / 255.0) as f32, y)
}

fn thumb(harness: &Harness<'_, Host>, name: &str) -> Pos2 {
    node(harness, Role::Slider, name).rect().center()
}

fn pointer(harness: &mut Harness<'_, Host>, pos: Pos2, pressed: bool, modifiers: Modifiers) {
    harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers });
    harness.step();
}

fn move_to(harness: &mut Harness<'_, Host>, pos: Pos2) {
    harness.hover_at(pos);
    harness.step();
}

fn press(harness: &mut Harness<'_, Host>, pos: Pos2) {
    move_to(harness, pos);
    pointer(harness, pos, true, Modifiers::NONE);
}

fn release(harness: &mut Harness<'_, Host>, pos: Pos2) {
    pointer(harness, pos, false, Modifiers::NONE);
    harness.run();
}

fn drag(harness: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    press(harness, from);
    move_to(harness, pos2((from.x + to.x) / 2.0, from.y));
    move_to(harness, to);
    release(harness, to);
}

fn click(harness: &mut Harness<'_, Host>, pos: Pos2) {
    move_to(harness, pos);
    pointer(harness, pos, true, Modifiers::NONE);
    pointer(harness, pos, false, Modifiers::NONE);
    harness.run();
}

/// A press and release at `pos` in one frame (`Harness::event` would give each its own).
fn tap(harness: &mut Harness<'_, Host>, pos: Pos2) {
    let tap = [true, false].map(|pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    harness.input_mut().events.extend(tap);
    harness.step();
}

#[track_caller]
fn assert_near(a: f32, b: f32, what: &str) {
    assert!((a - b).abs() < 0.01, "{what}: {a} vs {b}");
}

#[test]
fn a_thumb_drag_is_one_gesture_of_levels_and_the_gamma_thumb_follows_black_through_the_half_power() {
    let mut harness = rig(IDENTITY);
    let from = thumb(&harness, "Input black");
    let to = at(&harness, "Input black", 50.0);
    drag(&mut harness, from, to);
    let kinds = harness.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "{kinds:?}");
    assert_eq!(kinds.iter().filter(|k| **k == "Begin").count(), 1, "{kinds:?}");
    assert_eq!(harness.state().levels, Levels { in_black: 50.0, ..IDENTITY }, "the gamma and the output stay");
    assert_near(thumb(&harness, "Gamma").x, at(&harness, "Gamma", 152.5).x, "the gamma thumb sits halfway between black and white");
    assert_eq!(node(&harness, Role::SpinButton, "Input black").accesskit_node().numeric_value(), Some(50.0), "the field shows the thumb's value");

    // A quarter of the way from black to white is gamma ln 0.25 / ln 0.5.
    harness.state_mut().clear();
    let from = thumb(&harness, "Gamma");
    let to = pos2(at(&harness, "Gamma", 50.0 + 205.0 * 0.25).x, from.y);
    drag(&mut harness, from, to);
    assert_eq!(harness.state().levels, Levels { in_black: 50.0, gamma: 2.0, ..IDENTITY });
    assert_eq!(node(&harness, Role::SpinButton, "Gamma").accesskit_node().numeric_value(), Some(2.0));

    // Past the black thumb the gamma stops at its range's end, the thumb short of black.
    let from = thumb(&harness, "Gamma");
    let to = pos2(at(&harness, "Gamma", 0.0).x, from.y);
    drag(&mut harness, from, to);
    assert_eq!(harness.state().levels.gamma, 9.99);
    assert!(thumb(&harness, "Gamma").x > thumb(&harness, "Input black").x);
}

#[test]
fn a_field_scrub_is_one_gesture_of_levels_that_stops_at_the_other_thumb() {
    let mut harness = rig(Levels { in_white: 40.0, ..IDENTITY });
    let field = node(&harness, Role::SpinButton, "Input black").rect().center();
    press(&mut harness, field);
    for dx in [50.0, 100.0, 200.0] {
        move_to(&mut harness, field + vec2(dx, 0.0));
    }
    release(&mut harness, field + vec2(200.0, 0.0));
    let kinds = harness.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "{kinds:?}");
    assert_eq!(kinds.iter().filter(|k| **k == "Begin").count(), 1, "{kinds:?}");
    assert_eq!(harness.state().levels, Levels { in_black: 38.0, in_white: 40.0, ..IDENTITY }, "black stops two levels below white");
    assert_near(thumb(&harness, "Input black").x, at(&harness, "Input black", 38.0).x, "the thumb follows the field");
}

/// Typing into a field and pressing a row without Enter: the run's commit goes out first and the
/// row's drag starts from the typed value, one undo step each; the output row is drawn after the
/// input fields and the input row before the output fields.
#[test]
fn a_press_on_a_row_during_a_typing_run_commits_the_run_then_drags_from_it() {
    let mut harness = harness(Host::with_undo(IDENTITY), Direction::Ltr);
    let cases = [("Input black", "30", "Output white", 200.0), ("Output black", "20", "Input white", 220.0)];
    for (step, (field, typed, row, level)) in cases.into_iter().enumerate() {
        harness.state_mut().clear();
        let field_at = node(&harness, Role::SpinButton, field).rect().center();
        click(&mut harness, field_at);
        harness.event(Event::Text(typed.into()));
        harness.run();
        let from = thumb(&harness, row);
        let to = pos2(at(&harness, row, level).x, from.y);
        drag(&mut harness, from, to);
        let kinds = harness.state().kinds();
        assert_eq!(kinds[..4], ["Begin", "Update", "Commit", "Begin"], "{field}: {kinds:?}");
        assert_eq!(kinds.last(), Some(&"Commit"), "{field}: {kinds:?}");
        let undo = harness.state().undo.as_ref().map(|u| u.doc.undo.len());
        assert_eq!(undo, Some(2 * (step + 1)), "{field}: the run and the drag, one step each");
    }
    assert_eq!(harness.state().levels, Levels { in_black: 30.0, in_white: 220.0, out_black: 20.0, out_white: 200.0, ..IDENTITY });
}

/// The input row is drawn before the output fields: its press waits one pass for the typing run's
/// commit, and begins in the pass after.
#[test]
fn a_press_on_the_input_row_during_an_output_field_run_begins_one_pass_after_the_press() {
    let mut harness = rig(IDENTITY);
    let field_at = node(&harness, Role::SpinButton, "Output black").rect().center();
    click(&mut harness, field_at);
    harness.event(Event::Text("20".into()));
    harness.run();
    harness.state_mut().clear();
    let from = thumb(&harness, "Input white");
    move_to(&mut harness, from);
    pointer(&mut harness, from, true, Modifiers::NONE);
    assert_eq!(harness.state().kinds(), ["Commit"], "the press pass ends the run only");
    harness.step();
    assert_eq!(harness.state().kinds(), ["Commit", "Begin"], "the press begins one pass later");
    release(&mut harness, from);
    assert_eq!(harness.state().levels, Levels { out_black: 20.0, ..IDENTITY });
}

/// An on-release policy withholds the field's updates, and the thumbs still follow the scrub: every
/// part draws from the composite's shown value, not the host's.
#[test]
fn a_field_scrub_moves_its_thumb_under_an_on_release_policy() {
    let host = Host { policy: EditPolicy::default().on_release_only(true), ..Host::new(IDENTITY) };
    let mut harness = harness(host, Direction::Ltr);
    let field = node(&harness, Role::SpinButton, "Input black").rect().center();
    press(&mut harness, field);
    for dx in [20.0, 40.0, 60.0] {
        move_to(&mut harness, field + vec2(dx, 0.0));
    }
    harness.run();
    let shown = node(&harness, Role::SpinButton, "Input black").accesskit_node().numeric_value().expect("the field has a value");
    assert!(shown > 0.0, "the scrub moved the field");
    assert_eq!(harness.state().kinds(), ["Begin"], "the host sees no update before the release");
    assert_near(thumb(&harness, "Input black").x, at(&harness, "Input black", shown).x, "the thumb follows the field");
    assert_near(thumb(&harness, "Gamma").x, at(&harness, "Gamma", shown + (255.0 - shown) * 0.5).x, "and the gamma thumb follows black");
    release(&mut harness, field + vec2(60.0, 0.0));
    assert_eq!(harness.state().levels.in_black, shown);
}

/// A screen reader's step on a field while a row is dragged waits for the release: the drag is
/// undisturbed and the step is its own Set, and its own undo step, after the Commit.
#[test]
fn a_screen_reader_step_on_a_field_during_a_row_drag_is_its_own_set_after_the_release() {
    let mut harness = harness(Host::with_undo(IDENTITY), Direction::Ltr);
    let from = thumb(&harness, "Output white");
    let to = pos2(at(&harness, "Output white", 200.0).x, from.y);
    press(&mut harness, from);
    move_to(&mut harness, pos2((from.x + to.x) / 2.0, from.y));
    move_to(&mut harness, to);
    harness.state_mut().clear();
    let (target, tree) = node(&harness, Role::SpinButton, "Gamma").accesskit_node().locate();
    harness.event(Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::Increment,
        target_tree: tree,
        target_node: target,
        data: None,
    }));
    harness.step();
    harness.step();
    assert!(harness.state().events.is_empty(), "the step doesn't join the drag: {:?}", harness.state().kinds());
    assert_eq!(harness.state().levels.gamma, 1.0);
    release(&mut harness, to);
    assert_eq!(harness.state().kinds(), ["Commit", "Set"], "the step is its own Set after the release");
    assert_eq!(harness.state().levels, Levels { gamma: 1.01, out_white: 200.0, ..IDENTITY });
    let steps = harness.state().undo.as_ref().map(|u| u.doc.undo.len());
    assert_eq!(steps, Some(2), "the drag and the step, one undo step each");
}

/// A screen reader's step on the gamma thumb during a scrub of the black field waits for the
/// release and changes only gamma: the row's value from the step's pass doesn't bring back the
/// black the scrub moved on from.
#[test]
fn a_screen_reader_step_on_a_row_during_a_field_scrub_keeps_the_scrubs_value() {
    let mut harness = harness(Host::with_undo(IDENTITY), Direction::Ltr);
    let field = node(&harness, Role::SpinButton, "Input black").rect().center();
    press(&mut harness, field);
    for dx in [20.0, 40.0] {
        move_to(&mut harness, field + vec2(dx, 0.0));
    }
    let (target, tree) = node(&harness, Role::Slider, "Gamma").accesskit_node().locate();
    harness.event(Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::Increment,
        target_tree: tree,
        target_node: target,
        data: None,
    }));
    harness.step();
    let stepped = harness.state().levels.in_black;
    assert_eq!(harness.state().levels.gamma, 1.0, "the step waits");
    harness.state_mut().clear();
    for dx in [60.0, 80.0] {
        move_to(&mut harness, field + vec2(dx, 0.0));
    }
    release(&mut harness, field + vec2(80.0, 0.0));
    assert_eq!(harness.state().kinds()[harness.state().kinds().len() - 2..], ["Commit", "Set"], "{:?}", harness.state().kinds());
    assert_eq!(harness.state().pass_of("Set"), harness.state().pass_of("Commit") + 1, "the step goes out the pass after the release");
    let committed = harness.state().events.iter().find_map(|e| match e {
        EditEvent::Commit { value, .. } => Some(value.in_black),
        _ => None,
    });
    assert!(committed.is_some_and(|black| black > stepped), "the scrub moved on after the step: {committed:?} vs {stepped}");
    let levels = harness.state().levels;
    assert_eq!(Some(levels.in_black), committed, "the step keeps the black the scrub committed");
    assert_ne!(levels.gamma, 1.0, "the step applies");
    assert_eq!(levels, Levels { in_black: levels.in_black, gamma: levels.gamma, ..IDENTITY });
    let steps = harness.state().undo.as_ref().map(|u| u.doc.undo.len());
    assert_eq!(steps, Some(2), "the scrub and the step, one undo step each");
}

/// A screen reader's step on the black thumb during a scrub of the black field waits for the
/// release without disturbing the scrub: the field draws from the editor's value, which the
/// waiting step isn't in, so the field sees no outside change of its value.
#[test]
fn a_held_row_step_on_the_scrubbed_value_leaves_the_scrub_running() {
    let mut harness = harness(Host::with_undo(IDENTITY), Direction::Ltr);
    let field = node(&harness, Role::SpinButton, "Input black").rect().center();
    press(&mut harness, field);
    for dx in [20.0, 40.0] {
        move_to(&mut harness, field + vec2(dx, 0.0));
    }
    let (target, tree) = node(&harness, Role::Slider, "Input black").accesskit_node().locate();
    harness.event(Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::Increment,
        target_tree: tree,
        target_node: target,
        data: None,
    }));
    harness.step();
    harness.step();
    for dx in [60.0, 80.0] {
        move_to(&mut harness, field + vec2(dx, 0.0));
    }
    release(&mut harness, field + vec2(80.0, 0.0));
    let kinds = harness.state().kinds();
    assert!(!kinds.contains(&"Cancel"), "{kinds:?}");
    assert_eq!(kinds[kinds.len() - 2..], ["Commit", "Set"], "{kinds:?}");
}

/// A screen reader's step on the black thumb during a scrub of the white field that ends closer
/// to black than the step allows: the held step keeps the thumbs' order on the row of its turn,
/// as a drag of the thumb would stop at white less the gap, so it moves nothing.
#[test]
fn a_held_row_step_keeps_the_thumb_order_on_the_row_of_its_turn() {
    let mut harness = harness(Host::with_undo(Levels { in_black: 40.0, ..IDENTITY }), Direction::Ltr);
    let field = node(&harness, Role::SpinButton, "Input white").rect().center();
    press(&mut harness, field);
    for dx in [-20.0, -40.0] {
        move_to(&mut harness, field + vec2(dx, 0.0));
    }
    let (target, tree) = node(&harness, Role::Slider, "Input black").accesskit_node().locate();
    harness.event(Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::Increment,
        target_tree: tree,
        target_node: target,
        data: None,
    }));
    harness.step();
    assert_eq!(harness.state().levels.in_black, 40.0, "the step waits");
    for dx in [-150.0, -250.0] {
        move_to(&mut harness, field + vec2(dx, 0.0));
    }
    release(&mut harness, field + vec2(-250.0, 0.0));
    harness.run();
    let levels = harness.state().levels;
    assert_eq!(
        (levels.in_black, levels.in_white),
        (40.0, 42.0),
        "white stops two levels above black, and the step can't move black: {:?}",
        harness.state().kinds()
    );
}

/// A tap (press and release in one frame) on a row in the pass a field's typing run commits
/// reaches the host in a later pass than the Commit: a composite reports one edit per pass.
#[test]
fn a_tap_on_a_row_during_a_typing_run_reports_after_the_runs_commit() {
    for field in ["Input black", "Output black"] {
        let mut harness = harness(Host::with_undo(IDENTITY), Direction::Ltr);
        let field_at = node(&harness, Role::SpinButton, field).rect().center();
        click(&mut harness, field_at);
        harness.event(Event::Text("20".into()));
        harness.run();
        harness.state_mut().clear();
        let target = node(&harness, Role::Slider, "Input white");
        let pos = pos2(at(&harness, "Input white", 240.0).x, target.rect().center().y);
        move_to(&mut harness, pos);
        tap(&mut harness, pos);
        harness.state_mut().doubled = false;
        for _ in 0..4 {
            harness.step();
        }
        assert!(!harness.state().doubled, "{field}: one event per pass: {:?}", harness.state().kinds());
        let kinds = harness.state().kinds();
        assert_eq!(kinds, ["Commit", "Set"], "{field}");
        assert_eq!(harness.state().pass_of("Set"), harness.state().pass_of("Commit") + 1, "{field}: the tap reports the pass after the run");
        assert_eq!(harness.state().levels.in_white, 240.0, "{field}");
    }
}

/// A tap on a row in the pass a field outside the editor ends its typing run (the tap's click
/// moves the focus off the field, which is drawn first) goes out after the run's Commit, made from
/// the committed value: a value no update reported isn't lost to the tap.
#[test]
fn a_tap_ending_an_outside_fields_run_keeps_the_value_the_run_committed() {
    let host = Host { outside: Some(EditPolicy::default().on_release_only(true)), ..Host::new(IDENTITY) };
    let mut harness = harness(host, Direction::Ltr);
    let field_at = node(&harness, Role::SpinButton, "Outside gamma").rect().center();
    click(&mut harness, field_at);
    harness.event(Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND });
    harness.event(Event::Text("2".into()));
    harness.run();
    assert_eq!(harness.state().levels.gamma, 1.0, "the field reports no update");
    let target = node(&harness, Role::Slider, "Input white");
    let pos = pos2(at(&harness, "Input white", 240.0).x, target.rect().center().y);
    move_to(&mut harness, pos);
    tap(&mut harness, pos);
    harness.run();
    let state = harness.state();
    assert!(matches!(state.outside_events.as_slice(), [EditEvent::Begin { .. }, EditEvent::Commit { value: 2.0, .. }]), "{:?}", state.outside_events);
    assert_eq!((state.levels.in_white, state.levels.gamma), (240.0, 2.0), "{:?}", state.kinds());
}

#[test]
fn a_thumb_edit_leaves_the_other_values_as_they_are() {
    let start = Levels { in_black: -10.0, gamma: f64::NAN, in_white: 300.0, out_black: -20.0, out_white: 300.0 };
    let mut harness = rig(start);
    let from = thumb(&harness, "Output black");
    drag(&mut harness, from, from + vec2(40.0, 0.0));
    let levels = harness.state().levels;
    assert!(levels.out_black > 0.0, "{levels:?}");
    assert_eq!(levels.out_white, 300.0, "the other output value stays out of range");
    assert_eq!((levels.in_black, levels.in_white), (-10.0, 300.0));
    assert!(levels.gamma.is_nan());

    harness.state_mut().clear();
    node(&harness, Role::Slider, "Input black").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    let levels = harness.state().levels;
    assert_eq!(levels.in_black, 1.0, "a key steps from the thumb as drawn");
    assert!(levels.gamma.is_nan());
    assert_eq!((levels.in_white, levels.out_white), (300.0, 300.0));
}

#[test]
fn alt_during_an_input_end_drag_reports_which_end_clips() {
    let mut harness = rig(Levels { in_black: 20.0, in_white: 200.0, out_black: 30.0, out_white: 220.0, ..IDENTITY });
    for (name, clip) in
        [("Input black", Some(LevelsClip::Shadows)), ("Input white", Some(LevelsClip::Highlights)), ("Gamma", None), ("Output black", None)]
    {
        let from = thumb(&harness, name);
        harness.event(Event::ModifiersChanged(Modifiers::ALT));
        move_to(&mut harness, from);
        pointer(&mut harness, from, true, Modifiers::ALT);
        move_to(&mut harness, from + vec2(6.0, 0.0));
        move_to(&mut harness, from + vec2(12.0, 0.0));
        assert_eq!(harness.state().clip, clip, "{name}");
        harness.event(Event::ModifiersChanged(Modifiers::NONE));
        move_to(&mut harness, from + vec2(14.0, 0.0));
        assert_eq!(harness.state().clip, None, "{name}: without Alt");
        pointer(&mut harness, from + vec2(14.0, 0.0), false, Modifiers::NONE);
        harness.run();
        assert_eq!(harness.state().clip, None, "{name}: after the release");
    }

    // An editor hidden during the drag reports no clipping after it.
    let from = thumb(&harness, "Input black");
    harness.event(Event::ModifiersChanged(Modifiers::ALT));
    move_to(&mut harness, from);
    pointer(&mut harness, from, true, Modifiers::ALT);
    move_to(&mut harness, from + vec2(6.0, 0.0));
    move_to(&mut harness, from + vec2(12.0, 0.0));
    assert_eq!(harness.state().clip, Some(LevelsClip::Shadows));
    harness.state_mut().hidden = true;
    harness.run();
    assert_eq!(harness.state().clip, None, "the host stops drawing the editor");
}

#[test]
fn the_group_histogram_thumbs_and_fields_are_in_the_accesskit_tree_in_tab_order() {
    let mut harness = rig(Levels { in_white: 200.0, ..IDENTITY });
    a11y::assert_labelled(&harness);
    let group = node(&harness, Role::Group, "Levels");
    assert_eq!(group.value().as_deref(), Some("Red channel"));
    assert_eq!(group.query_all_by(|n| n.role() == Role::Image).count(), 1, "the histogram is in the group");
    let numeric = |harness: &Harness<'_, Host>, role, name| {
        let n = node(harness, role, name);
        let n = n.accesskit_node();
        (n.numeric_value(), n.min_numeric_value(), n.max_numeric_value())
    };
    let sliders = [
        ("Input black", (0.0, 0.0, 198.0)),
        ("Gamma", (1.0, 0.01, 9.99)),
        ("Input white", (200.0, 2.0, 255.0)),
        ("Output black", (0.0, 0.0, 255.0)),
        ("Output white", (255.0, 0.0, 255.0)),
    ];
    for (name, (value, min, max)) in sliders {
        assert_eq!(numeric(&harness, Role::Slider, name), (Some(value), Some(min), Some(max)), "{name}");
        assert_eq!(node(&harness, Role::SpinButton, name).accesskit_node().numeric_value(), Some(value), "{name} field");
    }

    let order: Vec<_> = a11y::tab_order(&mut harness).into_iter().map(|s| (s.role, s.label)).collect();
    let names = |role, names: &[&str]| names.iter().map(|n| (role, (*n).to_owned())).collect::<Vec<_>>();
    let input = ["Input black", "Gamma", "Input white"];
    let output = ["Output black", "Output white"];
    let expected = [names(Role::Slider, &input), names(Role::SpinButton, &input), names(Role::Slider, &output), names(Role::SpinButton, &output)];
    assert_eq!(order, expected.concat());

    // The gamma thumb: keyboard focus shows a ring, and Right moves the thumb right.
    node(&harness, Role::Slider, "Gamma").focus();
    harness.run();
    let ring = harness.state().focus;
    a11y::assert_focus_ring(&harness, ring);
    harness.state_mut().clear();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().kinds(), ["Set"]);
    assert_eq!(harness.state().levels.gamma, 0.99);
}

#[test]
fn a_pressed_thumb_has_the_focus_but_no_focus_ring() {
    let mut harness = rig(IDENTITY);
    let ring = harness.state().focus;
    let from = thumb(&harness, "Input white");
    press(&mut harness, from);
    release(&mut harness, from);
    let (target, rings) = a11y::focus_rings(&harness).expect("the pressed thumb has the focus");
    assert!(rings.iter().all(|r| r.stroke.color != ring), "a pointer press shows no focus ring at {target:?}: {rings:?}");
}

#[test]
fn right_to_left_keeps_the_geometry_and_the_plot_spans_the_thumbs() {
    let layout = |dir| {
        let harness = harness(Host::new(Levels { in_black: 10.0, in_white: 240.0, out_black: 255.0, out_white: 0.0, ..IDENTITY }), dir);
        let origin = node(&harness, Role::Group, "Levels").rect().min;
        let rects: Vec<_> = [Role::Image, Role::Slider, Role::SpinButton]
            .into_iter()
            .flat_map(|role| harness.query_all_by(move |n| n.role() == role).map(|n| n.rect().translate(-origin.to_vec2())).collect::<Vec<_>>())
            .collect();
        let plot = plot(&harness);
        assert_near(thumb(&harness, "Output black").x, plot.right(), "level 255 at the plot's right");
        assert_near(thumb(&harness, "Output white").x, plot.left(), "level 0 at the plot's left");
        let x = |name| node(&harness, Role::SpinButton, name).rect().center().x;
        assert!(x("Input black") < x("Gamma") && x("Gamma") < x("Input white"), "{dir:?}: black, gamma, white from the left");
        assert!(x("Output black") < x("Output white"), "{dir:?}");
        rects
    };
    let ltr = layout(Direction::Ltr);
    assert_eq!(ltr.len(), 11, "{ltr:?}");
    assert_eq!(layout(Direction::Rtl), ltr);
}

#[test]
fn values_outside_the_range_draw_at_the_ends_and_a_nan_gamma_begins_no_drag() {
    let mut harness = rig(Levels { in_black: -10.0, gamma: f64::NAN, in_white: 300.0, ..IDENTITY });
    let plot = plot(&harness);
    assert_near(thumb(&harness, "Input black").x, plot.left(), "below the range: at the start");
    assert_near(thumb(&harness, "Input white").x, plot.right(), "above the range: at the end");
    assert_near(thumb(&harness, "Gamma").x, plot.center().x, "an unset gamma draws at 1.00");
    assert_eq!(node(&harness, Role::SpinButton, "Input black").value().as_deref(), Some("-10"), "the field shows the host's value");
    let from = thumb(&harness, "Input black");
    drag(&mut harness, from, from + vec2(40.0, 0.0));
    assert!(harness.state().events.is_empty(), "{:?}", harness.state().kinds());
}

#[test]
fn inside_a_dialog_scope_the_editor_reports_updates_only_and_disabling_mid_drag_cancels() {
    let mut harness = harness(Host { scoped: true, ..Host::new(IDENTITY) }, Direction::Ltr);
    let from = thumb(&harness, "Output white");
    drag(&mut harness, from, from - vec2(40.0, 0.0));
    assert!(!harness.state().events.is_empty());
    assert!(harness.state().kinds().iter().all(|k| *k == "Update"), "{:?}", harness.state().kinds());

    let mut harness = rig(IDENTITY);
    let from = thumb(&harness, "Input white");
    press(&mut harness, from);
    move_to(&mut harness, from - vec2(30.0, 0.0));
    harness.state_mut().enabled = false;
    harness.run();
    assert!(matches!(harness.state().events.last(), Some(EditEvent::Cancel { reason: CancelReason::Disabled, .. })), "{:?}", harness.state().kinds());
    assert_eq!(harness.state().levels, IDENTITY);
}

#[test]
fn the_stories_name_every_control() {
    for story in crate::story::registry().into_iter().filter(|s| s.id.starts_with("composites/levels-editor/")) {
        let mut args = (story.args)();
        let mut harness = Harness::builder()
            .with_size([480.0, 640.0])
            .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
            .build_ui(move |ui| (story.show)(ui, &mut args));
        harness.run();
        a11y::assert_labelled(&harness);
        assert_eq!(harness.query_all_by(|n| n.role() == Role::Slider).count(), 5, "{}", story.id);
        assert_eq!(harness.query_all_by(|n| n.role() == Role::SpinButton).count(), 5, "{}", story.id);
    }
}
