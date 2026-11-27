//! The intents routed to a component and what it leaves untaken: Submit, Cancel, Escape and context requests.

use super::route::push;
use super::{Nudge, Source};
use crate::primitives::id::Id;
use crate::runtime::focus;
use craft_behavior::{EditCommand, EditorInput, Intent, NavMove, PartRef};
use craft_keymap::{Chord, Mods};
use craft_ui_rt::Vec2;
use std::sync::Arc;

/// An owned copy of the intents routed to one id this pass; read through `cx.intents()`.
#[derive(Clone, Debug, Default)]
pub struct Intents {
    items: Vec<(Intent, bool)>,
    /// The press or text each of `items` came from (`keys::Item::source`); read by index, so an item pushed without
    /// one has none.
    sources: Vec<Option<Source>>,
    took_arrows: Vec<NavMove>,
    nudge: Option<Nudge>,
    chords: Vec<Chord>,
    set_value: Option<(Option<PartRef>, f64)>,
    whole_reset: bool,
}

impl Intents {
    /// Queues an intent for this pass, as the key stage does for its own.
    pub(crate) fn push(&mut self, intent: Intent) {
        self.items.push((intent, false));
    }

    /// Whether an AccessKit click names some part of the component; reads without taking.
    pub(crate) fn any_access_click(&self) -> bool {
        self.items.iter().any(|(item, taken)| {
            !taken && matches!(item, Intent::Access(request) if request.target.is_some() && request.action == accesskit::Action::Click)
        })
    }

    /// Whether an AccessKit click names `part`; reads without taking.
    pub(crate) fn access_click(&self, part: PartRef) -> bool {
        self.items.iter().any(|(item, taken)| {
            !taken && matches!(item, Intent::Access(request) if request.target == Some(part) && request.action == accesskit::Action::Click)
        })
    }

    /// Takes a `Reset`, once per pass: `Some(true)` when the label step sent it (the whole binding resets),
    /// `Some(false)` when a key sent it (it resets the focused part when that part has its own reset).
    pub(crate) fn take_reset(&mut self) -> Option<bool> {
        self.take(Intent::Reset).then_some(self.whole_reset)
    }

    /// Takes an intent of the kind of `intent`, once per pass: the first taker gets it.
    pub fn take(&mut self, intent: Intent) -> bool {
        match self.items.iter().position(|(item, taken)| !taken && item.same_kind(&intent)) {
            Some(at) => {
                self.items[at].1 = true;
                true
            }
            None => false,
        }
    }

    /// Takes the next edit command.
    pub fn take_edit(&mut self) -> Option<EditCommand> {
        let at = self.items.iter().position(|(item, taken)| !taken && matches!(item, Intent::Edit(_)))?;
        self.items[at].1 = true;
        match self.items[at].0 {
            Intent::Edit(command) => Some(command),
            _ => None,
        }
    }

    /// Takes the physical arrow nudge for canvas content: a unit direction and the modifiers.
    pub fn take_nudge(&mut self) -> Option<(Vec2, Mods)> {
        self.nudge.take().map(|nudge| (nudge.direction, nudge.mods))
    }

    /// Takes the earliest queued chord among `wanted` (a claim's chords, `KeyUse::claims`), so a caller applies the
    /// claimed keys of a run in the order they were pressed.
    pub(crate) fn take_first_chord(&mut self, wanted: &[Chord]) -> Option<Chord> {
        let at = self.chords.iter().position(|chord| wanted.contains(chord))?;
        Some(self.chords.remove(at))
    }

    /// Whether an intent of the kind of `intent` is routed and not taken.
    pub fn untaken(&self, intent: &Intent) -> bool {
        self.items.iter().any(|(item, taken)| !taken && item.same_kind(intent))
    }
}

impl Intents {
    /// The intents a craft-ui-rt node queued since its last run (`contract::rt::intents`), oldest first: each with its
    /// place in the node's queue, whether a horizontal arrow sent it (read in the node's direction, `rtl`) and whether
    /// Shift was held. `chords` are the presses a recorder or a claim took; `nudge` the last arrow nudge.
    pub(crate) fn queued(items: Vec<(Intent, usize, bool, bool)>, chords: Vec<Chord>, nudge: Option<(Vec2, Mods)>, rtl: bool) -> Intents {
        let flip = |mv| match (rtl, mv) {
            (true, NavMove::Back) => NavMove::Forward,
            (true, NavMove::Forward) => NavMove::Back,
            (_, mv) => mv,
        };
        let sources = items.iter().map(|(_, place, _, shift)| Some(super::Source { place: *place, shift: *shift })).collect();
        let items = items
            .into_iter()
            .map(|(intent, _, horizontal, _)| match intent {
                Intent::Move(mv, mods) if horizontal => (Intent::Move(flip(mv), mods), false),
                intent => (intent, false),
            })
            .collect();
        Intents {
            items,
            sources,
            took_arrows: Vec::new(),
            nudge: nudge.map(|(direction, mods)| Nudge { direction, mods, held: Vec::new() }),
            chords,
            set_value: None,
            whole_reset: false,
        }
    }

    /// Takes the intent at the queue place `place` as editor input when it is one the editor keeps (the rule of
    /// [`take_editor_input`](Self::take_editor_input)) and no taker took it: an editor reads its intents and IME input
    /// in the order they arrived.
    pub(crate) fn take_editor_at(&mut self, place: usize, keep: focus::KeepKeys) -> Option<EditorInput> {
        let at = self.sources.iter().position(|s| s.is_some_and(|s| s.place == place))?;
        let (item, taken) = &self.items[at];
        let kept = match item {
            Intent::Move(NavMove::Up | NavMove::Down, _) => matches!(keep, focus::KeepKeys::BlockArrows | focus::KeepKeys::Tab),
            Intent::Move(..) => keep != focus::KeepKeys::None,
            _ => true,
        };
        if *taken || !kept {
            return None;
        }
        let shift = self.sources[at].is_some_and(|s| s.shift);
        let input = editor_input_of(item, shift)?;
        self.items[at].1 = true;
        Some(input)
    }
}

/// The editor input of `intent`, from a press with Shift held when `shift`.
fn editor_input_of(intent: &Intent, shift: bool) -> Option<EditorInput> {
    match intent {
        Intent::Text(text) => Some(EditorInput::Text(Arc::clone(text))),
        Intent::Move(mv, mods) => Some(EditorInput::Move(*mv, *mods)),
        Intent::Delete { forward, by, shift, .. } => Some(EditorInput::delete_with_shift(*forward, *by, *shift)),
        Intent::Edit(command) => Some(EditorInput::Edit(*command)),
        Intent::Submit => Some(EditorInput::newline(shift)),
        Intent::Tab => Some(EditorInput::Tab),
        Intent::BackTab => Some(EditorInput::BackTab),
        Intent::Cancel => Some(EditorInput::Cancel),
        _ => None,
    }
}

/// The keys an arena's session control borrowed this pass (`KeyUse::session`), with the ids they were routed to.
#[derive(Debug, Default)]
pub(crate) struct Lent {
    items: Vec<(Option<Id>, Intent)>,
    nudge: Option<Option<Id>>,
}
