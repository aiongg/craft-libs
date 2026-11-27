//! The editor part of the pipeline (18.4): `PartsCx::editor` registers the part, step 8b gives it the run's ordered
//! `EditorInput`, and `Cx::editor_state` reports the caret, the visible lines and the undo state back to the IME
//! engine (`contract::rt::ime`, kept on the node after the run) and to the rt access writer (`runtime::access::text`).

use super::Component;
use super::Ui;
use super::cx::{Cx, Edit as _, PartsCx};
use crate::contract::rt::ime::{self, BodyReport, EditorFacts};
use crate::primitives::Id;
use crate::primitives::{EditorPurpose, EditorState};
use crate::runtime::access::text::{Line, Run, TextFacts};
use crate::runtime::commands::{CommandSet, Commands as _, EditCommands, FireSource, Fired};
use crate::runtime::focus::KeyUse;
use craft_behavior::{EditCommand, EditorInput, PartKey};
use craft_ui_rt::{Pos2, Rect};
use std::sync::{Arc, Mutex};

/// What `PartsCx::editor` leaves for step 8b.
pub(crate) struct EditorDecl {
    pub key: PartKey,
    pub rect: Rect,
    pub purpose: EditorPurpose,
}

/// Where an editor input or a command goes among the run's input: the input's place in the node's queue, then its kind
/// (editor inputs, then the commands of the component's scope). A command of no one input goes last.
pub(crate) type Order = (usize, u8);

/// One step of an editor's run, in the order of the input it came from ([`Cx::editor_steps`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum EditorStep<S> {
    /// An input of the editor part.
    Input(EditorInput),
    /// A command of the component's scope that a key or another source fired.
    Command(S, Fired),
}

/// The editor part as step 8b set it up, for `editor_state`.
pub(crate) struct EditorInfo {
    /// The keys the focused editor keeps (`keys_of`), which its stop registers.
    pub keys: KeyUse,
    pub rect: Rect,
    pub purpose: EditorPurpose,
    pub focused: bool,
    pub read_only: bool,
    pub commit_now: bool,
}

impl<C: Component> PartsCx<'_, C> {
    /// Registers the component's one editor part (`INPUT = Input::Editor`).
    pub fn editor(&mut self, key: PartKey, rect: Rect, purpose: EditorPurpose) {
        debug_assert!(self.text.editor.is_none() && self.text.decl.is_none(), "a component registers one text or editor part");
        self.text.editor = Some(EditorDecl { key, rect, purpose });
    }
}

/// The keys an editor part keeps: a code editor keeps Tab.
pub(crate) fn keys_of(base: KeyUse, purpose: EditorPurpose) -> KeyUse {
    match purpose {
        EditorPurpose::Code => base.with_keep(crate::runtime::focus::KeepKeys::Tab),
        _ => base,
    }
}

/// Step 8b for an editor part: the run's inputs in the order they were queued (input method edits, an AccessKit
/// selection, keys, text and edit commands). A read-only editor receives only inputs that change no text; an undo or
/// redo the editor has no step for goes on to the app command.
pub(crate) fn run<C: Component>(cx: &mut Cx<'_, C>, step: &mut super::text::TextStep, keys: KeyUse) {
    let Some(decl) = step.editor.take() else { return };
    let keys = keys_of(keys, decl.purpose);
    // A press that leaves the editor ends its gesture, as a focus loss ends a text field's draft.
    let commit_now = super::text::press_left(cx.core.input());
    if commit_now {
        cx.commit();
    }
    let focused = cx.core.focus.focused;
    let read_only = cx.core.resolved.read_only || !cx.core.resolved.enabled;
    // The queue holds only what was routed to this node: a composition committed as focus left arrives after the focus
    // moved, so the inputs are taken whether or not the node still has focus.
    let mut inputs: Vec<(Order, EditorInput)> =
        cx.core.editor_input(keys.keep).into_iter().enumerate().map(|(place, input)| ((place, 0), input)).collect();
    if read_only {
        inputs.retain(|(_, input)| input.is_read_only_safe());
    } else {
        let (can_undo, can_redo) = cx.core.run.ime_history;
        let mut fall_through = Vec::new();
        inputs.retain(|(_, input)| {
            let EditorInput::Edit(command @ (EditCommand::Undo | EditCommand::Redo)) = input else { return true };
            let can = if *command == EditCommand::Undo { can_undo } else { can_redo };
            if !can {
                fall_through.push(*command);
            }
            can
        });
        for command in fall_through.into_iter().filter_map(EditCommands::of) {
            cx.fire(command, FireSource::Key);
        }
    }
    cx.core.editor = Some(EditorInfo { keys, rect: decl.rect, purpose: decl.purpose, focused, read_only, commit_now });
    cx.core.editor_inputs = inputs;
}

/// The editor part's rect, purpose, focus and read-only state of this run, which a body that runs inside a component
/// reports through for the component's editor part.
#[derive(Clone, Debug)]
pub struct EditorHandle {
    owner: Id,
    origin: Pos2,
    rect: Rect,
    purpose: EditorPurpose,
    focused: bool,
    read_only: bool,
    report: Arc<Mutex<BodyReport>>,
}

impl PartialEq for EditorHandle {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.rect == other.rect
            && self.purpose == other.purpose
            && self.focused == other.focused
            && self.read_only == other.read_only
    }
}

impl EditorHandle {
    /// Reports the caret, the composition, the undo state and the surrounding text for the component's editor part:
    /// the platform places its candidate window at the caret at once, and the component keeps the rest at its next run.
    pub fn report(&self, state: EditorState<'_>) {
        let caret = self.focused.then(|| state.caret.translate(Pos2::ZERO - self.origin));
        let facts = facts_of(&state);
        if let Ok(mut report) = self.report.lock() {
            report.caret = caret;
            report.facts = Some(facts);
        }
    }

    /// Notes that the body changed the editor's committed text for a reason other than the input method (typing it
    /// did not compose, a paste, undo, an app edit): the component runs once to send the new surrounding text.
    pub fn ime_application_change(&self, ui: &mut Ui<'_, '_>) {
        if let Ok(mut report) = self.report.lock() {
            report.app_change = true;
        }
        if let Some(node) = crate::contract::rt::ids::node_of(ui.cx, self.owner) {
            ui.cx.request_build_of(node);
        }
    }

    /// Ends the input method's composition, because the editor part refused an input method edit and the input
    /// method's view of the text is stale. Late events of the composition are dropped; with none running, later input
    /// is not held back.
    pub fn ime_interrupt(&self, ui: &mut Ui<'_, '_>) {
        if let Some(node) = crate::contract::rt::ids::node_of(ui.cx, self.owner) {
            ui.cx.interrupt_ime(node);
        }
    }
}

/// The input method facts of an editor's report: its composition, undo state and bounded surrounding text.
fn facts_of(state: &EditorState<'_>) -> EditorFacts {
    let surrounding = state.surrounding.as_ref().and_then(|s| match s {
        crate::primitives::EditorSurrounding::Text { text, cursor, anchor } => ime::bounded(text, *cursor, *anchor),
        crate::primitives::EditorSurrounding::Unavailable { .. } => None,
    });
    EditorFacts { composing: state.composing, shows_marked: state.shows_marked, can_undo: state.can_undo, can_redo: state.can_redo, surrounding }
}

/// The text facts of an editor's report for the rt access writer. A report with no lines has none: an editor whose
/// view describes its text (a page editor, through the page view's describer) reports its caret to the input method
/// only, and no caret is announced in an empty text.
fn text_facts(widget: Id, info: &EditorInfo, state: &EditorState<'_>) -> Option<TextFacts> {
    if state.lines.is_empty() {
        return None;
    }
    let lines = state
        .lines
        .iter()
        .map(|line| {
            let runs = if line.runs.is_empty() {
                vec![Run { bytes: 0..0, rtl: false, positions: Vec::new(), widths: Vec::new() }]
            } else {
                line.runs
                    .iter()
                    .map(|run| Run { bytes: run.bytes.clone(), rtl: run.rtl, positions: run.positions.to_vec(), widths: run.widths.to_vec() })
                    .collect()
            };
            Line { text: line.text.to_owned(), rect: line.rect, runs }
        })
        .collect();
    let text = state.lines.iter().map(|line| line.text).collect::<Vec<_>>().join("\n");
    let selection = state.selection.or(Some((state.caret_at, state.caret_at)));
    let masked = info.purpose == EditorPurpose::Password;
    Some(TextFacts { widget, text, masked, editable: !info.read_only, disabled: false, lines, selection, unit: None, value_override: None })
}

impl<C: Component> Cx<'_, C> {
    /// The editor part of this run, `None` when the component registered none.
    pub fn editor_handle(&self) -> Option<EditorHandle> {
        let info = self.core.editor.as_ref()?;
        let report = self.core.run.ime_body.borrow_mut().get_or_insert_with(Default::default).clone();
        Some(EditorHandle {
            owner: self.core.id,
            origin: self.core.rect.min,
            rect: info.rect,
            purpose: info.purpose,
            focused: info.focused,
            read_only: info.read_only,
            report,
        })
    }

    /// Whether a press moved focus away from the editor and asks it to commit its session in this run.
    pub fn editor_commit_requested(&self) -> bool {
        self.core.editor.as_ref().is_some_and(|editor| editor.commit_now)
    }

    /// Notes that the editor changed its committed text for a reason other than the input method (typing it did not
    /// compose, a paste, undo, an app edit): the platform gets the new surrounding text after the run. An input method
    /// edit needs no note; the platform made that text itself.
    pub fn ime_application_change(&self) {
        self.core.run.ime_app_change.set(true);
    }

    /// Ends the input method's composition after this run, because the editor refused an input method edit and the
    /// input method's view of the text is stale. A composition still running is ended and its late events are dropped;
    /// with none running, later input is not held back.
    pub fn ime_interrupt(&self) {
        self.core.run.ime_refused.set(true);
    }

    /// The editor part's inputs of this run in the order they arrived; call it in `update`. The editor applies them
    /// to its own document.
    pub fn editor_input(&mut self) -> impl Iterator<Item = EditorInput> + '_ {
        std::mem::take(&mut self.core.editor_inputs).into_iter().map(|(_, input)| input)
    }

    /// The editor part's inputs of this run and the commands of set `S` routed to this component, as one stream: the
    /// inputs in the order they were queued, then the commands (a command runs at event time on the board, so it has
    /// no place among the inputs). Call it in `update` in place of `editor_input` and `take_command`.
    pub fn editor_steps<S: CommandSet>(&mut self) -> Vec<EditorStep<S>> {
        let mut steps: Vec<(Order, EditorStep<S>)> =
            std::mem::take(&mut self.core.editor_inputs).into_iter().map(|(order, input)| (order, EditorStep::Input(input))).collect();
        while let Some((command, fired)) = self.take_command::<S>() {
            steps.push(((usize::MAX, 1), EditorStep::Command(command, fired)));
        }
        steps.into_iter().map(|(_, step)| step).collect()
    }

    /// Reports the editor's caret, visible lines, selection, composition, surrounding text and undo state once per run,
    /// from `update` or `paint`. The caret places the platform's candidate window while the editor has focus.
    pub fn editor_state(&self, state: EditorState<'_>) {
        let Some(info) = self.core.editor.as_ref() else {
            debug_assert!(false, "`editor_state` needs an editor part registered by `parts`");
            return;
        };
        if info.focused {
            self.core.ime_cursor(state.caret);
        }
        *self.core.run.editor_facts.borrow_mut() = Some(facts_of(&state));
        if let Some(facts) = text_facts(self.core.id, info, &state) {
            *self.core.text_access.borrow_mut() = Some(facts);
        }
    }
}
