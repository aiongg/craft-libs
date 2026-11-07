//! Focus scopes, key use and the programmatic focus target (spec 9.3, 9.4).

use craft_behavior::{CancelRule, DocKey, EditCommand, Input, OwnerClass, PartRef, ScopeKind};
use craft_keymap::Chord;
use egui::Id;

/// Where focus goes the first time a scope is pushed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Initial {
    /// Focus stays where it is.
    Keep,
    /// The scope's first stop.
    First,
    /// The stop registered as the default.
    Default,
    /// The stop registered under this name.
    Key(&'static str),
    /// Focus stays; a collection keeps its anchor.
    KeepAnchor,
}

/// What Enter does in a scope after the focused component declined it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EnterRule {
    /// Nothing: an untaken `Submit` goes to the next scope outward.
    Pass,
    /// The scope has a default action, which takes an untaken `Submit`.
    Submit,
}

/// A focus scope's rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FocusPolicy {
    /// What the scope is.
    pub kind: ScopeKind,
    /// Tab wraps inside the scope and cannot leave it.
    pub trap: bool,
    /// Focus returns to the opener when the scope disappears.
    pub restore: bool,
    /// Where focus goes when the scope first appears.
    pub initial: Initial,
    /// What Enter does after the focused component declined it.
    pub enter: EnterRule,
    /// What Escape does when nothing inside takes it.
    pub cancel: CancelRule,
    /// Ctrl+Tab cycles pages.
    pub pages: bool,
}

impl FocusPolicy {
    /// A scope that changes nothing.
    pub const NONE: FocusPolicy = FocusPolicy {
        kind: ScopeKind::Group,
        trap: false,
        restore: false,
        initial: Initial::Keep,
        enter: EnterRule::Pass,
        cancel: CancelRule::Ignore,
        pages: false,
    };
    /// A pane: an F6 stop.
    pub const PANE: FocusPolicy = FocusPolicy { kind: ScopeKind::Pane, ..FocusPolicy::NONE };
    /// A dialog: Tab stays inside, Enter submits, Escape closes.
    pub const DIALOG: FocusPolicy = FocusPolicy {
        kind: ScopeKind::Dialog,
        trap: true,
        restore: true,
        initial: Initial::First,
        enter: EnterRule::Submit,
        cancel: CancelRule::Close,
        pages: false,
    };
    /// A popup: focus moves in, Escape closes and focus returns to the opener.
    pub const POPUP: FocusPolicy = FocusPolicy {
        kind: ScopeKind::Popup,
        trap: true,
        restore: true,
        initial: Initial::First,
        enter: EnterRule::Pass,
        cancel: CancelRule::Close,
        pages: false,
    };
    /// A menu: like a popup.
    pub const MENU: FocusPolicy = FocusPolicy { kind: ScopeKind::Menu, ..FocusPolicy::POPUP };

    /// Whether Tab wraps inside the scope.
    pub const fn trap(mut self, on: bool) -> Self {
        self.trap = on;
        self
    }

    /// Whether focus returns to the opener.
    pub const fn restore(mut self, on: bool) -> Self {
        self.restore = on;
        self
    }

    /// Where focus goes first.
    pub const fn initial(mut self, initial: Initial) -> Self {
        self.initial = initial;
        self
    }

    /// What Escape does.
    pub const fn cancel(mut self, cancel: CancelRule) -> Self {
        self.cancel = cancel;
        self
    }

    /// Whether Ctrl+Tab cycles pages.
    pub const fn pages(mut self, on: bool) -> Self {
        self.pages = on;
        self
    }
}

impl Default for FocusPolicy {
    fn default() -> Self {
        FocusPolicy::NONE
    }
}

/// What `runtime::focus::request` focuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FocusTarget {
    /// A component.
    Id(Id),
    /// A focusable part of a component.
    Part(Id, PartRef),
    /// The component registered under a document key.
    Doc(DocKey),
}

/// The keys a focused component keeps from the key stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeepKeys {
    /// None.
    None,
    /// Tab.
    Tab,
    /// Left and right arrows.
    InlineArrows,
    /// All four arrows.
    BlockArrows,
}

/// The keys that activate a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActivateKeys {
    /// Neither.
    Neither,
    /// Enter.
    Enter,
    /// Space.
    Space,
    /// Both.
    Both,
}

impl ActivateKeys {
    /// Whether Enter activates.
    pub const fn enter(self) -> bool {
        matches!(self, ActivateKeys::Enter | ActivateKeys::Both)
    }

    /// Whether Space activates.
    pub const fn space(self) -> bool {
        matches!(self, ActivateKeys::Space | ActivateKeys::Both)
    }
}

/// The edit commands a component takes as `Intent::Edit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditSet(u8);

impl EditSet {
    /// No command.
    pub const NONE: EditSet = EditSet(0);
    /// Every command.
    pub const ALL: EditSet = EditSet(0b11_1111);

    const fn bit(command: EditCommand) -> u8 {
        1 << match command {
            EditCommand::Cut => 0,
            EditCommand::Copy => 1,
            EditCommand::Paste => 2,
            EditCommand::SelectAll => 3,
            EditCommand::Undo => 4,
            _ => 5,
        }
    }

    /// The set plus `command`.
    pub const fn with(self, command: EditCommand) -> EditSet {
        EditSet(self.0 | Self::bit(command))
    }

    /// Whether the set has `command`.
    pub const fn contains(self, command: EditCommand) -> bool {
        self.0 & Self::bit(command) != 0
    }
}

/// A component's declared keys.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct KeyUse {
    /// The keys kept from the key stage.
    pub keep: KeepKeys,
    /// The keys that activate.
    pub activate: ActivateKeys,
    /// Chords kept from the shortcut dispatcher while focused.
    pub claims: &'static [Chord],
    /// Edit commands delivered as `Intent::Edit`.
    pub edits: EditSet,
    /// The component's key class.
    pub owner: OwnerClass,
    /// The component's arrows step a value: Alt with an arrow is a fine move. Set per instance by a component that
    /// keeps Alt+Down for something else when it has a dropdown.
    pub value: bool,
    /// The component acts on the arrow moves routed to it (`Intent::Move`: it takes them with `take_move`), so a
    /// shortcut never shares them. A control that does not (a button) leaves them to the commands around the focus.
    pub moves: bool,
    /// The arrows are nudges for the component's content (a canvas tool or control), not moves: they arrive as
    /// `Intents::take_nudge`.
    pub nudge: bool,
}

impl KeyUse {
    /// A multi-line text field: Up and Down move the caret.
    pub const TEXT_MULTILINE: KeyUse = KeyUse::of(Input::Text).with_keep(KeepKeys::BlockArrows);
    /// A code editor: Tab inserts.
    pub const CODE: KeyUse = KeyUse::of(Input::Editor).with_keep(KeepKeys::Tab);
    /// A canvas: all arrows, no activation.
    pub const CANVAS: KeyUse = KeyUse {
        keep: KeepKeys::BlockArrows,
        activate: ActivateKeys::Neither,
        claims: &[],
        edits: EditSet::NONE,
        owner: OwnerClass::Widget,
        value: false,
        moves: false,
        nudge: true,
    };
    /// A shortcut recorder.
    pub const RECORDER: KeyUse = KeyUse { owner: OwnerClass::Recorder, nudge: false, ..KeyUse::CANVAS };

    /// The default key use of a component that takes `input`. The only place egui key filters start from.
    pub const fn of(input: Input) -> KeyUse {
        let (keep, activate, owner) = match input {
            Input::Pointer => (KeepKeys::None, ActivateKeys::Both, OwnerClass::Widget),
            Input::Text => (KeepKeys::InlineArrows, ActivateKeys::Neither, OwnerClass::Text),
            Input::Editor => (KeepKeys::BlockArrows, ActivateKeys::Neither, OwnerClass::Text),
            _ => (KeepKeys::None, ActivateKeys::Neither, OwnerClass::Widget),
        };
        // Text parts and editors list every edit command (11.3).
        let edits = match owner {
            OwnerClass::Text => EditSet::ALL,
            _ => EditSet::NONE,
        };
        KeyUse { keep, activate, claims: &[], edits, owner, value: false, moves: false, nudge: false }
    }

    /// The same use claiming `claims`.
    pub const fn with_claims(mut self, claims: &'static [Chord]) -> Self {
        self.claims = claims;
        self
    }

    /// The same use listing `edits`.
    pub const fn with_edits(mut self, edits: EditSet) -> Self {
        self.edits = edits;
        self
    }

    /// The same use with `activate` as the keys that activate: a value control that arrow and Page keys step takes none.
    pub const fn with_activate(mut self, activate: ActivateKeys) -> Self {
        self.activate = activate;
        self
    }

    /// The same use with `value` as whether its arrows step a value.
    pub const fn with_value(mut self, value: bool) -> Self {
        self.value = value;
        self
    }

    /// The same use declaring that the component acts on the moves routed to it.
    pub const fn with_moves(mut self) -> Self {
        self.moves = true;
        self
    }

    pub(crate) const fn with_keep(mut self, keep: KeepKeys) -> Self {
        self.keep = keep;
        self
    }

    /// Whether the component claims `chord`.
    pub fn claims_chord(&self, chord: &Chord) -> bool {
        self.claims.contains(chord)
    }
}
