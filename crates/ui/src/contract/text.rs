//! The text part of the pipeline (18.2): `PartsCx::text` registers the part with its typing run, and step 8b runs the
//! text adapter, then the typing run over what the adapter recorded, and applies the run's events through `Cx`.

use super::cx::{Cx, PartsCx, gesture_key};
use super::{Component, Edit as _, Focus as _, Overlays as _, ValueOf};
use crate::primitives::text_adapter::{self as adapter, End, TextState};
use crate::primitives::{TextKey, TextOptions, TextOutcome};
use crate::runtime::access::text as text_access;
use crate::runtime::keys;
use crate::runtime::memory::sub;
use crate::runtime::motion::Animate as _;
use craft_behavior::{CancelReason, Caret, Codec, EditRun, Ended, EscapeRule, Escaped, Gesture, NotAValue, PartKey, Typed, TypingRun};
use egui::{Rect, Ui};
use std::any::Any;
use std::sync::Arc;

/// The id of the text adapter's state in the component's memory.
fn state_id(id: egui::Id) -> egui::Id {
    sub(id, "text_state")
}

/// What `PartsCx::text` leaves for step 8b.
#[derive(Default)]
pub(crate) struct TextStep {
    /// A press elsewhere or another focus loss asks the run to commit now.
    pub commit_now: bool,
    pub decl: Option<TextDecl>,
    pub editor: Option<super::editor::EditorDecl>,
}

pub(crate) struct TextDecl {
    pub key: PartKey,
    pub rect: Rect,
    pub shown: String,
    pub options: TextOptions,
    pub caret: Caret,
    /// A `Resolve<V>` for the component's value type: the typing run, run once the adapter has recorded the pass.
    pub resolve: Box<dyn Any>,
    pub values: ValueOps,
}

/// What the typing run does with the component's value, written once for its type: the run keeps the value it last
/// showed, and a different one at the next pass was set outside the run.
#[derive(Clone, Copy)]
pub(crate) struct ValueOps {
    /// Ends the run when the value is not the one the run last showed.
    resync: fn(&mut TextState, Option<&dyn Any>),
    /// Records the value the run shows from now on.
    note: fn(&mut TextState, Option<&dyn Any>),
}

impl ValueOps {
    fn of<V: Clone + PartialEq + Send + Sync + 'static>() -> ValueOps {
        ValueOps {
            resync: |state, value| {
                let value = value.and_then(|value| value.downcast_ref::<V>());
                let seen = state.seen.as_ref().and_then(|seen| seen.downcast_ref::<V>());
                if let (Some(seen), Some(value)) = (seen, value)
                    && seen != value
                {
                    state.draft = None;
                    state.start = None;
                    state.normalised = None;
                    state.dirty = false;
                    state.invalid = false;
                }
            },
            note: |state, value| {
                state.seen = value.and_then(|value| value.downcast_ref::<V>()).map(|value| Arc::new(value.clone()) as Arc<dyn Any + Send + Sync>);
            },
        }
    }
}

enum TextEvent<V> {
    Propose(V),
    Commit(V),
    Cancel(CancelReason),
}

/// What the typing run decided this pass.
struct Resolved<V> {
    events: Vec<TextEvent<V>>,
    outcome: TextOutcome<V>,
    submitted: bool,
    escaped: bool,
}

/// The typing run over the state the adapter recorded this pass, the component's value (`None` while mixed) and
/// whether a commit is due.
type Resolve<V> = Box<dyn FnOnce(&mut TextState, Option<&V>, bool) -> Resolved<V>>;

/// Turns what the adapter recorded this pass into the typing run's events: a parsed draft applies in the pass it
/// is typed, and a commit proposes and commits in one pass. `fallback` is the value the part shows, the run's start
/// while the component's value is mixed.
fn resolver<K: Codec + Clone + 'static>(codec: K, options: &TextOptions, rect: Rect, fallback: K::Value) -> Resolve<K::Value> {
    let (live, limit, clear_on_escape) = (options.live, options.limit, options.clear_on_escape);
    Box::new(move |state, value, commit_now| {
        let value = value.unwrap_or(&fallback);
        let mut typing = TypingRun::new(codec).live(live);
        if let Some(limit) = limit {
            typing = typing.limit(limit);
        }
        if clear_on_escape {
            typing = typing.escape_rule(EscapeRule::Clear);
        }
        let started = state.start.as_ref().and_then(|start| start.downcast_ref::<K::Value>()).cloned();
        let mut run = EditRun::new(started.unwrap_or_else(|| value.clone()));
        run.draft = state.draft.clone();
        if state.invalid {
            run.error = Some(NotAValue);
        }
        let mut events = Vec::new();
        if std::mem::take(&mut state.dirty)
            && let Some(draft) = state.draft.clone()
        {
            let start = run.start.clone();
            if let Typed::Apply(applied) = typing.edit(&mut run, &start, &draft) {
                events.push(TextEvent::Propose(applied));
            }
        }
        let draft_text = run.draft.clone().unwrap_or_default();
        let (mut submitted, mut escaped) = (false, false);
        let end = match state.end.take() {
            None if commit_now && run.is_open() => Some(End::Blur),
            end => end,
        };
        match end {
            Some(End::Escape) => {
                escaped = true;
                if clear_on_escape && !run.is_open() {
                    let start = run.start.clone();
                    typing.open(&mut run, &start);
                }
                match typing.escape(&mut run) {
                    Escaped::Restore(_) => events.push(TextEvent::Cancel(CancelReason::Escape)),
                    Escaped::Cleared(cleared) => {
                        state.normalised = Some(typing.codec().normalise("", &cleared));
                        events.push(TextEvent::Commit(cleared));
                    }
                    _ => {}
                }
            }
            Some(end @ (End::Submit | End::Separator)) => match typing.commit(&mut run) {
                Ended::Commit(committed) => {
                    state.normalised = Some(typing.codec().normalise(&draft_text, &committed));
                    events.push(TextEvent::Commit(committed));
                    submitted = end == End::Submit;
                }
                Ended::Rejected => {}
                Ended::Idle => submitted = end == End::Submit,
                _ => {}
            },
            Some(End::Blur) => match typing.commit(&mut run) {
                Ended::Commit(committed) => {
                    state.normalised = Some(typing.codec().normalise(&draft_text, &committed));
                    events.push(TextEvent::Commit(committed));
                }
                Ended::Rejected => {
                    typing.cancel(&mut run);
                    events.push(TextEvent::Cancel(CancelReason::Blur));
                }
                _ => {}
            },
            None => {}
        }
        state.draft = run.draft.clone();
        state.start = run.is_open().then(|| Arc::new(run.start.clone()) as Arc<dyn Any + Send + Sync>);
        state.invalid = run.error.is_some();
        let mut outcome = TextOutcome::new(rect);
        outcome.draft = run.draft.clone();
        outcome.invalid = state.invalid;
        Resolved { events, outcome, submitted, escaped }
    })
}

impl<C: Component> PartsCx<'_, C> {
    /// Registers the component's one text part (`INPUT = Input::Text`).
    pub fn text<K: Codec + Clone + 'static>(&mut self, key: TextKey<K::Value>, rect: Rect, codec: &K, value: &K::Value, options: &TextOptions) {
        debug_assert!(self.text.decl.is_none(), "a component registers one text part");
        let shown = codec.format(value);
        self.text.decl = Some(TextDecl {
            key: key.0,
            rect,
            caret: codec.caret_on_focus(&shown),
            shown,
            options: options.clone(),
            resolve: Box::new(resolver(codec.clone(), options, rect, value.clone())),
            values: ValueOps::of::<K::Value>(),
        });
    }
}

/// Serves the AccessKit requests aimed at the text part: `SetTextSelection` moves the selection and `SetValue`
/// replaces the draft and commits it.
fn serve_access<C: Component>(cx: &mut Cx<'_, C>, state: &mut TextState, widget: egui::Id, shown: &str, read_only: bool) {
    let (ctx, id) = (cx.core.ctx.clone(), cx.core.id);
    while let Some(request) = cx.core.intents.take_access_where(text_access::is_text_request) {
        if let Some((anchor, focus)) = text_access::requested_selection(&ctx, id, &request) {
            let text = state.draft.as_deref().unwrap_or(shown);
            if let Some(rows) = &state.rows {
                adapter::select(&ctx, widget, rows.chars(text, anchor), rows.chars(text, focus));
            }
        } else if let Some(text) = text_access::requested_value(&request)
            && !read_only
        {
            state.draft = Some(text);
            state.dirty = true;
            state.end = Some(End::Blur);
        }
    }
}

/// The text facts AccessKit reads this pass, built only while AccessKit runs.
fn facts<C: Component>(
    cx: &Cx<'_, C>,
    state: &mut TextState,
    widget: egui::Id,
    out: &adapter::AdapterOut,
    rect: Rect,
    masked: bool,
    read_only: bool,
) {
    if !text_access::exposed(&cx.core.ctx, widget) {
        state.rows = None;
        return;
    }
    let rows = adapter::Rows::of(&out.layout, &out.layout.full, rect.min);
    let text = out.layout.full.clone();
    let selection = adapter::selection(&cx.core.ctx, widget).map(|(caret, anchor)| (rows.pos(&text, anchor), rows.pos(&text, caret)));
    let name = cx.core.resolved.label.to_owned();
    *cx.core.text_access.borrow_mut() =
        Some(text_access::TextFacts { widget, name, text, masked, editable: !read_only, lines: rows.lines.clone(), selection });
    state.rows = Some(rows);
}

/// Opens the completion list's popup while the adapter holds candidates and closes it otherwise. A popup the host
/// closed (Escape, a press outside) hides the list until the draft changes.
fn drive_completions<C: Component>(cx: &mut Cx<'_, C>, state: &mut TextState) {
    let list = cx.popup("completions");
    let shown = list.is_open(cx);
    if state.completions.sync(shown) {
        list.open(cx);
    } else {
        list.close(cx);
    }
}

/// Applies the typing run's events to the component's value: a parsed draft reaches the binding in the pass it is
/// typed, and a commit proposes and commits in one pass. Returns whether a commit changed the value, after the
/// parts of this pass were declared.
fn apply_events<C: Component>(cx: &mut Cx<'_, C>, events: Vec<TextEvent<ValueOf<C>>>) -> bool {
    let id = cx.core.id;
    let mut committed = false;
    for event in events {
        let running = cx.core.memory.recall::<Gesture<ValueOf<C>>>(gesture_key(id)).is_running();
        match event {
            TextEvent::Propose(value) => cx.propose(value),
            TextEvent::Commit(value) => {
                committed |= cx.value() != Some(&value);
                if running {
                    cx.propose(value);
                    cx.commit();
                } else if cx.value() != Some(&value) {
                    cx.set(value);
                }
            }
            TextEvent::Cancel(reason) if running => cx.cancel(reason),
            TextEvent::Cancel(_) => {}
        }
    }
    committed
}

/// Step 8b: runs the text adapter at the part's rect, then the typing run over what it recorded, and applies the
/// run's events: a typed value reaches the binding in the pass it is typed.
pub(crate) fn run<C: Component>(
    cx: &mut Cx<'_, C>,
    ui: &mut Ui,
    step: &mut TextStep,
    keys: crate::runtime::focus::KeyUse,
    rtl: bool,
    ink: egui::Color32,
    underline: f32,
) {
    let Some(decl) = step.decl.take() else { return };
    let Ok(resolve) = decl.resolve.downcast::<Resolve<ValueOf<C>>>() else {
        debug_assert!(false, "the text part's codec value is the component's value");
        return;
    };
    let resolve = *resolve;
    let (ctx, id) = (cx.core.ctx.clone(), cx.core.id);
    let state_id = state_id(id);
    let mut state = std::mem::take(cx.core.memory.shown::<TextState>(state_id));
    (decl.values.resync)(&mut state, cx.value().map(|value| value as &dyn Any));
    let widget = adapter::text_id(id, decl.key);
    let focused = crate::runtime::egui::focus::has_focus(&ctx, widget);
    let was_focused = state.focused;
    let now = cx.now();
    let caret_visible = !focused || cx.blink(state.caret_moved);
    let enabled = cx.core.resolved.enabled && decl.options.enabled;
    let read_only = cx.core.resolved.read_only || !enabled;
    let ink = if decl.options.enabled { ink } else { crate::style::rgba_color(crate::runtime::env::Env::of(ui).tokens.color.text.disabled) };
    serve_access(cx, &mut state, widget, &decl.shown, read_only);
    cx.core.surface_slot = Some(ui.painter().add(egui::Shape::Noop));
    let out = adapter::run(
        &mut state,
        adapter::AdapterIn {
            ctx: &ctx,
            ui,
            id,
            key: decl.key,
            rect: decl.rect,
            shown: &decl.shown,
            options: &decl.options,
            keys,
            read_only,
            enabled,
            rtl,
            focused,
            ink,
            intents: &mut cx.core.intents,
            placeholder: decl.options.placeholder.as_ref().map(|label| label.as_str()),
            caret_visible,
            underline,
            now,
        },
    );
    // Focus that moved to another part of the component leaves the text part like a focus loss: the draft commits.
    let left_for_part = out.focused && cx.active_part().is_some();
    if left_for_part {
        state.focused = false;
    }
    let commit_now = step.commit_now || (was_focused && left_for_part);
    if out.focused && !left_for_part && !was_focused {
        let len = decl.shown.chars().count();
        match decl.caret {
            Caret::All => adapter::select(&ctx, widget, 0, len),
            _ => adapter::select(&ctx, widget, len, len),
        }
    }
    if decl.options.complete.is_some() {
        drive_completions(cx, &mut state);
    }
    let resolved = resolve(&mut state, cx.value(), commit_now);
    facts(cx, &mut state, widget, &out, decl.rect, decl.options.mask, read_only);
    cx.core.text_invalid = state.invalid;
    cx.core.escape_to_list = state.completions.open;
    if state.draft.is_some() || (decl.options.clear_on_escape && !out.layout.full.is_empty()) {
        keys::hold_escape(&ctx, id);
    }
    let completions = state.completions.items.clone();
    *cx.core.memory.shown::<TextState>(state_id) = state;
    let mut outcome = resolved.outcome;
    outcome.layout = Some(out.layout);
    outcome.caret_visible = caret_visible;
    outcome.composing = crate::runtime::ime::composing(&ctx, widget);
    outcome.completions = completions;
    cx.core.texts = Some((decl.key, Box::new(outcome)));
    let committed = apply_events(cx, resolved.events);
    (decl.values.note)(cx.core.memory.shown::<TextState>(state_id), cx.shown.as_deref().map(|value| value as &dyn Any));
    // `parts` ran for the value before the commit; the pass runs again so a part the new value adds is never missing.
    if committed {
        crate::runtime::egui::sizing::request_discard(&ctx, "craft_ui: a text commit changed the value");
    }
    cx.core.submitted |= resolved.submitted;
    cx.core.escaped |= resolved.escaped;
}

impl<C: Component> Cx<'_, C> {
    /// Accepts candidate `at` of the text part's completion list in the next pass: a click on a row of the list.
    pub(crate) fn pick_completion(&mut self, at: usize) {
        self.core.memory.shown::<TextState>(state_id(self.core.id)).completions.picked = Some(at);
        crate::runtime::egui::repaint::request(&self.core.ctx);
    }

    /// The result of the text part `key` this pass: the draft, the laid-out text and the caret blink. `None` while
    /// the text service has not produced one (a part first registered this pass), and for a key `parts` never
    /// registered, which is a debug assertion.
    pub fn text<V: 'static>(&self, key: &TextKey<V>) -> Option<&TextOutcome<V>> {
        let outcome =
            self.core.texts.as_ref().filter(|(part, _)| *part == key.key()).and_then(|(_, outcome)| outcome.downcast_ref::<TextOutcome<V>>());
        debug_assert!(outcome.is_some() || self.core.texts.is_none(), "the text part {:?} was never registered by `parts`", key.key());
        outcome
    }
}
