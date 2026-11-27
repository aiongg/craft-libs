//! The IME engine on craft-ui-rt (docs/contract-rt.md section 10, "IME engine"). ui-rt runs one
//! platform IME session per window, for the focused node when it takes text input
//! (`Component::text_input`), and places the candidate window at the caret rect the node's last run
//! reported (`Cx::ime_cursor`). Every text input in every app, the canvas text tools included, goes
//! through this one engine:
//!
//! - A preedit is queued on the focused node as `EditorInput::Preedit` and marks its path, so only
//!   that node and the scopes above it run; the node keeps the composition.
//! - A commit is queued as `EditorInput::ImeCommit` followed by its text as `Intent::Text`: the
//!   editor inserts it once.
//! - While the input method composes, the node holds every key (the composition's).
//! - A node that loses focus while composing (a click elsewhere, Tab from a script, a dialog that
//!   opened) commits the composition itself before the focus moves on; ui-rt then stops the
//!   platform session, so the platform drops its marked text and nothing is inserted twice.
//! - Typed text outside a composition (`Event::Text`) and pasted text reach a typing node, and a
//!   node that types ahead, as `Intent::Text`.
//! - A press on the composing node commits the composition before the press is queued, and ends
//!   the platform's composition (`ImeRequest::Reset`): the run sees the commit, then the press,
//!   and the click lands. Presses go through ui-rt's press machine, one per window, stepped once
//!   per event, so this is one press like any other.
//! - ui-rt delivers each preedit and commit once, so the run that took a commit knows it applied
//!   it. An editor that refuses an input method edit (`Cx::ime_interrupt`), or that stops showing
//!   the marked text it showed, ends the composition the same way; ui-rt drops the ended
//!   composition's late events until the platform starts a new session.
//! - The editor's facts (surrounding text, undo state) are reported through the run
//!   (`Cx::editor_state`) and kept here. The surrounding text goes to the platform at a session
//!   start and when the editor changed its text for a reason other than the input method
//!   (`Cx::ime_application_change`), never for the input method's own edits.

use std::sync::{Arc, Mutex};

use craft_behavior::{EditorInput, Intent};
use craft_ui_rt::{BuildCx, Event, EventCx, Handled, ImeSurrounding, Pos2, Rect};

use super::node::{Kept, NodeInput};
use crate::contract::cx::RunIo;

/// The most committed text around the caret the platform is given, in bytes: a window around the
/// selection, never the whole document.
const MAX_SURROUNDING_BYTES: usize = 4000;

/// What an editor reports of itself once per run ([`crate::primitives::EditorState`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct EditorFacts {
    /// The editor shows marked text now.
    pub(crate) composing: bool,
    /// The editor draws the marked text of the preedits it is given.
    pub(crate) shows_marked: bool,
    pub(crate) can_undo: bool,
    pub(crate) can_redo: bool,
    /// The committed text around the caret: text, cursor and anchor byte offsets, bounded.
    pub(crate) surrounding: Option<(String, usize, usize)>,
}

/// What a body that runs inside a component reports for the component's editor part
/// (`EditorHandle`), shared between the handle and the node.
#[derive(Debug, Default)]
pub(crate) struct BodyReport {
    pub(crate) facts: Option<EditorFacts>,
    /// The caret rect in the node's coordinates.
    pub(crate) caret: Option<Rect>,
    /// The body changed the editor's text for a reason other than the input method.
    pub(crate) app_change: bool,
}

/// A node's input method state.
#[derive(Clone, Debug, Default)]
pub(crate) struct ImeKept {
    /// The text the input method composes, while it composes.
    pub(crate) preedit: Option<String>,
    /// The caret rect the node's last run reported, in the node's coordinates.
    pub(crate) caret: Option<Rect>,
    /// The editor's facts as its last run reported them.
    pub(crate) facts: Option<EditorFacts>,
    /// The surrounding text and its revision, which moves only for a change the input method did
    /// not make.
    pub(crate) surrounding: Option<ImeSurrounding>,
    /// The reports of a body that runs inside the component, for its editor part.
    pub(crate) body: Option<Arc<Mutex<BodyReport>>>,
    /// The text run sub-nodes the last access report wrote, for mapping a selection request back
    /// (`runtime::access::text`).
    pub(crate) text_runs: Vec<crate::runtime::access::text::Entry>,
}

impl ImeKept {
    /// The input method composes for the node.
    pub(crate) fn composing(&self) -> bool {
        self.preedit.is_some()
    }

    /// The caret rect for the platform's candidate window: the run's, else the body's.
    pub(crate) fn caret(&self) -> Option<Rect> {
        self.caret.or_else(|| self.body.as_ref().and_then(|b| b.lock().ok()?.caret))
    }

    /// The surrounding text when its revision is not `since` (ui-rt's IME sync).
    pub(crate) fn surrounding_since(&self, since: Option<u64>) -> Option<ImeSurrounding> {
        self.surrounding.as_ref().filter(|s| Some(s.revision) != since).cloned()
    }

    /// Whether the editor had something to undo and to redo at its last report.
    pub(crate) fn history(&self) -> (bool, bool) {
        self.facts.as_ref().map_or((false, false), |f| (f.can_undo, f.can_redo))
    }

    /// Takes a new report: an editor that showed marked text, or says it draws it, and reports none
    /// while the input method composes ended the composition itself (a press in its text, a
    /// selection, an app edit over the marked text). Returns whether the composition must end.
    fn note(&mut self, facts: EditorFacts, app_change: bool) -> bool {
        let was = self.facts.as_ref().is_some_and(|f| f.composing);
        let ended = !facts.composing && (was || facts.shows_marked) && self.composing();
        if let Some((text, cursor, anchor)) = &facts.surrounding {
            let next = match &self.surrounding {
                Some(s) if s.text == *text && s.cursor == *cursor && s.anchor == *anchor => None,
                // The first report, and every change the input method did not make, is a new revision.
                Some(s) if app_change => Some(s.revision + 1),
                None => Some(1),
                // An input method edit keeps the revision: the platform made that text itself.
                Some(s) => Some(s.revision),
            };
            if let Some(revision) = next {
                self.surrounding = Some(ImeSurrounding { revision, text: text.clone(), cursor: *cursor, anchor: *anchor });
            }
        }
        self.facts = Some(facts);
        ended
    }
}

/// The bounded window of `text` around the cursor and anchor (byte offsets), with the offsets
/// moved into it. Offsets that are not character boundaries give no text.
pub(crate) fn bounded(text: &str, cursor: usize, anchor: usize) -> Option<(String, usize, usize)> {
    if !text.is_char_boundary(cursor) || !text.is_char_boundary(anchor) {
        return None;
    }
    let (lo, hi) = (cursor.min(anchor), cursor.max(anchor));
    let room = MAX_SURROUNDING_BYTES.saturating_sub(hi - lo) / 2;
    let mut start = lo.saturating_sub(room);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    let mut end = (hi + room).min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let start = start.min(lo);
    Some((text[start..end.max(hi)].to_owned(), cursor - start, anchor - start))
}

fn text(kept: &mut Kept, text: &str) {
    kept.queue.push(NodeInput::Intent { intent: Intent::Text(std::sync::Arc::from(text)), horizontal: false, shift: false });
}

fn commit(kept: &mut Kept, committed: &str) {
    kept.queue.push(NodeInput::Ime(EditorInput::ImeCommit));
    if !committed.is_empty() {
        text(kept, committed);
    }
}

/// Text, IME and paste events on a focused node. `text_input`: the node takes text input.
pub(crate) fn event(text_input: bool, kept: &mut Kept, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
    match ev {
        Event::ImePreedit { text, cursor } if text_input => {
            kept.ime.preedit = (!text.is_empty()).then(|| text.clone());
            kept.queue.push(NodeInput::Ime(EditorInput::preedit(text.as_str(), cursor.clone())));
        }
        Event::ImeCommit(committed) if text_input => {
            // An empty commit with nothing marked inserts nothing and replaces no selection.
            if committed.is_empty() && kept.ime.preedit.is_none() {
                return Handled::Yes;
            }
            kept.ime.preedit = None;
            commit(kept, committed);
        }
        Event::Text(typed) | Event::Paste(typed) => {
            let Some(Intent::Text(typed)) = super::keys::text_intent(kept.focus.stop.as_ref(), typed) else { return Handled::No };
            text(kept, &typed);
        }
        _ => return Handled::No,
    }
    cx.request_build();
    Handled::Yes
}

/// The node lost focus: a composition it holds is committed into its editor first. ui-rt ends the
/// platform's session and drops its late events.
pub(crate) fn focus_left(kept: &mut Kept, cx: &mut EventCx<'_>) {
    if let Some(preedit) = kept.ime.preedit.take() {
        commit(kept, &preedit);
        cx.request_build();
    }
}

/// A press went down on the node: a composition it holds is committed before the press is queued,
/// so the run applies the commit, then the press, and the platform's composition ends.
pub(crate) fn pressed(kept: &mut Kept, cx: &mut EventCx<'_>) {
    if let Some(preedit) = kept.ime.preedit.take() {
        commit(kept, &preedit);
        cx.interrupt_ime();
        cx.request_build();
    }
}

/// After a run: the editor's report is kept, and a refused input method edit, or a composition the
/// editor ended itself, ends the platform's composition. `origin` is the node's top left in window
/// coordinates.
pub(crate) fn after_run(ime: &mut ImeKept, run: &RunIo<'_>, origin: Pos2, cx: &mut BuildCx<'_>) {
    ime.caret = run.ime_cursor.get().map(|caret| caret.translate(Pos2::ZERO - origin));
    let mut interrupt = run.ime_refused.get();
    if let Some(facts) = run.editor_facts.borrow_mut().take() {
        interrupt |= ime.note(facts, run.ime_app_change.get());
    }
    if let Some(body) = run.ime_body.borrow_mut().take() {
        ime.body = Some(body);
    }
    let body = ime.body.as_ref().and_then(|b| b.lock().ok().and_then(|mut b| b.facts.take().map(|f| (f, std::mem::take(&mut b.app_change)))));
    if let Some((facts, app_change)) = body {
        interrupt |= ime.note(facts, app_change);
    }
    if interrupt {
        ime.preedit = None;
        let node = cx.node();
        cx.interrupt_ime(node);
    }
}
