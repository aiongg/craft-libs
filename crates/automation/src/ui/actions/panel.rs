//! [`ActionsPanel`]: the tree on `List`, selection, rename, drag, events, the empty state and
//! Button Mode.

use std::collections::BTreeSet;

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::labels::ActionsLabels;
use crate::actions::{Action, ActionColor, ActionList, ActionStep};

/// What the panel asks of the app while it draws: words and state the list does not hold.
pub trait ActionsHost {
    /// A step's label in the tree ("Levels", "Play Action “Sharpen”"), from the app's command
    /// catalogue.
    fn step_label(&self, step: &ActionStep) -> String;

    /// The action's keyboard shortcut text ("F2", "⇧F3"), if it has one (the app's
    /// `actions.play:<name>` override). None by default.
    fn shortcut(&self, action: &Action) -> Option<String> {
        let _ = action;
        None
    }

    /// The action being recorded into, if a recording runs. None by default.
    fn recording(&self) -> Option<usize> {
        None
    }

    /// The steps recorded into `action` but not yet appended to it (the journal since the
    /// recording started), shown after its steps. None by default.
    fn pending_steps(&self, action: usize) -> &[ActionStep] {
        let _ = action;
        &[]
    }

    /// An action is playing: the panel takes no edits (renames, moves, toggles) while it plays.
    /// False by default.
    fn playing(&self) -> bool {
        false
    }
}

/// The Actions panel's persisted section (lead call L26): Button Mode and the collapsed sets.
/// Selection and expansion are not persisted.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ActionsSettings {
    /// Button Mode: each action is a coloured button that plays it.
    pub button_mode: bool,
    /// The sets shown collapsed.
    pub collapsed_sets: Vec<String>,
}

/// A row of the tree: a set header, an action or one of its steps. Also the selection (one row at
/// a time) and the list's key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ActionSelection {
    /// The set header of this name.
    Set(String),
    /// The action at this index.
    Action(usize),
    /// Step `.1` of action `.0` (pending steps of a recording follow the action's own).
    Step(usize, usize),
}

/// What the panel keeps between passes. The app holds it beside the [`ActionList`]; only
/// `settings` is persisted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionsState {
    /// The persisted section.
    pub settings: ActionsSettings,
    /// The selected row (one at most).
    pub selected: Option<ActionSelection>,
    /// The expanded actions, by name (names are unique, so expansion survives moves).
    pub expanded: BTreeSet<String>,
    /// The action whose name is being edited in place.
    pub renaming: Option<usize>,
}

/// A function-key shortcut chosen in the context menu (Photoshop's Action Options).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShortcutChoice {
    /// F1 to F12: 1..=12.
    pub key: u8,
    /// Shift is part of the shortcut.
    pub shift: bool,
    /// Command (macOS) or Ctrl (elsewhere) is part of it.
    pub command: bool,
}

/// Where a dragged row goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionPlace {
    /// An action dropped at list index `index` (after its removal), into `set` (`None`: loose).
    Action {
        /// The index in [`ActionList::actions`] after the removal.
        index: usize,
        /// The set it lands in.
        set: Option<String>,
    },
    /// A step dropped at position `index` of action `action`.
    Step {
        /// The action it lands in.
        action: usize,
        /// The position after the removal.
        index: usize,
    },
}

/// What the user did, in order. Each is one app command (PhotoCraft's `actions.*`); edits that
/// need no app logic are applied by the app with the matching [`ActionList`] call.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ActionsEvent {
    /// Play `action` from step `from` (0 for the whole action).
    Play {
        /// The action.
        action: usize,
        /// The first step.
        from: usize,
    },
    /// Start recording into `into`, or into a new action when `None` (New Action…, Create new
    /// action).
    BeginRecording {
        /// The action to record into.
        into: Option<usize>,
    },
    /// Stop recording or playing.
    StopRecording,
    /// New Action… from the panel menu: the app asks for a name, then records.
    NewAction,
    /// New Set… or Create new set: the app asks for a name ([`ActionList::new_set`]).
    NewSet,
    /// Set Options… for this set.
    SetOptions(String),
    /// Delete this row ([`ActionList::delete_action`], [`ActionList::delete_step`]; a set deletes
    /// its actions, the app confirms).
    Delete(ActionSelection),
    /// An in-place rename ended with `name` ([`ActionList::rename`]).
    Rename {
        /// The action.
        action: usize,
        /// The new name, as typed.
        name: String,
    },
    /// A row was dragged ([`ActionList::move_action`], [`ActionList::assign_set`],
    /// [`ActionList::move_step`]).
    Move {
        /// The dragged row.
        source: ActionSelection,
        /// Where it goes.
        to: ActionPlace,
    },
    /// The include cell of a step ([`ActionList::toggle_step`]).
    ToggleStep {
        /// The action.
        action: usize,
        /// The step.
        step: usize,
    },
    /// The include cell of an action row ([`ActionList::toggle_all`]).
    ToggleAll {
        /// The action.
        action: usize,
    },
    /// The dialog cell of a step ([`ActionList::toggle_dialog`]).
    ToggleDialog {
        /// The action.
        action: usize,
        /// The step.
        step: usize,
    },
    /// The context menu's Color ([`ActionList::set_color`]).
    SetColor {
        /// The action.
        action: usize,
        /// The colour.
        color: ActionColor,
    },
    /// The context menu's Keyboard Shortcuts: a key with its modifiers, or `None` to remove it.
    AssignShortcut {
        /// The action.
        action: usize,
        /// The shortcut.
        choice: Option<ShortcutChoice>,
    },
    /// Insert Stop… after the selection (the app's dialog).
    InsertStop,
    /// Batch… with this action (the app's dialog).
    Batch(usize),
    /// Load Actions… (the app's file dialog).
    Load,
    /// Save Actions…, or Save Set… for one set.
    Save {
        /// The set; `None` for every action.
        set: Option<String>,
    },
}

/// What [`ActionsPanel::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
pub struct ActionsPanelOutput {
    /// The panel's area.
    pub response: Response,
    /// What the user did this pass, in order.
    pub events: Vec<ActionsEvent>,
}

/// The Actions panel (Photoshop's Window › Actions): sets, actions and steps as a tree, the
/// include and dialog cells, in-place rename, drag to move, Button Mode, the footer, the panel
/// menu and the context menus. It runs nothing and changes nothing: it reads the
/// [`ActionList`] and reports [`ActionsEvent`]s (lead call L19).
///
/// **What the user sees.** A [`List`](craft_ui::list::List) in tree mode (lead call L20) over the
/// space above the footer: loose actions first, then each set (a header with a folder icon,
/// open or closed, and "{n} actions" at the end side; empty sets show their header), each action
/// (a play glyph, its name, and at the end side its shortcut and step count, "2/3 steps" when some
/// are off; the recording action a `status.bad.solid` dot, expanded, its latest step scrolled
/// into view), and an expanded action's steps (the host's labels, `text.muted`, `text.faint`
/// when off), the pending steps of a recording after them. The include and dialog cells sit at
/// the reading start of action and step rows. Row heights are `actions.set_row_height`,
/// `actions.row_height` and `actions.step_row_height`; an action in a set is indented
/// `actions.indent`. With no actions: an [`EmptyState`](craft_ui::composites::EmptyState) with the
/// Record glyph and [`ActionsLabels::empty`]. Under the list the footer ([`PanelFooter`](craft_ui::composites::PanelFooter)).
///
/// **Pointer.** A click selects a row; a click on the disclosure toggles it (Alt-click opens or
/// closes all); a double-click on an action's name renames it in place (Enter commits, Esc
/// cancels), elsewhere on an action row plays it, on a set toggles it. The include cell toggles a
/// step, or every step of an action (all on unless all already are); the dialog cell toggles the
/// step's dialog flag. A drag moves an action (into or out of a set) or a step (within or between
/// actions); the list auto-scrolls within 22 points of its edge, 8 per pass, and draws the drop
/// line. Right-click on an action opens Keyboard Shortcuts and Color; on a set, Save Set….
///
/// **Button Mode.** Each action is a button across the row in its colour
/// ([`ActionsPanelStyle::fill`](super::ActionsPanelStyle::fill)), set headers stay; a click plays
/// it once (a double-click does not play twice). Button Mode is off while recording.
///
/// **Keyboard and focus.** The list is one Tab stop: arrows move, Left and Right close and open,
/// Enter plays an action row or toggles a set, Space toggles the include cell of a step, F2
/// renames, Delete deletes the selection, Cmd/Ctrl+] and [ move the focused row one place later or
/// earlier; the footer buttons are Tab stops after it. Edits are refused while
/// [`ActionsHost::playing`]; a rename waits until a drag ends.
///
/// **AccessKit.** A `Tree` named [`ActionsLabels::name`]; rows are `TreeItem`s with level,
/// expanded and Expand or Collapse, selected; `Default` on an action plays it; a recording
/// action's name carries [`ActionsLabels::recording_suffix`]. The include and dialog cells are
/// `CheckBox`es (lead call L23) named [`ActionsLabels::include`] and [`ActionsLabels::dialog`],
/// the include cell of a partial action `Toggled::Mixed`.
///
/// **Direction.** The tree, disclosure and cells follow reading order (`List` rules); the counts
/// and shortcut sit at the end side.
#[must_use = "show the panel with `show`"]
#[expect(dead_code, reason = "skeleton: removed when the bodies land")]
pub struct ActionsPanel<'a> {
    pub(super) id_salt: IdSalt,
    pub(super) state: &'a mut ActionsState,
    pub(super) list: &'a ActionList,
    pub(super) host: Option<&'a dyn ActionsHost>,
    pub(super) labels: Option<ActionsLabels>,
    pub(super) enabled: bool,
}

impl<'a> ActionsPanel<'a> {
    /// A panel over `list` with `state`; `id_salt` keeps its scroll, focus, rename and drag.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut ActionsState, list: &'a ActionList) -> Self {
        Self { id_salt: IdSalt::new(id_salt), state, list, host: None, labels: None, enabled: true }
    }

    /// The app's side; without one, steps show their command ids and nothing records or plays.
    pub fn host(mut self, host: &'a dyn ActionsHost) -> Self {
        self.host = Some(host);
        self
    }

    /// The words; the installed words' [`ActionsLabels`] without a call.
    pub fn labels(mut self, labels: ActionsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Whether the panel takes input (default true).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the tree (or the buttons) and the footer, and reports this pass's events in order:
    /// the list's, the cells', the context menu's, the footer's.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn show(self, ui: &mut Ui) -> ActionsPanelOutput {
        // pc: crates/ui-egui/src/panels/actions.rs:906 `panel` (rows, selection, rename, play, drag, auto-scroll :1153);
        // new: on `List` in tree mode (L20), keys and AccessKit from the list.
        todo!()
    }
}

/// Button Mode's body: each action of `list` in display order as a full-width button in its
/// colour, set headers between; a click (not a double-click's second) reports `Play`. Returns
/// the pass's events.
#[expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
fn button_mode(ui: &mut Ui, list: &ActionList, style: &super::style::ActionsPanelStyle) -> Vec<ActionsEvent> {
    // pc: crates/ui-egui/src/panels/actions.rs:991-992 (filled row), :1045-1048 (click plays once, not on a double-click), :297-313 (fills).
    todo!()
}
