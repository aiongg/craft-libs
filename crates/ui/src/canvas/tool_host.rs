//! `ToolHost` (8.3): maps a canvas's background press, hover and keys to the calls of a [`CanvasTool`].
//!
//! The host runs after `Canvas::show`, with the [`CanvasOutput`] the canvas returned:
//!
//! 1. `CanvasOutput::press` is the press the arena's winner left for the background: a press on a canvas control
//!    part never reaches the tool. One pass carries one phase; a click inside one pass reaches the tool as `press` then
//!    `release`, and a press that moved past slop before it came up as `press`, `drag` and `release`: the tool decides
//!    a drag from whether `drag` was called.
//! 2. `takes(button, mods)` decides whether a button starts a gesture. A middle-button pan belongs to the canvas.
//! 3. Escape during a drag arrives as a cancelled press ([`PressCancel`]) and calls `cancel` once.
//! 4. Keys the canvas holds as intents (arrows, Delete, Enter, Escape) go to `key` while the canvas has focus; a
//!    text part inside the canvas takes its own keys before they are routed here.
//! 5. The tool paints over the canvas and requests its cursor at tool priority.
//!
//! The host keeps its state under the canvas's id (`CanvasOutput::response.id`), so two canvases never share a
//! gesture, and hands the tool the canvas's view from the output ([`ToolContext::space`]). A tool changes the view
//! with [`ToolContext::set_view`]; the host keeps it inside the document as the canvas does (Overscroll off) and
//! reports the last one in [`ToolHostResponse::view`]. A view set with [`ToolContext::ease_view`] eases in the
//! canvas's one zoom ease, which the canvas shows from its next pass. `run` also paints the tool's readout;
//! [`ToolHostResponse::readout`] repeats it as data.
//!
//! Space and the app's momentary Hand: while a gesture runs the host withholds Space from the app's shortcuts for
//! the tool, so a Space pressed mid-drag never starts a momentary Hand; the tool reads it with
//! [`ToolContext::held`]. A press of Space away from a gesture reaches the app as usual. [`ToolHost::momentary`]
//! takes the app's [`Momentary`] set: while a momentary key is held the host runs the set's Hand or Zoom tool in
//! place of the selected tool, and a gesture keeps the tool it began with. An app that switches tools itself from
//! other input asks [`ToolHost::in_gesture`] first and keeps the tool it started the gesture with.
//!
//! ```no_run
//! use craft_ui::canvas::{Canvas, CanvasTool, Momentary, ToolHost};
//! use craft_ui::contract::ShowContainer as _;
//! # fn demo(ui: &mut egui::Ui, canvas: Canvas, tool: &mut dyn CanvasTool, momentary: &mut Momentary, view: &mut craft_behavior::CanvasView) {
//! let shown = canvas.show(ui, |_, _| {});
//! let reply = ToolHost::new().momentary(momentary).run(ui, &shown.output, tool);
//! if let Some(next) = reply.view { *view = next; }
//! # }
//! ```

use super::momentary::Momentary;
use super::readout::paint_readout;
use super::tool::{CanvasTool, ToolAction, ToolContext, ToolKey, ToolPointer, ToolRequests};
use super::tools::HandTool;
use super::zoom_motion::{self, Steer};
use super::{CanvasOutput, CanvasSpace};
use crate::primitives::{ContentPainterCx, Draw, DrawEnv, PaintRecord};
use crate::runtime::env::{Env, Label};
use crate::runtime::memory::{global, with_session};
use crate::runtime::pointer::{self, PointerFrame};
use crate::runtime::{cursor, frame, keys};
use crate::style::Ink;
use craft_behavior::{
    CancelReason, CanvasView, Cursor, CursorPriority, GesturePhase, Granularity, Intent, PointerButton, PointerSample, PressCancel, PressPhase,
    PressStart, PressState,
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
    #[non_exhaustive]
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
    #[non_exhaustive]
    Move {
        /// The pointer.
        pos: Pos2,
        /// The pen's reading.
        sample: PointerSample,
        /// The modifiers held.
        mods: Mods,
    },
    /// The button came up.
    #[non_exhaustive]
    Up {
        /// The pointer.
        pos: Pos2,
        /// The modifiers held.
        mods: Mods,
    },
}

impl ToolInput {
    /// A mouse press of `button` at `pos`, with no modifiers.
    pub const fn down(button: PointerButton, pos: Pos2) -> Self {
        ToolInput::Down { button, pos, sample: PointerSample::new(1.0), mods: Mods::NONE }
    }

    /// A mouse move to `pos`, with no modifiers.
    pub const fn moved(pos: Pos2) -> Self {
        ToolInput::Move { pos, sample: PointerSample::new(1.0), mods: Mods::NONE }
    }

    /// The button coming up at `pos`, with no modifiers.
    pub const fn up(pos: Pos2) -> Self {
        ToolInput::Up { pos, mods: Mods::NONE }
    }

    /// The input with the modifiers `held`.
    #[must_use]
    pub fn mods(mut self, held: Mods) -> Self {
        match &mut self {
            ToolInput::Down { mods, .. } | ToolInput::Move { mods, .. } | ToolInput::Up { mods, .. } => *mods = held,
        }
        self
    }

    /// The input with the pen's reading `pen`; a button coming up has none.
    #[must_use]
    pub fn sample(mut self, pen: PointerSample) -> Self {
        match &mut self {
            ToolInput::Down { sample, .. } | ToolInput::Move { sample, .. } => *sample = pen,
            ToolInput::Up { .. } => {}
        }
        self
    }
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
#[must_use = "apply the view and the edge overshoot to the app's view and read the gesture"]
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
    /// The values the tool wants beside the pointer. The host has painted them already; this is the data for
    /// automation.
    pub readout: Option<ToolReadout>,
    /// The view a tool set with [`ToolContext::set_view`] this pass (the last one, inside the document with Overscroll
    /// off), else the canvas's own ([`CanvasOutput::view`]): the one view the app copies into its own. A view that
    /// eases is its target; the canvas shows the way there.
    pub view: Option<CanvasView>,
}

#[derive(Clone, Copy, Debug)]
struct Active {
    button: PointerButton,
    last: ToolPointer,
    /// Started by [`ToolHost::inject`]: the pointer neither moves nor ends it.
    injected: bool,
    /// The tool has had a `drag` call in this gesture.
    dragged: bool,
}

/// What the host keeps between passes, in a session slot keyed by the canvas's id.
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
    space: Option<CanvasSpace>,
    /// The view a tool set while [`ToolHost::cancel`] ended its gesture outside a pass: the next pass reports it.
    cancelled_view: Option<CanvasView>,
}

fn state_id(canvas: Id) -> Id {
    global("craft-ui tool host", canvas)
}

/// The on-canvas tool host. Built each pass, shown once with the canvas's output and the tool.
#[derive(Default)]
pub struct ToolHost<'m> {
    disabled: bool,
    momentary: Option<&'m mut Momentary>,
}

impl<'m> ToolHost<'m> {
    /// A host that runs the tool of the canvas whose output it is given; the canvas's id keys its state.
    pub fn new() -> Self {
        ToolHost::default()
    }

    /// Whether the host runs the tool. A disabled host cancels a running gesture and delivers nothing; the press it
    /// was disabled during stays spent, so enabling the host mid-press starts no gesture.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.disabled = !enabled;
        self
    }

    /// Runs the momentary tools of `momentary` in place of the selected tool while their keys are held (Space: Hand;
    /// with Cmd or Alt: Zoom). A gesture keeps the tool it began with, and a flick released with the momentary Hand
    /// keeps gliding after the key comes up. The app feeds the set the fired [`MomentaryCommand`](super::MomentaryCommand)s.
    #[must_use]
    pub fn momentary(mut self, momentary: &'m mut Momentary) -> Self {
        self.momentary = Some(momentary);
        self
    }

    /// Delivers the pass's pointer and key events to `tool`, paints it over the canvas, and paints the readout it
    /// asked for beside the pointer.
    pub fn run(self, ui: &Ui, canvas: &CanvasOutput, tool: &mut dyn CanvasTool) -> ToolHostResponse {
        let ToolHost { disabled, momentary } = self;
        let ctx = ui.ctx().clone();
        let env = Env::of(ui);
        let key = state_id(canvas.response.id);
        let mut state = with_session(&ctx, key, |slot: &mut HostState| {
            let seen = slot.clone();
            slot.queued.clear();
            seen
        });
        let queued = std::mem::take(&mut state.queued);
        let cancelled_view = state.cancelled_view.take();
        let space = canvas.space;
        let down = keys::down(&ctx);
        let rect = canvas.response.rect;
        let input = pointer::frame(&ctx);
        let mut glide = None;
        let tool: &mut dyn CanvasTool = match momentary {
            Some(momentary) => {
                let (tool, hand) = momentary.substitute(tool, input.mods, state.gesture.is_some());
                glide = hand;
                tool
            }
            None => tool,
        };
        let mut run = Run {
            tool: &mut *tool,
            space,
            slop: input.slop,
            down: &down,
            rect,
            mods: input.mods,
            time: input.time,
            state: &mut state,
            response: ToolHostResponse::default(),
            requests: ToolRequests::default(),
            cancelled: false,
        };
        run.call(None, |tool, cx| tool.each_pass(cx));
        if let Some(hand) = glide {
            run.glide(hand);
        }
        if disabled {
            run.cancel(CancelReason::Dismissed);
            run.state.spent |= canvas.press.phase.is_press();
        } else {
            run.phase(&canvas.press, &input);
            for event in queued {
                run.injected(event);
            }
            run.keys(&ctx, canvas.response.id, env.direction.is_rtl());
        }
        let Run { mut response, requests, .. } = run;
        if response.view.is_none() {
            response.view = cancelled_view;
        }
        // A tool's view goes through the canvas's clamp, and its ease through the canvas's one zoom ease; a view that
        // jumps stops that ease.
        if let Some(view) = response.view.as_mut() {
            *view = canvas.clamped(*view);
            let steer = match (requests.ease, canvas.space) {
                (Some(ease), Some(drawn)) => Steer::Ease { from: drawn.view, anchor: ease.anchor },
                _ => Steer::Stop,
            };
            zoom_motion::steer(&ctx, canvas.response.id, steer);
        }
        // The canvas's own view, when no tool moved the view: the app copies one view from the host.
        if response.view.is_none() {
            response.view = canvas.view;
        }
        let phase = canvas.press.phase;
        let over = phase.is_press() || matches!(phase, PressPhase::Hover) || state.gesture.is_some();
        let current = state.gesture.map(|active| active.last).or_else(|| over.then(|| hover_pointer(&input, canvas.press)).flatten());
        let context = || ToolContext::new(rect, space).pointer(current).mods(input.mods).time(input.time).slop(input.slop).down(&down);
        paint_tool(ui, &env, tool, context());
        let cx = context();
        if over {
            let wanted = tool.cursor(&cx);
            cursor::request(&ctx, wanted.clone(), CursorPriority::Tool);
            response.cursor = Some(wanted);
        }
        if let (Some(rows), Some(at)) = (requests.readout, current) {
            paint_readout(&ctx, at.pos, &rows);
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
        } else if state.gesture.is_some() {
            keys::claim(&ctx, canvas.response.id, Chord::new(Mods::NONE, Key::Space));
        }
        state.rect = Some(rect);
        state.space = space;
        with_session(&ctx, key, |slot: &mut HostState| {
            let later = std::mem::take(&mut slot.queued);
            *slot = state;
            slot.queued.extend(later);
        });
        response
    }

    /// Queues an automation pointer event for the host of the canvas `canvas` (`CanvasOutput::response.id`); it is
    /// delivered on the next pass, after the pointer's own events. A `Down` while a gesture runs is dropped.
    pub fn inject(ctx: &Context, canvas: Id, input: ToolInput) {
        with_session(ctx, state_id(canvas), |state: &mut HostState| state.queued.push(input));
        frame::request_repaint(ctx);
    }

    /// Ends the gesture in progress on the canvas `canvas` now, without a commit (the app switched tools or
    /// documents, a modal opened). `tool` is the tool that holds the gesture, so call it before the app replaces
    /// it; it runs against the canvas of the last pass. Returns whether a gesture was cancelled.
    pub fn cancel(ctx: &Context, canvas: Id, tool: &mut dyn CanvasTool, reason: CancelReason) -> bool {
        let taken = with_session(ctx, state_id(canvas), |slot: &mut HostState| slot.gesture.take().map(|active| (active, slot.rect, slot.space)));
        let Some((active, rect, space)) = taken else { return false };
        let down = keys::down(ctx);
        let mut cx = ToolContext::new(rect.unwrap_or(Rect::NOTHING), space).pointer(Some(active.last)).down(&down);
        tool.cancel(reason, &mut cx);
        if let Some(view) = cx.take_requests().view {
            with_session(ctx, state_id(canvas), |state: &mut HostState| state.cancelled_view = Some(view));
            frame::request_repaint(ctx);
        }
        true
    }

    /// Whether the canvas `canvas` has a gesture in progress: the app finishes a drag with the tool it started
    /// with, whatever tool a key or command selects meanwhile, and starts no momentary tool during one.
    pub fn in_gesture(ctx: &Context, canvas: Id) -> bool {
        with_session(ctx, state_id(canvas), |state: &mut HostState| state.gesture.is_some())
    }
}

/// Paints the tool over the canvas, clipped to it.
fn paint_tool(ui: &Ui, env: &Env, tool: &dyn CanvasTool, cx: ToolContext<'_>) {
    let painter = ui.painter().with_clip_rect(cx.rect);
    let record = RefCell::new(PaintRecord::default());
    let draw = Draw::new(&painter, &record, DrawEnv { tokens: &env.tokens, direction: env.direction, ink: Ink::default() });
    let content = ContentPainterCx::new(painter.clone(), &env.tokens, env.direction);
    tool.paint(&draw, &cx.painter(&content));
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

/// The sample of a press at `pos` taken at `time`: the button the press runs with, and its sample.
fn press_pointer(start: PressStart, pos: Pos2, time: f64, press: &PressState) -> ToolPointer {
    ToolPointer::new(pos, start.button, time).mods(press.mods).sample(press.sample.unwrap_or(PointerSample::new(1.0))).clicks(start.clicks)
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
    space: Option<CanvasSpace>,
    slop: f32,
    down: &'a [Key],
    rect: Rect,
    mods: Mods,
    time: f64,
    state: &'a mut HostState,
    response: ToolHostResponse,
    requests: ToolRequests,
    /// A gesture was cancelled in this pass: its Escape is not a key for the tool.
    cancelled: bool,
}

impl<'a> Run<'a> {
    fn context(&self, pointer: Option<ToolPointer>) -> ToolContext<'a> {
        ToolContext::new(self.rect, self.space).pointer(pointer).mods(self.mods).time(self.time).slop(self.slop).down(self.down)
    }

    /// Takes what a call asked for. A view it set is the view the calls after it in the pass start from.
    fn absorb(&mut self, mut cx: ToolContext<'_>) {
        let asked = cx.take_requests();
        self.requests.readout = asked.readout.or(self.requests.readout.take());
        self.requests.repaint |= asked.repaint;
        self.requests.keep_keys |= asked.keep_keys;
        if let Some(view) = asked.view {
            if let Some(space) = self.space.as_mut() {
                space.view = view;
            }
            self.response.view = Some(view);
            self.requests.ease = asked.ease;
        }
    }

    fn call<R>(&mut self, pointer: Option<ToolPointer>, run: impl FnOnce(&mut dyn CanvasTool, &mut ToolContext<'_>) -> R) -> R {
        let mut cx = self.context(pointer);
        let out = run(&mut *self.tool, &mut cx);
        self.absorb(cx);
        out
    }

    /// Advances the glide of a momentary Hand that is not the tool of this pass.
    fn glide(&mut self, hand: &mut HandTool) {
        let mut cx = self.context(None);
        hand.each_pass(&mut cx);
        self.absorb(cx);
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
                self.state.gesture = Some(Active { button: pointer.button, last: pointer, injected, dragged: false });
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
        active.dragged = true;
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
                self.begin(press_pointer(start, start.origin, start.time, press), false);
            }
            PressPhase::DragStart { start, pos, .. } | PressPhase::Drag { start, pos, .. } => {
                if self.state.gesture.is_none() && !self.state.spent {
                    self.begin(press_pointer(start, start.origin, start.time, press), false);
                }
                self.drag(press_pointer(start, pos, self.time, press));
            }
            PressPhase::Release { start, pos, click, .. } => {
                if self.state.gesture.is_none() && !self.state.spent {
                    self.begin(press_pointer(start, start.origin, start.time, press), false);
                }
                let pointer = press_pointer(start, pos, self.time, press);
                // A press that moved past slop and came up within one pass is still a drag.
                if !click && self.state.gesture.is_some_and(|active| !active.dragged) {
                    self.drag(pointer);
                }
                self.end(pointer);
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
