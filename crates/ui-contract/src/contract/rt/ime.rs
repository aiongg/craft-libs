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
//!
//! Presses go through ui-rt's press machine, one per window, stepped once per event, so a click
//! during a composition is one press like any other.

use craft_behavior::{EditorInput, Intent};
use craft_ui_rt::{Event, EventCx, Handled, Rect};

use super::node::{Kept, NodeInput};

/// A node's input method state.
#[derive(Clone, Debug, Default)]
pub(crate) struct ImeKept {
    /// The text the input method composes, while it composes.
    pub(crate) preedit: Option<String>,
    /// The caret rect the node's last run reported, in the node's coordinates.
    pub(crate) caret: Option<Rect>,
}

impl ImeKept {
    /// The input method composes for the node.
    pub(crate) fn composing(&self) -> bool {
        self.preedit.is_some()
    }
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

/// The node lost focus: a composition it holds is committed into its editor first.
pub(crate) fn focus_left(kept: &mut Kept, cx: &mut EventCx<'_>) {
    if let Some(preedit) = kept.ime.preedit.take() {
        commit(kept, &preedit);
        cx.request_build();
    }
}
