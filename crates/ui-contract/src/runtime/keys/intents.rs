//! The intents routed to a component and what it leaves untaken: Submit, Cancel, Escape and context requests.

use super::{Nudge, Source};
use crate::primitives::Id;
use crate::runtime::focus;
use craft_behavior::{AccessRequest, EditCommand, EditorInput, Intent, MoveMods, NavMove, PartRef};
use craft_keymap::{Chord, Key, Mods};
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

    /// Queues the reset a label asks of its control: it resets the whole binding (`take_reset` gives `Some(true)`).
    pub(crate) fn push_whole_reset(&mut self) {
        self.push(Intent::Reset);
        self.whole_reset = true;
    }

    /// Takes the next typed digit.
    pub fn take_digit(&mut self) -> Option<u8> {
        let at = self.items.iter().position(|(item, taken)| !taken && matches!(item, Intent::Digit(_)))?;
        self.items[at].1 = true;
        match self.items[at].0 {
            Intent::Digit(digit) => Some(digit),
            _ => None,
        }
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

    /// Leaves an intent this pass's taker took to the enclosing scope, in the same pass: the next taker gets it. No
    /// input is queued or copied.
    pub(crate) fn pass_on(&mut self, intent: Intent) {
        if let Some((_, taken)) = self.items.iter_mut().find(|(item, taken)| *taken && item.same_kind(&intent)) {
            *taken = false;
        }
    }

    /// Takes an intent equal to `intent` in every field, once per pass: `Intent::delete(false, Granularity::Grapheme)`
    /// takes Backspace and leaves Delete, which `take` would also take.
    pub fn take_exact(&mut self, intent: &Intent) -> bool {
        match self.items.iter().position(|(item, taken)| !taken && item == intent) {
            Some(at) => {
                self.items[at].1 = true;
                true
            }
            None => false,
        }
    }

    /// Takes the next move, whichever it is: for a component that acts on every move. A move taken is the component's,
    /// so the Page Up, Page Down, Home and End it takes never reach the scroll owner around it (a page view under a
    /// focused page part); a component that acts on some moves takes those with [`take_move_where`](Self::take_move_where)
    /// and leaves the rest.
    pub fn take_move(&mut self) -> Option<(NavMove, MoveMods)> {
        self.take_move_where(|_, _| true)
    }

    /// Takes the next move that `wanted` accepts; the others stay for a later taker, or for the scroll owner around a
    /// focused component when they scroll (9.2 rule 5).
    pub fn take_move_where(&mut self, wanted: impl Fn(NavMove, MoveMods) -> bool) -> Option<(NavMove, MoveMods)> {
        let at = self.items.iter().position(|(item, taken)| !taken && matches!(item, Intent::Move(mv, mods) if wanted(*mv, *mods)))?;
        self.items[at].1 = true;
        match self.items[at].0 {
            Intent::Move(mv, mods) => {
                if matches!(mv, NavMove::Back | NavMove::Forward | NavMove::Up | NavMove::Down) {
                    self.took_arrows.push(mv);
                }
                Some((mv, mods))
            }
            _ => None,
        }
    }

    /// The arrow moves `take_move` took this pass: the run step checks that the component holds those keys.
    pub(crate) fn taken_arrow_moves(&self) -> &[NavMove] {
        &self.took_arrows
    }

    /// Takes the next AccessKit request that asks a scroll region to scroll, or with `into_view` to show its target.
    /// A component that scrolls reads its own requests through it.
    pub fn take_scroll_access(&mut self, into_view: bool) -> Option<AccessRequest> {
        use craft_behavior::AccessAction as Action;
        let at = self.items.iter().position(|(item, taken)| {
            !taken
                && matches!(
                    item,
                    Intent::Access(request)
                        if request.target.is_none()
                            && (request.action == Action::ScrollIntoView) == into_view
                            && matches!(
                                request.action,
                                Action::ScrollUp | Action::ScrollDown | Action::ScrollLeft | Action::ScrollRight | Action::ScrollIntoView | Action::SetScrollOffset
                            )
                )
        })?;
        self.items[at].1 = true;
        match &self.items[at].0 {
            Intent::Access(request) => Some(request.clone()),
            _ => None,
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

    /// Takes the next typed text.
    pub fn take_text(&mut self) -> Option<Arc<str>> {
        let at = self.items.iter().position(|(item, taken)| !taken && matches!(item, Intent::Text(_)))?;
        self.items[at].1 = true;
        match &self.items[at].0 {
            Intent::Text(text) => Some(Arc::clone(text)),
            _ => None,
        }
    }

    /// Takes the physical arrow nudge for canvas content: a unit direction and the modifiers.
    pub fn take_nudge(&mut self) -> Option<(Vec2, Mods)> {
        self.nudge.take().map(|nudge| (nudge.direction, nudge.mods))
    }

    /// The keys that were down when the arrow of the nudge not yet taken went down, the arrow not among them; empty
    /// without one. Unlike `keys::held`, which is the state after the frame's input, it keeps the order of the frame's
    /// events: a key let go after the arrow in the same frame was held with it, and one pressed after it was not.
    pub fn nudge_held(&self) -> &[Key] {
        self.nudge.as_ref().map_or(&[], |nudge| &nudge.held)
    }

    /// Takes the chords this pass's presses spell, for a component whose key use is `KeyUse::RECORDER`; empty for any
    /// other component. A recorder gets no other key intents, and no command runs on its presses.
    pub fn take_chords(&mut self) -> Vec<Chord> {
        std::mem::take(&mut self.chords)
    }

    /// Takes the earliest queued chord among `wanted` (a claim's chords, `KeyUse::claims`), so a caller applies the
    /// claimed keys of a run in the order they were pressed.
    pub(crate) fn take_first_chord(&mut self, wanted: &[Chord]) -> Option<Chord> {
        let at = self.chords.iter().position(|chord| wanted.contains(chord))?;
        Some(self.chords.remove(at))
    }

    /// Takes the AccessKit `SetValue` request aimed at the component's own node.
    pub fn take_set_value(&mut self) -> Option<f64> {
        self.take_set_value_of(None)
    }

    /// Takes the AccessKit `SetValue` request aimed at the sub-node of `part`.
    pub fn take_set_value_at(&mut self, part: impl Into<PartRef>) -> Option<f64> {
        self.take_set_value_of(Some(part.into()))
    }

    fn take_set_value_of(&mut self, part: Option<PartRef>) -> Option<f64> {
        match self.set_value {
            Some((target, value)) if target == part => {
                self.set_value = None;
                Some(value)
            }
            _ => None,
        }
    }

    /// Whether an intent of the kind of `intent` is routed and not taken.
    pub fn untaken(&self, intent: &Intent) -> bool {
        self.items.iter().any(|(item, taken)| !taken && item.same_kind(intent))
    }

    /// Takes the next intent an editor reads as input: typed text, a move, a delete, an edit command, Enter as a
    /// newline with its Shift, Tab and Shift+Tab, and Escape as a cancel. An arrow move the editor's `keep` does not
    /// keep stays for the component's navigation (18.4), and with `KeepKeys::None` so do Home, End and the page keys:
    /// the editor is a part of a grid that navigates until a cell opens. With it, the place among the frame's events of
    /// the press or text it came from, `None` for an intent of no one event.
    pub(crate) fn take_editor_input(&mut self, keep: focus::KeepKeys) -> Option<(EditorInput, Option<usize>)> {
        let kept = |item: &Intent| match item {
            Intent::Move(NavMove::Up | NavMove::Down, _) => matches!(keep, focus::KeepKeys::BlockArrows | focus::KeepKeys::Tab),
            Intent::Move(NavMove::Back | NavMove::Forward, _) => keep != focus::KeepKeys::None,
            Intent::Move(NavMove::Start | NavMove::End | NavMove::PageUp | NavMove::PageDown, _) => keep != focus::KeepKeys::None,
            _ => true,
        };
        let at = self.items.iter().position(|(item, taken)| !taken && kept(item) && editor_input_of(item, false).is_some())?;
        self.items[at].1 = true;
        let source = self.sources.get(at).copied().flatten();
        editor_input_of(&self.items[at].0, source.is_some_and(|source| source.shift)).map(|input| (input, source.map(|source| source.place)))
    }

    /// Takes the next `Intent::Access` request that `wanted` accepts.
    #[doc(hidden)]
    pub fn take_access_where(&mut self, wanted: impl Fn(&AccessRequest) -> bool) -> Option<AccessRequest> {
        let at = self.items.iter().position(|(item, taken)| !taken && matches!(item, Intent::Access(request) if wanted(request)))?;
        self.items[at].1 = true;
        match &self.items[at].0 {
            Intent::Access(request) => Some(request.clone()),
            _ => None,
        }
    }

    /// Takes the next `Intent::Access` request.
    pub fn take_access(&mut self) -> Option<AccessRequest> {
        let at = self.items.iter().position(|(item, taken)| !taken && matches!(item, Intent::Access(_)))?;
        self.items[at].1 = true;
        match &self.items[at].0 {
            Intent::Access(request) => Some(request.clone()),
            _ => None,
        }
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

    /// An AccessKit SetValue with a number on `part`'s sub-node (`None`: the component's own node), for
    /// `take_set_value` and `take_set_value_at`.
    pub(crate) fn note_set_value(&mut self, part: Option<PartRef>, value: f64) {
        self.set_value = Some((part, value));
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
