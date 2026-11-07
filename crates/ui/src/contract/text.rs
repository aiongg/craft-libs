//! The text part of the pipeline (18.2): `PartsCx::text` turns what the text adapter recorded last pass into the
//! typing run's events, and step 8b runs the adapter and applies the events through `Cx`.

use super::cx::{gesture_key, Cx, PartsCx};
use super::{Component, Edit as _, ValueOf};
use crate::primitives::text_adapter::{self as adapter, End, TextState};
use crate::primitives::{TextKey, TextOptions, TextOutcome};
use crate::runtime::memory::sub;
use crate::runtime::motion::Animate as _;
use craft_behavior::{CancelReason, Caret, Codec, EditRun, Ended, EscapeRule, Escaped, Gesture, PartKey, Typed, TypingRun};
use egui::{Rect, Ui};
use std::any::Any;
use std::sync::Arc;

/// What `PartsCx::text` leaves for step 8b.
#[derive(Default)]
pub(crate) struct TextStep {
    pub state: TextState,
    pub commit_now: bool,
    pub decl: Option<TextDecl>,
    pub plan: Option<Box<dyn Any>>,
}

pub(crate) struct TextDecl {
    pub key: PartKey,
    pub rect: Rect,
    pub shown: String,
    pub options: TextOptions,
    pub caret: Caret,
}

enum TextEvent<V> {
    Propose(V),
    Commit(V),
    Cancel(CancelReason),
}

struct TextPlan<V> {
    events: Vec<TextEvent<V>>,
    outcome: TextOutcome<V>,
    submitted: bool,
    escaped: bool,
}

impl<C: Component> PartsCx<'_, C> {
    /// Registers the component's one text part (`INPUT = Input::Text`).
    pub fn text<K: Codec>(&mut self, key: TextKey<K::Value>, rect: Rect, codec: &K, value: &K::Value, options: &TextOptions) {
        debug_assert!(self.text.decl.is_none(), "a component registers one text part");
        let step = &mut *self.text;
        let mut state = std::mem::take(&mut step.state);
        let mut typing = TypingRun::new(codec).live(options.live);
        if let Some(limit) = options.limit {
            typing = typing.limit(limit);
        }
        if options.clear_on_escape {
            typing = typing.escape_rule(EscapeRule::ClearThenRestore);
        }
        let started = state.start.as_ref().and_then(|start| start.downcast_ref::<K::Value>()).cloned();
        let mut run = EditRun::new(started.clone().unwrap_or_else(|| value.clone()));
        run.draft = state.draft.clone();
        let mut events = Vec::new();
        let mut shown = None;
        if std::mem::take(&mut state.dirty)
            && let Some(draft) = state.draft.clone()
        {
            let start = run.start.clone();
            if let Typed::Apply(applied) = typing.edit(&mut run, &start, &draft) {
                events.push(TextEvent::Propose(applied));
            }
            state.dirty = false;
        }
        let draft_text = run.draft.clone().unwrap_or_default();
        let (mut submitted, mut escaped) = (false, false);
        let end = match state.end.take() {
            None if step.commit_now && run.is_open() => Some(End::Blur),
            end => end,
        };
        match end {
            Some(End::Escape) => {
                escaped = true;
                match typing.escape(&mut run) {
                    Escaped::Restore(start) => {
                        events.push(TextEvent::Cancel(CancelReason::Escape));
                        shown = Some(codec.format(&start));
                    }
                    Escaped::Cleared => state.dirty = true,
                    _ => {}
                }
            }
            Some(End::Submit) => match typing.commit(&mut run) {
                Ended::Commit(committed) => {
                    shown = Some(codec.normalise(&draft_text, &committed));
                    events.push(TextEvent::Commit(committed));
                    submitted = true;
                }
                Ended::Rejected => {}
                Ended::Idle => submitted = true,
                _ => {}
            },
            Some(End::Blur) => match typing.commit(&mut run) {
                Ended::Commit(committed) => {
                    shown = Some(codec.normalise(&draft_text, &committed));
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
        let shown = shown.unwrap_or_else(|| typing.shown(&run, value));
        let mut outcome = TextOutcome::new(rect);
        outcome.draft = run.draft.clone();
        outcome.invalid = state.invalid;
        step.decl = Some(TextDecl { key: key.0, rect, caret: codec.caret_on_focus(&shown), shown, options: options.clone() });
        step.plan = Some(Box::new(TextPlan { events, outcome, submitted, escaped }));
        step.state = state;
    }
}

/// Step 8b: runs the text adapter at the part's rect and applies the typing run's events.
pub(crate) fn run<C: Component>(cx: &mut Cx<'_, C>, ui: &mut Ui, step: &mut TextStep, keys: crate::runtime::focus::KeyUse, rtl: bool, ink: egui::Color32) {
    let (Some(decl), Some(plan)) = (step.decl.take(), step.plan.take()) else { return };
    let Ok(plan) = plan.downcast::<TextPlan<ValueOf<C>>>() else {
        debug_assert!(false, "the text part's codec value is the component's value");
        return;
    };
    let plan = *plan;
    let (ctx, id) = (cx.core.ctx.clone(), cx.core.id);
    let state_id = sub(id, "text_state");
    let mut state = std::mem::take(cx.core.memory.shown::<TextState>(state_id));
    let widget = adapter::text_id(id, decl.key);
    let focused = ctx.memory(|memory| memory.has_focus(widget));
    let was_focused = state.focused;
    let now = cx.now();
    let caret_visible = !focused || cx.blink(state.caret_moved);
    let enabled = cx.core.resolved.enabled;
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
            read_only: cx.core.resolved.read_only || !enabled,
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
            Caret::All => crate::runtime::egui::text_edit::select(&ctx, widget, 0, len),
            _ => crate::runtime::egui::text_edit::select(&ctx, widget, len, len),
        }
    }
    *cx.core.memory.shown::<TextState>(state_id) = state;
    let mut outcome = plan.outcome;
    outcome.layout = Some(out.layout);
    outcome.caret_visible = caret_visible;
    outcome.composing = crate::runtime::ime::composing(&ctx, widget);
    cx.core.texts = Some((decl.key, Box::new(outcome)));
    let running = cx.core.memory.recall::<Gesture<ValueOf<C>>>(gesture_key(id)).is_running();
    for event in plan.events {
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
    cx.core.submitted |= plan.submitted;
    cx.core.escaped |= plan.escaped;
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
