//! `foundations/edit/log`: a fake slider and a fake text field wired to [`Gesture`], their
//! events in a log, and a [`GestureHost`] keeping undo steps for two documents.

use std::collections::VecDeque;
use std::fmt::Debug;
use std::time::Duration;

use craft_theme::TypeRole;
use egui::accesskit::Role;
use egui::{Align2, Event, EventFilter, Key, Modifiers, Response, Sense, Ui, pos2, vec2};

use super::{CancelReason, Cost, EditEvent, EditPolicy, EditResponse, Gesture, GestureHost, UndoMarks, UpdateRate, cancel_all};
use crate::primitives::{Content, Gap, Interactive, Row, Stack, Surface, SurfaceKind, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf};

const RATES: &[&str] = &["realtime", "per frame", "debounced 300 ms"];

/// Events kept in the log.
const LOG_LEN: usize = 20;

/// The slider's range.
pub(super) const SLIDER_MAX: f32 = 100.0;

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "foundations/edit/log",
        level: Level::Foundations,
        component: "Edit lifecycle",
        name: "Event log",
        args: || Args::new().with_choice("update rate", RATES, 0).with_bool("cancellable", true).with_bool("on release only", false),
        show,
    }]
}

/// The story's policy for both controls.
fn policy(args: &Args) -> EditPolicy {
    let update = match args.choice("update rate") {
        "per frame" => UpdateRate::PerFrame,
        "debounced 300 ms" => UpdateRate::Debounced(Duration::from_millis(300)),
        _ => UpdateRate::Realtime,
    };
    EditPolicy { update, cost: Cost::Cheap, cancellable: args.bool("cancellable"), on_release_only: args.bool("on release only") }
}

/// A document of the story: two fields and an undo history of whole snapshots.
#[derive(Clone, Debug)]
struct Doc {
    name: &'static str,
    level: f32,
    label: String,
    undo: Vec<(f32, String)>,
}

impl Doc {
    fn new(name: &'static str, level: f32, label: &str) -> Self {
        Self { name, level, label: label.to_owned(), undo: Vec::new() }
    }

    /// Applies `edit` as one undo step.
    fn edit(&mut self, edit: impl FnOnce(&mut Self)) {
        self.undo.push((self.level, self.label.clone()));
        edit(self);
    }

    fn undo(&mut self) -> bool {
        let Some((level, label)) = self.undo.pop() else { return false };
        self.level = level;
        self.label = label;
        true
    }
}

impl UndoMarks for Doc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some((level, label)) = self.undo.get(mark).cloned() {
            self.level = level;
            self.label = label;
            self.undo.truncate(mark);
        }
    }
}

#[derive(Clone, Debug)]
struct State {
    docs: [Doc; 2],
    current: usize,
    host: GestureHost<Doc>,
    /// Newest first.
    log: VecDeque<String>,
}

impl Default for State {
    fn default() -> Self {
        Self { docs: [Doc::new("A", 40.0, "Layer 1"), Doc::new("B", 75.0, "Background")], current: 0, host: GestureHost::new(), log: VecDeque::new() }
    }
}

impl State {
    fn doc(&mut self) -> &mut Doc {
        &mut self.docs[self.current % 2]
    }

    fn log(&mut self, line: String) {
        self.log.push_front(line);
        self.log.truncate(LOG_LEN);
    }

    fn handle<T: Debug>(&mut self, control: &str, event: Option<EditEvent<T>>, apply: impl FnMut(&mut Doc, T)) {
        let Some(event) = event else { return };
        self.log(format!("{control}: {}", describe(&event)));
        let doc = &mut self.docs[self.current % 2];
        self.host.handle(doc, event, apply);
    }

    /// Undo: during a gesture it cancels the gesture instead of undoing a step.
    fn undo(&mut self, ui: &Ui) {
        match self.host.active() {
            Some(id) => {
                let doc = &mut self.docs[self.current % 2];
                self.host.cancel(doc, id);
                cancel_all(ui.ctx(), CancelReason::UndoDuringGesture);
                self.log(format!("undo: cancel_all({:?})", CancelReason::UndoDuringGesture));
            }
            None if self.doc().undo() => self.log("undo: one step".to_owned()),
            None => {}
        }
    }

    /// The gesture belongs to the document it began in: it is cancelled there before the switch.
    fn switch(&mut self, ui: &Ui) {
        if let Some(id) = self.host.active() {
            let doc = &mut self.docs[self.current % 2];
            self.host.cancel(doc, id);
        }
        cancel_all(ui.ctx(), CancelReason::DocumentSwitched);
        self.current = (self.current + 1) % 2;
        let name = self.doc().name;
        self.log(format!("switched to document {name}: cancel_all({:?})", CancelReason::DocumentSwitched));
    }
}

/// One log line, e.g. `Update #3 42 (shift)`.
fn describe<T: Debug>(event: &EditEvent<T>) -> String {
    match event {
        EditEvent::Begin { id } => format!("Begin {id}"),
        EditEvent::Update { id, value, modifiers } if modifiers.is_none() => format!("Update {id} {value:?}"),
        EditEvent::Update { id, value, modifiers } => format!("Update {id} {value:?} ({modifiers:?})"),
        EditEvent::Commit { id, value, changed: true } => format!("Commit {id} {value:?}"),
        EditEvent::Commit { id, value, changed: false } => format!("Commit {id} {value:?}, unchanged"),
        EditEvent::Cancel { id, reason } => format!("Cancel {id} {reason:?}"),
        EditEvent::Set { value } => format!("Set {value:?}"),
    }
}

/// The controls, the undo and switch buttons (also Cmd/Ctrl+Z and Ctrl+Tab, which work during a
/// drag; with no gesture under way, undo takes back one step), and the log.
fn show(ui: &mut Ui, args: &mut Args) {
    let policy = policy(args);
    let state_id = ui.scope_id().with("edit-story-state");
    let mut state = ui.data(|d| d.get_temp::<State>(state_id)).unwrap_or_default();
    if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
        state.undo(ui);
    }
    if ui.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Tab)) {
        state.switch(ui);
    }
    Stack::new(Gap::Stack).show(ui, |ui| {
        Row::new(Gap::Inline).show(ui, |ui| {
            let level = state.doc().level;
            let edit = slider(ui, level, policy);
            state.handle("slider", edit.event, |doc, value| doc.edit(|d| d.level = value));
            let label = state.doc().label.clone();
            let edit = text_field(ui, &label, policy);
            state.handle("field", edit.event, |doc, value| doc.edit(|d| d.label = value));
        });
        Row::new(Gap::Inline).show(ui, |ui| {
            if button(ui, "undo", "Undo during gesture").clicked() {
                state.undo(ui);
            }
            if button(ui, "switch", "Switch document").clicked() {
                state.switch(ui);
            }
        });
        let doc = state.doc();
        let status = format!("Document {}: level {}, label {:?}, {} undo steps", doc.name, doc.level, doc.label, doc.undo.len());
        ui.add(Text::new(status).content(Content::Muted));
        Surface::new(SurfaceKind::Sunken).show(ui, |ui| {
            ui.set_min_width(LOG_WIDTH);
            if state.log.is_empty() {
                ui.add(Text::new("No events yet").content(Content::Faint));
            }
            for line in &state.log {
                ui.add(Text::new(line.as_str()).role(TypeRole::Mono));
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(state_id, state));
}

const LOG_WIDTH: f32 = 420.0;
const CONTROL_WIDTH: f32 = 200.0;

/// A fake slider over 0 to [`SLIDER_MAX`]: drag horizontally (one unit per point), or the arrow
/// keys while focused.
pub(super) fn slider(ui: &mut Ui, value: f32, policy: EditPolicy) -> EditResponse<f32> {
    let tokens = Tokens::of(ui);
    let response = Interactive::new("edit-story-slider", Role::Slider, "Level")
        .sense(Sense::drag())
        .state_layer(false)
        .show(ui, vec2(CONTROL_WIDTH, tokens.size.control.md));
    let mut gesture = Gesture::load(ui, response.id, &value, policy).event_filter(EventFilter { horizontal_arrows: true, ..Default::default() });
    if response.drag_started() {
        response.request_focus();
        gesture.begin();
    }
    if response.dragged()
        && let Some(start) = gesture.start().copied()
        && let Some((pos, origin)) = ui.input(|i| i.pointer.interact_pos().zip(i.pointer.press_origin()))
    {
        gesture.update((start + (pos.x - origin.x)).round().clamp(0.0, SLIDER_MAX), ui.input(|i| i.modifiers));
    }
    if response.drag_stopped() {
        gesture.commit();
    }
    if response.has_focus() {
        let step = ui.input(|i| f32::from(u8::from(i.key_pressed(Key::ArrowRight))) - f32::from(u8::from(i.key_pressed(Key::ArrowLeft))));
        if step != 0.0 {
            gesture.set((gesture.shown() + step).clamp(0.0, SLIDER_MAX));
        }
    }
    let shown = *gesture.shown();
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_numeric_value(f64::from(shown));
        node.set_min_numeric_value(0.0);
        node.set_max_numeric_value(f64::from(SLIDER_MAX));
    });
    if ui.is_rect_visible(response.rect) {
        let rect = response.rect;
        let radius = rect.height() * 0.3;
        let (left, right) = (rect.left() + radius, rect.right() - radius);
        let painter = ui.painter();
        painter.line_segment([pos2(left, rect.center().y), pos2(right, rect.center().y)], (2.0, tokens.color.line.control_edge.to_egui()));
        let x = left + (right - left) * shown / SLIDER_MAX;
        painter.circle_filled(pos2(x, rect.center().y), radius, tokens.color.accent.solid.to_egui());
    }
    EditResponse { response, event: gesture.finish() }
}

/// A fake one-line text field: click to focus, type, Backspace, Enter to commit, Esc to cancel;
/// moving the focus away commits.
pub(super) fn text_field(ui: &mut Ui, value: &str, policy: EditPolicy) -> EditResponse<String> {
    let tokens = Tokens::of(ui);
    let response = Interactive::new("edit-story-field", Role::TextInput, "Label")
        .surface(SurfaceKind::Sunken)
        .state_layer(false)
        .show(ui, vec2(CONTROL_WIDTH, tokens.size.control.md));
    if response.clicked() {
        response.request_focus();
    }
    let mut gesture = Gesture::load(ui, response.id, &value.to_owned(), policy);
    if response.has_focus() {
        for event in ui.input(|i| i.events.clone()) {
            type_key(&mut gesture, &event);
        }
    }
    let shown = gesture.shown().clone();
    ui.ctx().accesskit_node_builder(response.id, |node| node.set_value(shown.as_str()));
    if ui.is_rect_visible(response.rect) {
        let rect = response.rect.shrink2(vec2(tokens.space.pad.control, 0.0));
        let color = tokens.color.text.normal.to_egui();
        let text = ui.painter().text(rect.left_center(), Align2::LEFT_CENTER, &shown, tokens.font(TypeRole::Body), color);
        if response.has_focus() {
            ui.painter().vline(text.right() + 1.0, text.y_range(), (1.0, color));
        }
    }
    EditResponse { response, event: gesture.finish() }
}

/// One input event of the focused text field.
fn type_key(gesture: &mut Gesture<String>, event: &Event) {
    let edit = |gesture: &mut Gesture<String>, change: &dyn Fn(&mut String), modifiers: Modifiers| {
        gesture.begin();
        let mut text = gesture.shown().clone();
        change(&mut text);
        gesture.update(text, modifiers);
    };
    match event {
        Event::Text(typed) => edit(gesture, &|text| text.push_str(typed), Modifiers::NONE),
        Event::Key { key: Key::Backspace, pressed: true, modifiers, .. } => edit(
            gesture,
            &|text| {
                text.pop();
            },
            *modifiers,
        ),
        Event::Key { key: Key::Enter, pressed: true, .. } => gesture.commit(),
        _ => {}
    }
}

/// A plain raised button with a centred label.
fn button(ui: &mut Ui, salt: &str, label: &str) -> Response {
    let tokens = Tokens::of(ui);
    let font = tokens.font(TypeRole::Label);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, tokens.color.text.normal.to_egui());
    let size = vec2(galley.size().x + 2.0 * tokens.space.pad.control, tokens.size.control.md);
    let response = Interactive::new(("edit-story-button", salt), Role::Button, label).surface(SurfaceKind::Raised).show(ui, size);
    let pos = response.rect.center() - galley.size() / 2.0;
    ui.painter().galley(pos, galley, egui::Color32::PLACEHOLDER);
    response
}
