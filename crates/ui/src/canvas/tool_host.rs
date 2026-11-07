//! `ToolHost` (8.3): maps a canvas's background press, hover and keys to the calls of a [`CanvasTool`].
//!
//! The host runs after `Canvas::show`, with the [`CanvasOutput`] the canvas returned:
//!
//! 1. `CanvasOutput::press` is the press the arena's winner left for the background: a press on a canvas control
//!    part never reaches the tool. One pass carries one phase; a press and release inside one pass reach the tool as
//!    `press` then `release`.
//! 2. `takes(button, mods)` decides whether a button starts a gesture. A middle-button pan belongs to the canvas.
//! 3. Escape during a drag arrives as a cancelled press ([`PressCancel`]) and calls `cancel` once.
//! 4. Keys the canvas holds as intents (arrows, Delete, Enter, Escape) go to `key` while the canvas has focus; a
//!    text part inside the canvas takes its own keys before they are routed here.
//! 5. The tool paints over the canvas and requests its cursor at tool priority.
//!
//! The tool reads the canvas's space through [`ToolContext::value`]: call [`ToolHost::capture_values`] with the
//! body's `Ui` inside the canvas body, where `content_env` has published the values.
//!
//! ```no_run
//! use craft_ui::canvas::{Canvas, CanvasTool, ToolHost};
//! use craft_ui::contract::ShowContainer as _;
//! # fn demo(ui: &mut egui::Ui, canvas: Canvas, tool: &mut dyn CanvasTool) {
//! let mut host = ToolHost::new("Document");
//! let shown = canvas.show(ui, |ui, _body| host.capture_values(ui));
//! let reply = host.show(ui, &shown.output, tool);
//! # let _ = reply;
//! # }
//! ```

use super::CanvasOutput;
use super::tool::{CanvasTool, ToolAction, ToolContext, ToolKey, ToolPointer, ToolRequests};
use crate::primitives::{Draw, DrawEnv, PaintRecord};
use crate::runtime::env::{AppValues, Env, Label};
use crate::runtime::memory::{global, with_session};
use crate::runtime::pointer::{self, PointerFrame};
use crate::runtime::{cursor, frame, keys};
use crate::style::Ink;
use craft_behavior::{
    CancelReason, Cursor, CursorPriority, GesturePhase, Granularity, Intent, PointerButton, PointerSample, PressCancel, PressPhase, PressStart,
    PressState,
};
use craft_keymap::{Chord, Key, Mods};
use egui::{Context, Id, Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;

/// A pointer event from automation, queued with [`ToolHost::inject`] and delivered on the next pass as if the
/// pointer did it. Positions are screen points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ToolInput {
    /// A press of `button`. The host starts nothing when the tool takes no such button with `mods`.
    Down {
        /// The button.
        button: PointerButton,
        /// The pointer.
        pos: Pos2,
        /// The pen's reading.
        sample: PointerSample,
        /// The modifiers held.
        mods: Mods,
    },
    /// The pointer moved: a drag while a gesture runs, else a hover.
    Move {
        /// The pointer.
        pos: Pos2,
        /// The pen's reading.
        sample: PointerSample,
        /// The modifiers held.
        mods: Mods,
    },
    /// The button came up.
    Up {
        /// The pointer.
        pos: Pos2,
        /// The modifiers held.
        mods: Mods,
    },
}

/// Labelled values for the pointer's side, as a tool handed them over with [`ToolContext::readout`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToolReadout {
    /// The pointer on screen.
    pub at: Pos2,
    /// The rows: a label and its value.
    pub rows: Vec<(Label, String)>,
}

/// What the host did in a pass.
#[derive(Clone, Debug, Default, PartialEq)]
#[must_use = "apply the edge overshoot to the app's view and read the gesture"]
#[non_exhaustive]
pub struct ToolHostResponse {
    /// The gesture's phase, when one began, moved or ended this pass. A pass that began and ended a gesture reports
    /// the end.
    pub gesture: Option<GesturePhase>,
    /// The press the tool answered with [`ToolAction::Ignored`]: the app may use it.
    pub ignored_press: Option<ToolPointer>,
    /// The key the tool took this pass.
    pub took_key: Option<ToolKey>,
    /// How far past the canvas edge the pointer is, in screen points, while a gesture of a tool that autoscrolls
    /// runs. The host moves no view: the app scrolls its view toward the pointer.
    pub past_edge: Option<Vec2>,
    /// The cursor the tool asked for, after the host requested it.
    pub cursor: Option<Cursor>,
    /// The values the tool wants beside the pointer; the app shows them with the canvas readout.
    pub readout: Option<ToolReadout>,
}

#[derive(Clone, Copy, Debug)]
struct Active {
    button: PointerButton,
    last: ToolPointer,
    /// Started by [`ToolHost::inject`]: the pointer neither moves nor ends it.
    injected: bool,
}

/// What the host keeps between passes, in a session slot keyed by the host's label.
#[derive(Clone, Default)]
struct HostState {
    gesture: Option<Active>,
    /// The open press was refused, ignored or finished whole: its later phases are not the tool's.
    spent: bool,
    queued: Vec<ToolInput>,
    /// The tool kept the keys in the last pass.
    keep_keys: bool,
    /// The canvas of the last pass, for [`ToolHost::cancel`] outside a pass.
    rect: Option<Rect>,
    values: AppValues,
}

fn state_id(label: &Label) -> Id {
    global("craft-ui tool host", label.as_str())
}

/// The on-canvas tool host. Built each pass, shown once with the canvas's output and the tool.
pub struct ToolHost {
    label: Label,
    values: Option<AppValues>,
    enabled: bool,
}

impl ToolHost {
    /// A host named `label`: the name keys the host's state, so use one label per canvas.
    pub fn new(label: impl Into<Label>) -> Self {
        ToolHost { label: label.into(), values: None, enabled: true }
    }

    /// Reads the app values the canvas body published (`content_env`), so tools read them with
    /// [`ToolContext::value`]. Call it with the `Ui` of the canvas body. Without it a tool reads the values around
    /// the canvas.
    pub fn capture_values(&mut self, body: &Ui) {
        self.values = Some(Env::of(body).values.clone());
    }

    /// Whether the host runs the tool. A disabled host cancels a running gesture and delivers nothing.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Delivers the pass's pointer and key events to `tool` and paints it over the canvas.
    pub fn show(self, ui: &mut Ui, canvas: &CanvasOutput, tool: &mut dyn CanvasTool) -> ToolHostResponse {
        let ctx = ui.ctx().clone();
        let env = Env::of(ui);
        let key = state_id(&self.label);
        let mut state = with_session(&ctx, key, |slot: &mut HostState| {
            let seen = slot.clone();
            slot.queued.clear();
            seen
        });
        let queued = std::mem::take(&mut state.queued);
        let values = self.values.unwrap_or_else(|| env.values.clone());
        let rect = canvas.response.rect;
        let input = pointer::frame(&ctx);
        let mut run = Run {
            tool: &mut *tool,
            values: &values,
            rect,
            mods: input.mods,
            time: input.time,
            state: &mut state,
            response: ToolHostResponse::default(),
            requests: ToolRequests::default(),
            cancelled: false,
        };
        run.call(None, |tool, cx| tool.each_pass(cx));
        if self.enabled {
            run.phase(&canvas.press, &input);
            for event in queued {
                run.injected(event);
            }
            run.keys(&ctx, canvas.response.id, env.direction.is_rtl());
        } else {
            run.cancel(CancelReason::Dismissed);
        }
        let Run { mut response, requests, .. } = run;
        let phase = canvas.press.phase;
        let over = phase.is_press() || matches!(phase, PressPhase::Hover) || state.gesture.is_some();
        let current = state.gesture.map(|active| active.last).or_else(|| over.then(|| hover_pointer(&input, canvas.press)).flatten());
        let cx = ToolContext::new(rect, &values).pointer(current).mods(input.mods).time(input.time);
        paint_tool(ui, &env, tool, &cx);
        if over {
            let wanted = tool.cursor(&cx);
            cursor::request(&ctx, wanted.clone(), CursorPriority::Tool);
            response.cursor = Some(wanted);
        }
        if let (Some(rows), Some(at)) = (requests.readout, current) {
            response.readout = Some(ToolReadout { at: at.pos, rows });
        }
        if requests.repaint {
            frame::request_repaint(&ctx);
        }
        state.keep_keys = requests.keep_keys;
        if requests.keep_keys {
            for chord in kept_chords() {
                keys::claim(&ctx, canvas.response.id, chord);
            }
        }
        state.rect = Some(rect);
        state.values = values;
        with_session(&ctx, key, |slot: &mut HostState| {
            let later = std::mem::take(&mut slot.queued);
            *slot = state;
            slot.queued.extend(later);
        });
        response
    }

    /// Queues an automation pointer event for the host named `label`; it is delivered on the next pass, after the
    /// pointer's own events. A `Down` while a gesture runs is dropped.
    pub fn inject(ctx: &Context, label: impl Into<Label>, input: ToolInput) {
        with_session(ctx, state_id(&label.into()), |state: &mut HostState| state.queued.push(input));
        frame::request_repaint(ctx);
    }

    /// Ends the gesture in progress on the host named `label` now, without a commit (the app switched tools or
    /// documents, a modal opened). `tool` is the tool that holds the gesture, so call it before the app replaces
    /// it; it runs against the canvas of the last pass. Returns whether a gesture was cancelled.
    pub fn cancel(ctx: &Context, label: impl Into<Label>, tool: &mut dyn CanvasTool, reason: CancelReason) -> bool {
        let taken = with_session(ctx, state_id(&label.into()), |slot: &mut HostState| {
            slot.gesture.take().map(|active| (active, slot.rect, slot.values.clone()))
        });
        let Some((active, rect, values)) = taken else { return false };
        let mut cx = ToolContext::new(rect.unwrap_or(Rect::NOTHING), &values).pointer(Some(active.last));
        tool.cancel(reason, &mut cx);
        true
    }

    /// Whether the host named `label` has a gesture in progress: the app finishes a drag with the tool it started
    /// with, whatever tool a key or command selects meanwhile.
    pub fn in_gesture(ctx: &Context, label: impl Into<Label>) -> bool {
        with_session(ctx, state_id(&label.into()), |state: &mut HostState| state.gesture.is_some())
    }
}

/// Paints the tool over the canvas, clipped to it.
fn paint_tool(ui: &Ui, env: &Env, tool: &dyn CanvasTool, cx: &ToolContext<'_>) {
    let painter = ui.painter().with_clip_rect(cx.rect);
    let record = RefCell::new(PaintRecord::default());
    let draw = Draw::new(&painter, &record, DrawEnv { tokens: &env.tokens, direction: env.direction, ink: Ink::default() });
    tool.paint(&draw, cx);
}

/// The hover sample of the pass: the pointer's latest position with the pen's reading.
fn hover_pointer(frame: &PointerFrame, press: PressState) -> Option<ToolPointer> {
    let pos = frame.pos?;
    Some(
        ToolPointer::new(pos, PointerButton::Primary, frame.time)
            .mods(press.mods)
            .sample(press.sample.or(frame.sample).unwrap_or(PointerSample::new(1.0))),
    )
}

/// Every key without Ctrl or Cmd, with and without Shift: the keys a tool that kept the keyboard types with. The
/// app's chords (Cmd+Z, Cmd+S) still run.
fn kept_chords() -> impl Iterator<Item = Chord> {
    Key::ALL.iter().flat_map(|key| [Mods::NONE, Mods::SHIFT].map(|mods| Chord::new(mods, *key)))
}

/// The button a phase's press runs with, and its sample.
fn press_pointer(start: PressStart, pos: Pos2, press: &PressState) -> ToolPointer {
    ToolPointer::new(pos, start.button, start.time).mods(press.mods).sample(press.sample.unwrap_or(PointerSample::new(1.0))).clicks(start.clicks)
}

fn cancel_reason(reason: PressCancel) -> CancelReason {
    match reason {
        PressCancel::Escape => CancelReason::Escape,
        PressCancel::FocusLost => CancelReason::Blur,
        PressCancel::Disabled => CancelReason::Dismissed,
        _ => CancelReason::Removed,
    }
}

/// One pass of the host: the tool, the state and what the calls produced.
struct Run<'a> {
    tool: &'a mut dyn CanvasTool,
    values: &'a AppValues,
    rect: Rect,
    mods: Mods,
    time: f64,
    state: &'a mut HostState,
    response: ToolHostResponse,
    requests: ToolRequests,
    /// A gesture was cancelled in this pass: its Escape is not a key for the tool.
    cancelled: bool,
}

impl Run<'_> {
    fn call<R>(&mut self, pointer: Option<ToolPointer>, run: impl FnOnce(&mut dyn CanvasTool, &mut ToolContext<'_>) -> R) -> R {
        let mut cx = ToolContext::new(self.rect, self.values).pointer(pointer).mods(self.mods).time(self.time);
        let out = run(&mut *self.tool, &mut cx);
        let asked = cx.take_requests();
        self.requests.readout = asked.readout.or(self.requests.readout.take());
        self.requests.repaint |= asked.repaint;
        self.requests.keep_keys |= asked.keep_keys;
        out
    }

    fn report(&mut self, phase: GesturePhase) {
        let ends = matches!(phase, GesturePhase::Commit | GesturePhase::Cancel);
        if ends || self.response.gesture.is_none() {
            self.response.gesture = Some(phase);
        }
    }

    /// Offers a press to the tool when it takes the button; a refused, ignored or whole press is spent.
    fn begin(&mut self, pointer: ToolPointer, injected: bool) {
        if !self.tool.takes(pointer.button, pointer.mods) {
            self.state.spent = true;
            return;
        }
        match self.call(Some(pointer), |tool, cx| tool.press(&pointer, cx)) {
            ToolAction::Claimed => {
                self.state.gesture = Some(Active { button: pointer.button, last: pointer, injected });
                self.report(GesturePhase::Begin);
            }
            ToolAction::Done => {
                self.state.spent = true;
                self.report(GesturePhase::Commit);
            }
            _ => {
                self.state.spent = true;
                self.response.ignored_press = Some(pointer);
            }
        }
    }

    fn drag(&mut self, pointer: ToolPointer) {
        let Some(active) = self.state.gesture.as_mut() else { return };
        active.last = pointer;
        self.call(Some(pointer), |tool, cx| tool.drag(&pointer, cx));
        self.report(GesturePhase::Update);
        if self.tool.autoscrolls() {
            let past = pointer.pos - self.rect.clamp(pointer.pos);
            if past != Vec2::ZERO {
                self.response.past_edge = Some(past);
            }
        }
    }

    fn end(&mut self, pointer: ToolPointer) {
        if self.state.gesture.take().is_some() {
            self.call(Some(pointer), |tool, cx| tool.release(&pointer, cx));
            self.report(GesturePhase::Commit);
        }
    }

    fn cancel(&mut self, reason: CancelReason) {
        if let Some(active) = self.state.gesture.take() {
            self.call(Some(active.last), |tool, cx| tool.cancel(reason, cx));
            self.report(GesturePhase::Cancel);
            self.cancelled = true;
        }
    }

    fn hover(&mut self, pointer: ToolPointer) {
        self.call(Some(pointer), |tool, cx| tool.hover(&pointer, cx));
    }

    /// The canvas background's press phase of this pass. A pass carries one phase, so a press that moved or
    /// released in the pass it began in starts here.
    fn phase(&mut self, press: &PressState, frame: &PointerFrame) {
        if self.state.gesture.is_some_and(|active| active.injected) {
            return;
        }
        match press.phase {
            PressPhase::Idle => self.state.spent = false,
            PressPhase::Hover => {
                self.state.spent = false;
                if let Some(pointer) = hover_pointer(frame, *press) {
                    self.hover(pointer);
                }
            }
            PressPhase::Down { start, .. } => {
                self.state.spent = false;
                self.begin(press_pointer(start, start.origin, press), false);
            }
            PressPhase::DragStart { start, pos, .. } | PressPhase::Drag { start, pos, .. } => {
                if self.state.gesture.is_none() && !self.state.spent {
                    self.begin(press_pointer(start, start.origin, press), false);
                }
                self.drag(press_pointer(start, pos, press));
            }
            PressPhase::Release { start, pos, .. } => {
                if self.state.gesture.is_none() && !self.state.spent {
                    self.begin(press_pointer(start, start.origin, press), false);
                }
                self.end(press_pointer(start, pos, press));
                self.state.spent = false;
            }
            PressPhase::Cancel { reason, .. } => {
                self.cancel(cancel_reason(reason));
                self.state.spent = false;
            }
            _ => {}
        }
    }

    fn injected(&mut self, input: ToolInput) {
        let own = self.state.gesture.is_none_or(|active| active.injected);
        match input {
            ToolInput::Down { button, pos, sample, mods } if self.state.gesture.is_none() => {
                let pointer = ToolPointer::new(pos, button, self.time).mods(mods).sample(sample).clicks(1);
                self.begin(pointer, true);
                self.state.spent = false;
            }
            ToolInput::Move { pos, sample, mods } if own => {
                let button = self.state.gesture.map_or(PointerButton::Primary, |active| active.button);
                let pointer = ToolPointer::new(pos, button, self.time).mods(mods).sample(sample);
                if self.state.gesture.is_some() {
                    self.drag(pointer);
                } else {
                    self.hover(pointer);
                }
            }
            ToolInput::Up { pos, mods } if own => {
                if let Some(active) = self.state.gesture {
                    self.end(ToolPointer::new(pos, active.button, self.time).mods(mods));
                }
            }
            _ => {}
        }
    }

    /// Offers one key to the tool; the intent is taken from the canvas only when the tool takes the key.
    fn offer(&mut self, intents: &mut keys::Intents, read: impl FnOnce(&mut keys::Intents) -> Option<ToolKey>) -> bool {
        let mut next = intents.clone();
        let Some(key) = read(&mut next) else { return false };
        if !self.call(None, |tool, cx| tool.key(&key, cx)) {
            return false;
        }
        *intents = next;
        self.response.took_key = Some(key);
        true
    }

    /// The keys the canvas holds: intents routed to it while it has focus, and the plain keys a tool kept.
    fn keys(&mut self, ctx: &Context, canvas: Id, rtl: bool) {
        let mut intents = keys::intents_of(ctx, Some(canvas), rtl);
        let mut taken = self.offer(&mut intents, |intents| intents.take_nudge().map(|(direction, mods)| ToolKey::Nudge { direction, mods }));
        for forward in [true, false] {
            taken |= self.offer(&mut intents, |intents| {
                intents.take_exact(&Intent::delete(forward, Granularity::Unit)).then_some(ToolKey::Delete { forward })
            });
        }
        taken |= self.offer(&mut intents, |intents| intents.take(Intent::Submit).then_some(ToolKey::Submit));
        if !self.cancelled {
            taken |= self.offer(&mut intents, |intents| intents.take(Intent::Cancel).then_some(ToolKey::Cancel));
        }
        if taken {
            keys::settle(ctx, Some(canvas), &intents);
        }
        if self.state.keep_keys {
            for chord in kept_chords().filter(|chord| keys::take_claimed(ctx, canvas, *chord)) {
                let kept = ToolKey::Kept(chord);
                if self.call(None, |tool, cx| tool.key(&kept, cx)) {
                    self.response.took_key = Some(kept);
                }
            }
        }
    }
}
