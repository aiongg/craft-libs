//! The text part of the pipeline (18.2): `PartsCx::text` registers the part with its typing run, and step 8b runs the
//! text adapter, then the typing run over what the adapter recorded, and applies the run's events through `Cx`.
//!
//! Selection parts are read-only text with no typing run, any number per component: `PartsCx::selectable_text`
//! declares one and its press-only stop, step 8b gives the focused one Select All and Copy, and
//! `PaintUi::selectable_text` shows it, so it paints in the state's colours.

use super::cx::{Cx, CxCore, PartsCx, RecordedText, gesture_key};
use super::{Component, Edit as _, Focus as _, Overlays as _, ValueOf};
use crate::primitives::Id;
use crate::primitives::text_adapter::{self as adapter, End, TextState};
use crate::primitives::{TextKey, TextOptions, TextOutcome, TextRun, Wrap};
use crate::runtime::access::text as text_access;
use crate::runtime::focus::{EditSet, KeepKeys, KeyUse};
use crate::runtime::memory::{MemoryStage, sub};
use craft_behavior::Sense;
use craft_behavior::{
    CancelReason, Caret, Codec, EditCommand, EditRun, Ended, EscapeRule, Escaped, Gesture, NotAValue, PartKey, PartRef, Typed, TypingRun,
};
use craft_keymap::Mods;
use craft_ui_rt::Rect;
use craft_ui_rt::text_edit::TextEdit;
use std::any::Any;
use std::sync::Arc;

/// The popup name of a text part's edit menu: step 8b registers it for every text part, and the text control opens
/// and shows it from its `overlay` hook.
pub const EDIT_MENU: &str = "edit";

/// The id of the text adapter's state in the component's memory.
fn state_id(id: Id) -> Id {
    sub(id, "text_state")
}

/// The selection the text part of the component `id` held when its edit menu took the focus, while the menu is open.
pub(crate) fn held_selection(memory: &mut MemoryStage, id: Id) -> Option<(usize, usize)> {
    memory.shown::<TextState>(state_id(id)).held_selection
}

/// Whether the text part of the component `id` has a typing run open, as its last pass left it.
pub(crate) fn run_open(memory: &MemoryStage, id: Id) -> bool {
    memory.recall::<TextState>(state_id(id)).start.is_some()
}

/// The draft the text part of the component `id` holds while a typing run is open.
pub(crate) fn draft(memory: &mut MemoryStage, id: Id) -> Option<String> {
    memory.shown::<TextState>(state_id(id)).draft.clone()
}

/// What `PartsCx::text` leaves for step 8b.
#[derive(Default)]
pub(crate) struct TextStep {
    pub decl: Option<TextDecl>,
    pub editor: Option<super::editor::EditorDecl>,
    pub selections: Vec<SelectDecl>,
}

/// A selection part as `PartsCx::selectable_text` declared it.
pub(crate) struct SelectDecl {
    pub part: PartRef,
    pub text: String,
    /// The text wraps: the arrows move the caret between lines.
    pub multiline: bool,
}

impl SelectDecl {
    /// The keys a focused selection part keeps: a read-only text's, with Select All, Copy and Cut its only edit
    /// commands. Cut copies, as in a label's selection: the text removes nothing, and the app's Cut never runs on what
    /// lies behind it.
    pub(crate) fn keys(&self) -> KeyUse {
        let edits = EditSet::NONE.with(EditCommand::SelectAll).with(EditCommand::Copy).with(EditCommand::Cut);
        let keys = KeyUse { edits, ..KeyUse::of(crate::contract::Input::Text) };
        if self.multiline { keys.with_keep(KeepKeys::BlockArrows) } else { keys }
    }
}

/// The engine state of the selection part `part` of component `id`, in its node memory.
fn selection_id(id: Id, part: PartRef) -> Id {
    sub(id, ("selection", part.key, part.index))
}

/// Step 8b for selection parts: the focused one takes this run's Select All, Copy and Cut on its engine state.
pub(crate) fn run_selections(core: &mut super::cx::CxCore<'_>, selections: &[SelectDecl]) {
    let Some(active) = core.active_part() else { return };
    let Some(focused) = selections.iter().find(|decl| decl.part == active) else { return };
    let key = selection_id(core.id, focused.part);
    let mut edit = std::mem::take(core.memory.shown::<TextEdit>(key));
    while let Some(command) = core.intents.take_edit() {
        adapter::select_or_copy(core, &mut edit, &focused.text, command);
    }
    *core.memory.shown::<TextEdit>(key) = edit;
}

pub(crate) struct TextDecl {
    pub part: TextPart,
    /// A `Resolve<V>` for the component's value type, which the text step takes and invokes after the adapter records
    /// the pass; the part stays declared, so the stops read its keys as the pass's edits left them.
    resolve: Option<Box<dyn Any>>,
    values: ValueOps,
}

pub(crate) struct TextPart {
    pub key: PartKey,
    rect: Rect,
    pub shown: String,
    pub options: TextOptions,
    caret: Caret,
}

/// The shown copy of a component's value, type-erased.
pub(crate) type Shown = Arc<dyn Any + Send + Sync>;

/// What the typing run does with the component's value, written once for its type: the run keeps the value it last
/// showed, and a different one at the next pass was set outside the run.
#[derive(Clone, Copy)]
pub(crate) struct ValueOps {
    /// Ends the run when the value is not the one the run last showed.
    resync: fn(&mut TextState, Option<&Shown>),
}

impl ValueOps {
    fn of<V: Clone + PartialEq + Send + Sync + 'static>() -> ValueOps {
        ValueOps {
            resync: |state, shown| {
                let (Some(seen), Some(shown)) = (state.seen.as_ref(), shown) else { return };
                if Arc::ptr_eq(seen, shown) {
                    return;
                }
                if let (Some(seen), Some(shown)) = (seen.downcast_ref::<V>(), shown.downcast_ref::<V>())
                    && seen != shown
                {
                    state.draft = None;
                    state.start = None;
                    state.normalised = None;
                    state.dirty = false;
                    state.invalid = false;
                }
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
    /// The modifiers of the submitting Enter; none unless `submitted`.
    submit_mods: Mods,
    escaped: bool,
    /// The draft became the codec's normal form of its value: `run` selects the whole text.
    rewritten: bool,
    /// The pass refused input (a character the codec's filter dropped, a rewriting Tab): `run` calls
    /// `frame::refuse`, since the resolver has no context.
    refused: bool,
}

/// The typing run over the state the adapter recorded this pass, the component's value (`None` while mixed) and
/// whether a commit is due.
type Resolve<V> = Box<dyn FnOnce(&mut TextState, Option<&V>, bool, bool) -> Resolved<V>>;

/// Turns what the adapter recorded this pass into the typing run's events: a parsed draft applies in the pass it
/// is typed, and a commit proposes and commits in one pass. `fallback` is the value the part shows, the run's start
/// while the component's value is mixed. The closure's last argument says whether the part keeps its text on Escape
/// this pass: the option alone is not enough, since a part that went read-only or disabled mid-run takes Escape as a
/// plain field does.
fn resolver<K: Codec + Clone + 'static>(codec: K, options: &TextOptions, rect: Rect, fallback: K::Value) -> Resolve<K::Value> {
    let (live, limit, clear_on_escape) = (options.live, options.limit, options.clear_on_escape);
    Box::new(move |state, value, commit_now, keep_on_escape| {
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
        let (mut rewritten, mut refused) = (false, false);
        if std::mem::take(&mut state.dirty)
            && let Some(draft) = state.draft.clone()
        {
            let start = run.start.clone();
            let (typed, dropped) = typing.edit(&mut run, &start, &draft);
            refused |= dropped;
            if let Typed::Apply(applied) = typed {
                events.push(TextEvent::Propose(applied));
            }
        }
        let draft_text = run.draft.clone().unwrap_or_default();
        let (mut submitted, mut escaped) = (false, false);
        let mut committed_note = None;
        let submit_mods = std::mem::take(&mut state.submit_mods);
        let end = match state.end.take() {
            None if commit_now && run.is_open() => Some(End::Blur),
            end => end,
        };
        match end {
            Some(End::Revert) => {
                if let Escaped::Restore(_) = typing.escape(&mut run) {
                    events.push(TextEvent::Cancel(CancelReason::Escape));
                }
            }
            Some(End::Escape) if keep_on_escape => {
                // The text stays: an open run commits once, neither submitted nor cancelled.
                escaped = true;
                if let (Ended::Commit(committed), note) = typing.commit_tagged(&mut run) {
                    committed_note = Some(note);
                    state.normalised = Some(typing.codec().normalise(&draft_text, &committed));
                    events.push(TextEvent::Commit(committed));
                }
            }
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
            Some(End::Rewrite { refuses }) => {
                rewritten = typing.rewrite(&mut run);
                refused |= refuses;
            }
            Some(end @ (End::Submit | End::Separator)) => match typing.commit_tagged(&mut run) {
                (Ended::Commit(committed), note) => {
                    committed_note = Some(note);
                    state.normalised = Some(typing.codec().normalise(&draft_text, &committed));
                    events.push(TextEvent::Commit(committed));
                    submitted = end == End::Submit;
                }
                (Ended::Rejected, _) => {}
                (Ended::Idle, _) => submitted = end == End::Submit,
                _ => {}
            },
            Some(End::Blur) => match typing.commit_tagged(&mut run) {
                (Ended::Commit(committed), note) => {
                    committed_note = Some(note);
                    state.normalised = Some(typing.codec().normalise(&draft_text, &committed));
                    events.push(TextEvent::Commit(committed));
                }
                (Ended::Rejected, _) => {
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
        outcome.committed = committed_note;
        Resolved { events, outcome, submitted, submit_mods: if submitted { submit_mods } else { Mods::NONE }, escaped, rewritten, refused }
    })
}

impl<C: Component> PartsCx<'_, C> {
    /// Declares `run` as a selection part: read-only text that a press focuses, drag and double click select and Copy
    /// copies, and that Tab and F6 pass over. `paint` shows it with `PaintUi::selectable_text` and the same run in
    /// the state's colours. Its text is the component's own: it has no node of its own, so the component's node
    /// names it. An elided run cannot be one, since the selection counts the characters shown.
    pub fn selectable_text(&mut self, part: impl Into<PartRef>, run: &TextRun<'_>) {
        debug_assert!(!(run.wrap.cuts() || matches!(run.wrap, Wrap::Clamp(_))), "a selection part shows its whole text");
        let part = part.into();
        debug_assert!(self.text.selections.iter().all(|decl| decl.part != part), "selection part {part:?} declared twice");
        self.text.selections.push(SelectDecl { part, text: run.full_text(), multiline: run.wrap == Wrap::Wrap });
    }

    /// Registers the component's one text part (`INPUT = Input::Text`).
    pub fn text<K: Codec + Clone + 'static>(&mut self, key: TextKey<K::Value>, rect: Rect, codec: &K, value: &K::Value, options: &TextOptions) {
        debug_assert!(self.text.decl.is_none(), "a component registers one text part");
        let shown = if options.mixed { String::new() } else { codec.format(value) };
        let reads = codec.clone();
        let declared = TextOptions { whole_paste: Some(Arc::new(move |pasted: &str| reads.whole_paste(pasted))), ..options.clone() };
        let resolve = Box::new(resolver(codec.clone(), &declared, rect, value.clone()));
        // The text takes the pointer on its rect: a press places the caret (so the press marks the node), a drag
        // selects, and the pointer shows the text cursor. Several lines scroll by the wheel.
        let wheel = if options.multiline() { Sense::WHEEL_Y } else { Sense::empty() };
        self.part(key.0, rect)
            .sense(Sense::HOVER | Sense::CLICK | Sense::DRAG | Sense::SETS_ON_PRESS | wheel)
            .cursor(craft_behavior::CursorIcon::Text);
        self.text.decl = Some(TextDecl {
            part: TextPart {
                key: key.0,
                rect,
                caret: if options.select_on_focus { Caret::All } else { codec.caret_on_focus(&shown) },
                shown,
                options: declared,
            },
            resolve: Some(resolve),
            values: ValueOps::of::<K::Value>(),
        });
    }
}

/// Serves the AccessKit requests aimed at the text part: `SetValue` replaces the draft and commits it. A
/// `SetTextSelection` reaches the engine as an editor input in queue order.
fn serve_access(core: &mut CxCore<'_>, state: &mut TextState, read_only: bool) {
    while let Some(request) = core.intents.take_access_where(text_access::is_text_request) {
        if let Some(text) = text_access::requested_value(&request)
            && !read_only
        {
            state.draft = Some(text);
            state.dirty = true;
            state.end = Some(End::Blur);
        }
    }
}

/// What a field says about itself to AccessKit besides its text.
struct FieldFacts {
    masked: bool,
    read_only: bool,
    enabled: bool,
}

/// The text facts the access writer reads this run, built only while the access tree is on. A masked field reports
/// its shown dots, never its text.
fn facts(core: &CxCore<'_>, state: &mut TextState, out: &adapter::AdapterOut, field: FieldFacts, access: bool) {
    let FieldFacts { masked, read_only, enabled } = field;
    if !access {
        state.rows = None;
        return;
    }
    let origin = out.paint.origin;
    let rows = adapter::Rows::of(&out.layout, out.layout.full(), origin);
    let text = out.layout.full().to_owned();
    let selection = (!masked).then(|| (rows.pos(state.edit.anchor()), rows.pos(state.edit.caret())));
    *core.text_access.borrow_mut() = Some(text_access::TextFacts {
        widget: core.id,
        text,
        masked,
        editable: !read_only,
        disabled: !enabled,
        lines: rows.lines.clone(),
        selection,
        unit: None,
        value_override: None,
    });
    state.rows = Some(rows);
}

/// The component's shown copy of its value, shared.
fn shown_copy<C: Component>(cx: &Cx<'_, C>) -> Option<Shown> {
    cx.shown.clone().map(|shown| shown as Shown)
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

/// Applies the typing run's events to the component's value: a parsed draft reaches the binding in the run it is
/// typed, and a commit proposes and commits in one run. Returns whether a commit changed the value.
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

/// Whether a scrubbing field was in edit before the press that is now down: a press that becomes a drag scrubs and
/// leaves edit as it found it, and only a click without travel enters it.
#[derive(Clone, Default)]
struct InEditBeforePress(bool);

/// What the driver hands the text step besides the component's run.
pub(crate) struct TextIn<'a> {
    pub services: &'a dyn crate::primitives::Services,
    pub keys: KeyUse,
    pub rtl: bool,
    /// The component senses a drag on its root: it scrubs its value by it.
    pub scrubs: bool,
    /// The enclosing focus scope's Escape rules (`adapter::ScopeEscape`, provided by a dialog or palette).
    pub scope: adapter::ScopeEscape,
    /// The window's access tree is on: the run reports its text facts.
    pub access: bool,
    /// The focus is on the window's menu bar, or in what it opened, and F10 took it from this node.
    pub bar_returns_here: bool,
}

struct PreparedText {
    out: adapter::AdapterOut,
    read_only: bool,
    list_open: bool,
    commit_now: bool,
    was_focused: bool,
    /// The window lost focus in this run while the text part had it.
    window_blur: bool,
    /// The part's edit menu is open and holds its editing focus.
    menu_open: bool,
    /// Focus moved from the text part to another part of its component in this run.
    left_for_part: bool,
}

/// Runs the text adapter and adjusts focus before the component's typed resolver handles its value.
fn prepare(core: &mut CxCore<'_>, decl: &TextPart, state: &mut TextState, text_in: &TextIn<'_>, commit_now: bool) -> PreparedText {
    let id = core.id;
    let node_focused = core.focus.focused;
    let mut focused = node_focused;
    let was_focused = state.focused;
    let enabled = core.resolved.enabled && decl.options.enabled;
    // A component that senses a drag on its root scrubs its value by it: the field takes the pointer only once it is
    // in edit, a press that begins in edit stays the text's (a drag selects), and a click that enters it selects as
    // focus does (`decl.caret`).
    let mut in_edit_before = false;
    if text_in.scrubs {
        let before = core.memory.shown::<InEditBeforePress>(sub(id, "in_edit_before_press")).0;
        if core.press.phase.is_drag() {
            // The root press focused the field; a press that travelled is a scrub, so the field gives the focus back.
            if !before {
                core.surrender_focus();
                focused = false;
            }
        } else if !core.press.phase.button_down() {
            core.memory.shown::<InEditBeforePress>(sub(id, "in_edit_before_press")).0 = focused;
        }
        in_edit_before = core.memory.shown::<InEditBeforePress>(sub(id, "in_edit_before_press")).0;
    }
    let pointer = !text_in.scrubs || in_edit_before || (focused && !core.press.phase.is_drag());
    core.text_takes_pointer = pointer;
    let read_only = core.resolved.read_only || !enabled;
    let env = core.env.to_env();
    let ink = match decl.options.ink {
        _ if !enabled => crate::style::rgba_color(env.tokens.color.text.disabled),
        Some(own) => crate::style::ink_color(&env.tokens, own),
        None => crate::style::rgba_color(env.tokens.color.text.normal),
    };
    let held = core.popup(EDIT_MENU).is_open_in(core);
    let current = decl.options.resting.as_deref().filter(|_| !focused && !held && state.draft.is_none()).unwrap_or(&decl.shown);
    // The bound caret is compared with the engine's before anything else this run moves it: last run reported the
    // engine's caret, so only an app write differs here. A composition keeps its caret.
    let caret_placed =
        state.edit.preedit().is_none() && adapter::place_caret(state, &state.draft.clone().unwrap_or_else(|| decl.shown.clone()), decl.options.caret);
    serve_access(core, state, read_only);
    let select_all = (std::mem::take(&mut state.select_all) || decl.options.select_all) && state.edit.preedit().is_none();
    if select_all {
        let text = state.draft.clone().unwrap_or_else(|| current.to_owned());
        state.edit.select_all(&text);
    }
    let menu_held = state.held_selection.is_some();
    // A menu bar the keyboard goes back to the part from has the focus: its menus' Cut and Copy act on the selection
    // the part keeps for them. Only a part that had the focus last run, or keeps a selection already, is gone back to.
    let bar_held = text_in.bar_returns_here && (was_focused || menu_held);
    // Focus that arrived since the last run places the caret before this run's input is applied, so the first text
    // typed replaces a selection. Focus back from the edit menu is no arrival.
    let arrived = focused && !was_focused && !held && !menu_held && core.active_part().is_none();
    // Focus that arrives with a press on the part came with the pointer.
    let by_pointer = core.press.phase.button_down();
    if arrived {
        arrive(core, decl, state, by_pointer, select_all);
    }
    let out = adapter::run(
        core,
        state,
        adapter::AdapterIn {
            services: text_in.services,
            key: decl.key,
            part_id: crate::contract::rt::node::part_id(PartRef::from(decl.key)),
            rect: decl.rect,
            shown: &decl.shown,
            options: &decl.options,
            keys: text_in.keys,
            read_only,
            enabled,
            pointer,
            rtl: text_in.rtl,
            focused: focused && core.active_part().is_none(),
            held,
            keeps_selection: held || bar_held,
            ink,
            list_open: core.popups.any_open(),
            placeholder: decl.options.placeholder.as_ref().map(|label| label.as_str()),
            underline: env.tokens.stroke.thin,
            caret_placed,
            scope: text_in.scope,
        },
    );
    // Focus that moved to another part of the component leaves the text part like a focus loss: the draft commits.
    let left_for_part = focused && core.active_part().is_some();
    if left_for_part {
        state.focused = false;
    }
    // A window deactivation is no edit's end: the draft stays for the next Enter or a press elsewhere.
    let window_blur = was_focused && !core.context().window_focused();
    let commit_now = (commit_now || (was_focused && left_for_part)) && !window_blur;
    // A caller can ask for focus after showing the field: the selection is asked for again next run, so focus gain
    // cannot replace it with the default caret.
    if decl.options.select_all && select_all && !out.focused {
        state.select_all = true;
    }
    if state.select_on_click {
        if out.clicked && out.focused {
            state.edit.select_all(&decl.shown);
            state.select_on_click = false;
        } else if !out.focused || !core.press.phase.button_down() {
            // A drag, or a press that ended elsewhere: the pointer's caret or range stays.
            state.select_on_click = false;
        }
    }
    let list_open = core.popups.any_open();
    PreparedText { out, read_only, list_open, commit_now, was_focused, window_blur, menu_open: held, left_for_part }
}

/// Places the caret as focus arrives on the text part: where the codec puts it, or over the whole text for
/// `select_on_focus`, or when a select was asked for (`select_all`). `select_on_focus` is for focus by key or request:
/// focus a pointer press brings (`by_pointer`) keeps the caret or range the pointer makes, and a plain click selects
/// the whole text once it is released (`TextState::select_on_click`).
fn arrive(core: &CxCore<'_>, decl: &TextPart, state: &mut TextState, by_pointer: bool, select_all: bool) {
    // A dialog that opens on this field selects its whole text whatever the codec's caret, so typing replaces it
    // (photocraft#15540).
    let opened = !decl.options.keep_caret_on_open && core.focus.arrived_origin == Some(crate::runtime::focus::FocusOrigin::Open);
    if by_pointer && decl.options.select_on_focus && !opened && !select_all {
        state.select_on_click = true;
        return;
    }
    let text = state.draft.clone().unwrap_or_else(|| decl.shown.clone());
    match if opened || select_all { Caret::All } else { decl.caret } {
        Caret::All => state.edit.select_all(&text),
        // A bound caret is where the app put it: focus arriving keeps it.
        _ if decl.options.caret.is_some() => {}
        _ => state.edit.select(&text, text.len(), text.len()),
    }
}

/// Step 8b: runs the text adapter at the part's rect, then the typing run over what it recorded, and applies the
/// run's events: a typed value reaches the binding in the run it is typed. Returns the keys the part's stop registers.
pub(crate) fn run<C: Component>(cx: &mut Cx<'_, C>, step: &mut TextStep, text_in: TextIn<'_>) -> Option<KeyUse> {
    let decl = step.decl.as_mut()?;
    let resolve = decl.resolve.take()?;
    let Ok(resolve) = resolve.downcast::<Resolve<ValueOf<C>>>() else {
        debug_assert!(false, "the text part's codec value is the component's value");
        return None;
    };
    let resolve = *resolve;
    let part = &decl.part;
    let id = cx.core.id;
    let state_id = state_id(id);
    let mut state = std::mem::take(cx.core.memory.shown::<TextState>(state_id));
    // A part whose node is new (not shown last frame) lost its focus, and its typing run with it: the part starts
    // afresh from its value.
    if crate::runtime::memory::Remember::appeared(cx) {
        state = TextState::default();
    }
    (decl.values.resync)(&mut state, shown_copy(cx).as_ref());
    let keys = text_in.keys;
    let held = cx.core.input().iter().any(|input| matches!(input, crate::contract::rt::node::NodeInput::HeldPress));
    let text = prepare(cx.core, part, &mut state, &text_in, press_left(cx.core.input()));
    let PreparedText { out, read_only, list_open, commit_now, was_focused, window_blur, menu_open, left_for_part } = text;
    for command in out.unhandled.iter().copied().filter_map(crate::runtime::commands::EditCommands::of) {
        use crate::runtime::commands::Commands as _;
        cx.fire(command, crate::runtime::commands::FireSource::Key);
    }
    if part.options.complete.is_some() {
        drive_completions(cx, &mut state);
    }
    if window_blur && state.end == Some(End::Blur) {
        state.end = None;
    }
    if part.options.holds_drafts && held && state.end.is_none() {
        // A press on a part declared `holds_drafts`, or in a popup above the field, kept the focus: the draft shows as
        // its value would.
        state.end = Some(End::Rewrite { refuses: false });
    }
    let keeps_escape = part.options.keep_on_escape && !part.options.clear_on_escape && !read_only && cx.core.resolved.enabled && part.options.enabled;
    let resolved = resolve(&mut state, cx.value(), commit_now, keeps_escape);
    if resolved.escaped && keeps_escape && !state.invalid {
        // The same Escape goes on to the enclosing scope in this run, which still finds this focused stop; the field
        // lets focus go once the frame's builds are done, unless a scope the Escape closed returned it first.
        cx.core.intents.pass_on(craft_behavior::Intent::Cancel);
        cx.core.surrender_focus();
    }
    if resolved.refused {
        cx.core.context().refuse();
    }
    if resolved.rewritten {
        let draft = state.draft.clone().unwrap_or_default();
        state.edit.select_all(&draft);
        cx.core.run.wake.set(true);
    }
    if let Some(baseline) = out.baseline {
        cx.core.paint.borrow_mut().baseline.get_or_insert(baseline);
    }
    let enabled = cx.core.resolved.enabled && part.options.enabled;
    facts(cx.core, &mut state, &out, FieldFacts { masked: part.options.mask, read_only, enabled }, text_in.access);
    if let Some(caret) = out.caret {
        cx.core.ime_cursor(caret);
    }
    cx.core.text_invalid = state.invalid;
    cx.core.text_unparsed = part.options.unparsed.clone().filter(|_| state.invalid);
    cx.core.escape_to_list = state.completions.open || list_open;
    if state.draft.is_some() || (part.options.clear_on_escape && !out.layout.full().is_empty()) || (keeps_escape && out.focused) {
        cx.core.hold_escape();
    }
    let completions = state.completions.items.clone();
    // The caret after every move of this run, which a bound caret is written back from.
    let caret = Some(state.edit.caret());
    let composing = state.edit.preedit().is_some();
    cx.core.edit_paint = Some(out.paint.clone());
    // The keys the part's stop registers, as this run's edits left them.
    let stop_keys = adapter::with_idle(&state, adapter::keys_of(keys, &part.options, state.draft.is_some()));
    *cx.core.memory.shown::<TextState>(state_id) = state;
    let mut outcome = resolved.outcome;
    outcome.layout = Some(out.layout);
    // The caret shows while the part holds focus; its blink is the node's paint.
    outcome.caret_visible = out.focused;
    outcome.composing = composing;
    outcome.completions = completions;
    outcome.clicked = out.clicked;
    // A Tab off the part reports its direction; any other loss (a press, a request, the node going) reports none.
    let left_by_tab = if was_focused && !out.focused { tabbed_off(cx.core.input()) } else { None };
    // The edit menu holds the part's editing focus while it is open: key focus that is with the menu is not lost
    // until the menu is gone and focus is not back.
    let text_lost_focus = was_focused && !menu_open && (!out.focused || left_for_part);
    cx.core.texts =
        Some((part.key, Box::new(RecordedText { outcome, submit_mods: resolved.submit_mods, left_by_tab, tab: out.tab, caret, text_lost_focus })));
    apply_events(cx, resolved.events);
    cx.core.memory.shown::<TextState>(state_id).seen = shown_copy(cx);
    cx.core.submitted |= resolved.submitted;
    // A read-only multi-line part makes no line break, so its Enter goes on to the default button.
    if keys.submits && !(read_only && part.options.multiline()) {
        cx.core.intents.take(craft_behavior::Intent::Submit);
    }
    cx.core.escaped |= resolved.escaped;
    Some(stop_keys)
}

/// The direction of the Tab that moved focus off the node this run, from the last focus loss its queue holds.
/// A pointer press elsewhere took the focus in this run: the draft commits now, before the page that press switches can
/// remove the field.
pub(crate) fn press_left(input: &[crate::contract::rt::node::NodeInput]) -> bool {
    input.iter().any(|item| matches!(item, crate::contract::rt::node::NodeInput::Focus { gained: false, cause: craft_ui_rt::FocusCause::Pointer }))
}

fn tabbed_off(input: &[crate::contract::rt::node::NodeInput]) -> Option<bool> {
    use crate::contract::rt::node::NodeInput;
    input.iter().rev().find_map(|item| match item {
        NodeInput::Focus { gained: false, cause } => Some(match cause {
            craft_ui_rt::FocusCause::Tab { forward } => Some(*forward),
            _ => None,
        }),
        _ => None,
    })?
}

impl CxCore<'_> {
    /// Gives up key focus once the frame's builds are done, if the node still holds it then: a scrubbing field after
    /// a scrub, a field whose Escape leaves it or hands it on. The driver asks ui-rt after the run.
    pub(crate) fn surrender_focus(&self) {
        self.run.surrender.set(true);
    }
}

impl<C: Component> Cx<'_, C> {
    /// Changes the text part's draft in the next run: `Some(text)` puts `text` in as a change of the typing run (one
    /// that begins the run when none is under way), the caret at its end; `None` ends the run with the text from
    /// before it, without the run's Escape reaching the host. A combo box's keys in its open list put the active
    /// option's text in, and its list's Escape puts the typed text back.
    pub fn set_draft(&mut self, text: Option<String>) {
        self.core.memory.shown::<TextState>(state_id(self.core.id)).change =
            Some(text.map_or(adapter::DraftChange::Revert, adapter::DraftChange::Text));
        self.core.run.wake.set(true);
    }

    /// Selects the whole text of the text part at the start of the next run, as a field that takes focus with
    /// `select_on_focus` does: a font field's click that opens its list selects the name, so typing replaces it.
    pub fn select_all_text(&mut self) {
        self.core.memory.shown::<TextState>(state_id(self.core.id)).select_all = true;
        self.core.run.wake.set(true);
    }

    /// Accepts candidate `at` of the text part's completion list in the next run: a click on a row of the list.
    #[doc(hidden)]
    pub fn pick_completion(&mut self, at: usize) {
        self.core.memory.shown::<TextState>(state_id(self.core.id)).completions.picked = Some(at);
        self.core.run.wake.set(true);
    }

    /// The key of the text part step 8b ran in this run; `None` for a component that declared none.
    pub fn text_part(&self) -> Option<PartKey> {
        self.core.texts.as_ref().map(|(part, _)| *part)
    }

    /// The result of the text part `key` this run: the draft, the laid-out text and the caret blink. `None` while
    /// the text service has not produced one (a part first registered this run), and for a key `parts` never
    /// registered, which is a debug assertion.
    pub fn text<V: 'static>(&self, key: &TextKey<V>) -> Option<&TextOutcome<V>> {
        let outcome =
            self.core.texts.as_ref().filter(|(part, _)| *part == key.key()).and_then(|(_, text)| text.as_any().downcast_ref::<RecordedText<V>>());
        let outcome = outcome.map(|recorded| &recorded.outcome);
        debug_assert!(outcome.is_some() || self.core.texts.is_none(), "the text part {:?} was never registered by `parts`", key.key());
        outcome
    }
}
