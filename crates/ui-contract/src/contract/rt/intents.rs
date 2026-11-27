//! Intents on craft-ui-rt (docs/contract-rt.md sections 4 and 10). The key stage queues intents on
//! the node that takes them (`NodeInput::Intent`, [`super::keys`]); its run reads them through
//! `Cx::intents`, the contract's `Intents` with its takers, and an editor reads them with its IME
//! input in arrival order (`Cx::editor_input`). `Activate` sets the run's `clicked`.
//!
//! A layer child's scope takes what its focused stop left: an Enter whose scope submits
//! (`EnterRule::Submit`) and an Escape whose scope cancels. Under `CancelRule::ClearThenClose` the
//! first Escape clears the scope's content when its body says it has some (`Ui::clearable`), and
//! the next one closes it. Both are queued on the layer child, which marks it, and its body reads
//! them in that frame (`Ui::take_submit`, `Ui::take_clear`).

use craft_behavior::focus::{EscapeFacts, EscapeStep, escape_step};
use craft_behavior::{AccessRequest, EditorInput, Intent, PartRef};
use craft_ui_rt::{Key, KeyInput};

use super::node::NodeInput;
use super::ui::Ui;
use crate::runtime::focus::{EnterRule, FocusPolicy, KeepKeys};
use crate::runtime::keys::Intents;

/// The intents of a run's queue; `rtl` (asked only when a horizontal move is queued) reads
/// horizontal moves in the node's direction. AccessKit actions become access requests on the part
/// whose sub-node they name (`part_of` maps a sub-node key to its part); a SetValue with a number is
/// also the run's set value. A Click or Focus on the node's own node is served elsewhere (the
/// run's activation, ui-rt's focus).
pub(crate) fn of_queue(input: &[NodeInput], part_of: impl Fn(u64) -> Option<PartRef>, rtl: impl FnOnce() -> bool) -> Intents {
    let mut items = Vec::new();
    let mut chords = Vec::new();
    let mut nudge = None;
    let mut access = Vec::new();
    for (place, i) in input.iter().enumerate() {
        match i {
            NodeInput::Intent { intent, horizontal, shift } => items.push((intent.clone(), place, *horizontal, *shift)),
            NodeInput::Chord(chord) => chords.push(*chord),
            NodeInput::Nudge { direction, mods } => nudge = Some((*direction, *mods)),
            NodeInput::Access(a) if a.sub.is_some() || !matches!(a.action, accesskit::Action::Click | accesskit::Action::Focus) => {
                access.push((a.clone(), a.sub.and_then(&part_of)));
            }
            _ => {}
        }
    }
    let rtl = items.iter().any(|(intent, _, horizontal, _)| *horizontal && matches!(intent, Intent::Move(..))) && rtl();
    let mut intents = Intents::queued(items, chords, nudge, rtl);
    for (a, target) in access {
        if let Some(value) = a.numeric_value() {
            intents.note_set_value(target, value);
        }
        intents.push(Intent::Access(AccessRequest::new(a.action, target, a.data)));
    }
    intents
}

/// The editor input of a run's queue in arrival order: IME input as it came, and each intent the
/// editor keeps under `keep` that no taker took, taken from `intents`.
pub(crate) fn editor_input(input: &[NodeInput], intents: &mut Intents, keep: KeepKeys) -> Vec<EditorInput> {
    let mut out = Vec::new();
    for (place, i) in input.iter().enumerate() {
        match i {
            NodeInput::Ime(e) => out.push(e.clone()),
            NodeInput::Intent { .. } => out.extend(intents.take_editor_at(place, keep)),
            _ => {}
        }
    }
    out
}

/// What a layer child's scope took for its body, and what its body said about its content.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ScopeIntents {
    /// An Enter the focused stop left, for a scope that submits.
    pub(crate) submit: bool,
    /// An Escape that clears the scope's content.
    pub(crate) clear: bool,
    /// The body has content an Escape clears; `None` when it did not say.
    pub(crate) clearable: Option<bool>,
}

impl ScopeIntents {
    /// What the body of a layer child's next run reads: the queued intents, taken.
    pub(crate) fn take_for_body(&mut self) -> ScopeIntents {
        ScopeIntents { submit: std::mem::take(&mut self.submit), clear: std::mem::take(&mut self.clear), clearable: None }
    }
}

/// What a layer child does with a key its content left.
pub(crate) enum ScopeStep {
    /// Nothing: it goes on (Tab to the trap, the rest up the path).
    No,
    /// Queued for the body's next run.
    Queued,
    /// The Escape closes the layer child.
    Close,
}

/// The scope rules of a layer child with `policy` for `k`: Enter submits under `EnterRule::Submit`;
/// Escape clears or closes by the cancel rule.
pub(crate) fn scope_key(scope: &mut ScopeIntents, policy: FocusPolicy, k: &KeyInput) -> ScopeStep {
    if !k.down {
        return ScopeStep::No;
    }
    match k.key {
        Key::Enter if policy.enter == EnterRule::Submit && !k.mods.cmd && !k.mods.ctrl => {
            scope.submit = true;
            ScopeStep::Queued
        }
        Key::Escape => {
            let facts = EscapeFacts::new().scope_clearable(scope.clearable.unwrap_or(false));
            match escape_step(facts, policy.cancel) {
                EscapeStep::ClearScope => {
                    scope.clear = true;
                    ScopeStep::Queued
                }
                EscapeStep::CloseScope => ScopeStep::Close,
                _ => ScopeStep::No,
            }
        }
        _ => ScopeStep::No,
    }
}

impl Ui<'_, '_> {
    /// Takes the Submit of the layer child whose body this is: an Enter its focused stop left, under
    /// `EnterRule::Submit` (a dialog's default action). Once per Enter; false in every other flow.
    pub fn take_submit(&mut self) -> bool {
        std::mem::take(&mut self.flow.scope.submit)
    }

    /// Takes the clearing Escape of the layer child whose body this is (`CancelRule::ClearThenClose`
    /// while the body said it has content): the body clears its content.
    pub fn take_clear(&mut self) -> bool {
        std::mem::take(&mut self.flow.scope.clear)
    }

    /// Says whether the layer child whose body this is has content an Escape clears before the next
    /// one closes it (`CancelRule::ClearThenClose`).
    pub fn clearable(&mut self, has_content: bool) {
        self.flow.scope.clearable = Some(has_content);
    }
}
