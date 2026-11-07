//! The text part of the pipeline (18.2): `PartsCx::text` registers the part with its typing run, and step 8b runs the
//! text adapter, then the typing run over what the adapter recorded, and applies the run's events through `Cx`.

use super::cx::{Cx, PartsCx, gesture_key};
use super::{Component, Edit as _, ValueOf};
use crate::primitives::text_adapter::{self as adapter, End, TextState};
use crate::primitives::{TextKey, TextOptions, TextOutcome};
use crate::runtime::access::text as text_access;
use crate::runtime::egui::text_edit;
use crate::runtime::memory::sub;
use crate::runtime::motion::Animate as _;
use craft_behavior::{CancelReason, Caret, Codec, EditRun, Ended, EscapeRule, Escaped, Gesture, NotAValue, PartKey, Typed, TypingRun};
use egui::{Rect, Ui};
use std::any::Any;
use std::sync::Arc;

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

/// The typing run over the state the adapter recorded this pass, the value shown and whether a commit is due.
type Resolve<V> = Box<dyn FnOnce(&mut TextState, &V, bool) -> Resolved<V>>;

/// Turns what the adapter recorded this pass into the typing run's events: a parsed draft applies in the pass it
/// is typed, and a commit proposes and commits in one pass.
fn resolver<K: Codec + Clone + 'static>(codec: K, options: &TextOptions, rect: Rect) -> Resolve<K::Value> {
    let (live, limit, clear_on_escape) = (options.live, options.limit, options.clear_on_escape);
    Box::new(move |state, value, commit_now| {
        let mut typing = TypingRun::new(codec).live(live);
        if let Some(limit) = limit {
            typing = typing.limit(limit);
        }
        if clear_on_escape {
            typing = typing.escape_rule(EscapeRule::ClearThenRestore);
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
                match typing.escape(&mut run) {
                    Escaped::Restore(_) => events.push(TextEvent::Cancel(CancelReason::Escape)),
                    Escaped::Cleared => state.dirty = true,
                    _ => {}
                }
            }
            Some(End::Submit) => match typing.commit(&mut run) {
                Ended::Commit(committed) => {
                    state.normalised = Some(typing.codec().normalise(&draft_text, &committed));
                    events.push(TextEvent::Commit(committed));
                    submitted = true;
                }
                Ended::Rejected => {}
                Ended::Idle => submitted = true,
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
            resolve: Box::new(resolver(codec.clone(), options, rect)),
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
                text_edit::select(&ctx, widget, rows.chars(text, anchor), rows.chars(text, focus));
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
    if !crate::runtime::egui::access::enabled(&cx.core.ctx) {
        state.rows = None;
        return;
    }
    let rows = adapter::Rows::of(&out.layout, &out.layout.full, rect.min);
    let text = out.layout.full.clone();
    let selection = text_edit::selection(&cx.core.ctx, widget).map(|(caret, anchor)| (rows.pos(&text, anchor), rows.pos(&text, caret)));
    *cx.core.text_access.borrow_mut() = Some(text_access::TextFacts { text, masked, editable: !read_only, lines: rows.lines.clone(), selection });
    state.rows = Some(rows);
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
) {
    let Some(decl) = step.decl.take() else { return };
    let Ok(resolve) = decl.resolve.downcast::<Resolve<ValueOf<C>>>() else {
        debug_assert!(false, "the text part's codec value is the component's value");
        return;
    };
    let resolve = *resolve;
    let (ctx, id) = (cx.core.ctx.clone(), cx.core.id);
    let state_id = sub(id, "text_state");
    let mut state = std::mem::take(cx.core.memory.shown::<TextState>(state_id));
    let widget = adapter::text_id(id, decl.key);
    let focused = ctx.memory(|memory| memory.has_focus(widget));
    let was_focused = state.focused;
    let now = cx.now();
    let caret_visible = !focused || cx.blink(state.caret_moved);
    let enabled = cx.core.resolved.enabled;
    let read_only = cx.core.resolved.read_only || !enabled;
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
            rtl,
            focused,
            ink,
            intents: &mut cx.core.intents,
            placeholder: decl.options.placeholder.as_ref().map(|label| label.as_str()),
            caret_visible,
            now,
        },
    );
    if out.focused && !was_focused {
        let len = decl.shown.chars().count();
        match decl.caret {
            Caret::All => text_edit::select(&ctx, widget, 0, len),
            _ => text_edit::select(&ctx, widget, len, len),
        }
    }
    let Some(resolved) = cx.value().map(|value| resolve(&mut state, value, step.commit_now)) else { return };
    facts(cx, &mut state, widget, &out, decl.rect, decl.options.mask, read_only);
    cx.core.text_invalid = state.invalid;
    *cx.core.memory.shown::<TextState>(state_id) = state;
    let mut outcome = resolved.outcome;
    outcome.layout = Some(out.layout);
    outcome.caret_visible = caret_visible;
    outcome.composing = crate::runtime::ime::composing(&ctx, widget);
    cx.core.texts = Some((decl.key, Box::new(outcome)));
    for event in resolved.events {
        let running = cx.core.memory.recall::<Gesture<ValueOf<C>>>(gesture_key(id)).is_running();
        match event {
            TextEvent::Propose(value) => cx.propose(value),
            TextEvent::Commit(value) if running => {
                cx.propose(value);
                cx.commit();
            }
            TextEvent::Commit(value) => {
                if cx.value() != Some(&value) {
                    cx.set(value);
                }
            }
            TextEvent::Cancel(reason) if running => cx.cancel(reason),
            TextEvent::Cancel(_) => {}
        }
    }
    cx.core.submitted |= resolved.submitted;
    cx.core.escaped |= resolved.escaped;
}

impl<C: Component> Cx<'_, C> {
    /// The result of the text part `key` this pass: the draft, the laid-out text and the caret blink. A key
    /// `parts` never registered panics.
    pub fn text<V: 'static>(&self, key: &TextKey<V>) -> &TextOutcome<V> {
        self.core
            .texts
            .as_ref()
            .filter(|(part, _)| *part == key.key())
            .and_then(|(_, outcome)| outcome.downcast_ref::<TextOutcome<V>>())
            .unwrap_or_else(|| panic!("the text part {:?} was never registered by `parts`", key.key()))
    }
}
