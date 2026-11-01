//! [`ActionsPanel`]: the tree on `List`, selection, rename, drag, events, the empty state and
//! Button Mode.

use std::collections::BTreeSet;
use std::sync::Arc;

use craft_ui::behavior::{Axis, DropPosition, Move, Reorder, Selection, SelectionMode};
use craft_ui::composites::{EmptyState, PanelFooter};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow, OpenChange, RowEvent, ToggleEdit};
use craft_ui::localized::SharedLabels as _;
use craft_ui::primitives::Interactive;
use craft_ui::theme::{Tokens, TokensOf as _, role_format};
use egui::{Align2, AsIdSalt, CornerRadius, CursorIcon, Id, IdSalt, Key, Modifiers, Response, Role, ScrollArea, Sense, Ui, Vec2, vec2};

use super::footer::footer;
use super::labels::ActionsLabels;
use super::menu::show_context_menu;
use super::rows::{ActionRows, row};
use super::style::ActionsPanelStyle;
use crate::actions::{Action, ActionColor, ActionList, ActionStep, moved_index};

/// What the panel asks of the app while it draws: words and state the list does not hold.
pub trait ActionsHost {
    /// A step's label in the tree ("Levels", "Play Action “Sharpen”"), from the app's command
    /// catalogue.
    fn step_label(&self, step: &ActionStep) -> String;

    /// The action's keyboard shortcut, if it has one (the app's `actions.play:<name>`
    /// override). None by default.
    fn shortcut(&self, action: &Action) -> Option<ShortcutChoice> {
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
///
/// The panel remaps `selected` itself when it reports a [`Reorder`](ActionsEvent::Reorder)
/// that moves a row, since it owns the selection. After a [`Delete`](ActionsEvent::Delete) the
/// app clears `selected` (the panel cannot tell which rows the delete removed). An open rename
/// field belongs to the list; a drag ends it, so a move never leaves one open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionsPanelState {
    /// The persisted section.
    pub settings: ActionsSettings,
    /// The selected row (one at most).
    pub selected: Option<ActionSelection>,
    /// The expanded actions, by name (names are unique, so expansion survives moves).
    pub expanded: BTreeSet<String>,
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

impl ShortcutChoice {
    /// The shortcut as a row shows it: "F2" and "⇧⌘F3" on macOS, "Ctrl+Shift+F3" elsewhere.
    pub fn text(&self) -> String {
        let (shift, command) = (self.shift, self.command);
        if cfg!(target_os = "macos") {
            format!("{}{}F{}", if shift { "⇧" } else { "" }, if command { "⌘" } else { "" }, self.key)
        } else {
            format!("{}{}F{}", if command { "Ctrl+" } else { "" }, if shift { "Shift+" } else { "" }, self.key)
        }
    }
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

impl ActionPlace {
    /// Where dropping `mv` lands in `list`, or `None` when the drop is not allowed (more than one
    /// row, a copy, or a place the tree does not take). An action goes before or after another
    /// action (into its set), into a set (as its first), before the first set (loose) or before
    /// a later set (at the end of the set before it, where the drop line is drawn); a step goes
    /// before or after a step of any action or into an action as its first. The app applies a
    /// [`Reorder`](ActionsEvent::Reorder) with this: [`ActionList::move_action`] and
    /// [`ActionList::assign_set`] for an action, [`ActionList::move_step`] for a step.
    pub fn of(list: &ActionList, mv: &Move<ActionSelection>) -> Option<ActionPlace> {
        if mv.copy || mv.items.len() != 1 {
            return None;
        }
        place(list, &mv.items[0], &mv.target, mv.position)
    }
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
    /// An in-place rename ended with `text` ([`ActionList::rename`]).
    Rename {
        /// The action.
        action: usize,
        /// The new name, as typed.
        text: String,
    },
    /// A row was dragged, as the list reports the gesture: apply the `Commit` or `Set` with
    /// [`ActionPlace::of`] ([`ActionList::move_action`], [`ActionList::assign_set`],
    /// [`ActionList::move_step`]) and restore on `Cancel`.
    Reorder(EditEvent<Move<ActionSelection>>),
    /// A press or sweep on the include or dialog column, as the list reports the gesture (one
    /// undo step per sweep; Esc cancels it). Each `Update`, `Commit` and `Set` names a cell
    /// and the state it takes: column 0 of a step row is [`ActionList::set_step`], column 0 of an
    /// action row [`ActionList::set_all`], column 1 of a step row [`ActionList::set_dialog`].
    Toggle(EditEvent<ToggleEdit<ActionSelection>>),
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
#[must_use = "handle the events and save the settings when `settings_changed`"]
#[non_exhaustive]
pub struct ActionsPanelOutput {
    /// The panel's area.
    pub response: Response,
    /// What the user did this pass, in order.
    pub events: Vec<ActionsEvent>,
    /// The persisted [`ActionsSettings`] changed this pass (Button Mode, a set opened or
    /// closed): the app saves them.
    pub settings_changed: bool,
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
/// the reading start of action and step rows: the list's toggle columns (the include cell a check
/// box, the dialog cell the `Icon::Note` column). Row heights are `actions.set_row_height`,
/// `actions.row_height` and `actions.step_row_height`; an action in a set sits one tree
/// level in, as the list indents it. With no actions: an [`EmptyState`](craft_ui::composites::EmptyState) with the
/// Record glyph and [`ActionsLabels::empty`]. Under the list the footer ([`PanelFooter`](craft_ui::composites::PanelFooter)).
///
/// **Pointer.** A click selects a row; a click on the disclosure toggles it (Alt-click opens or
/// closes all); a double-click on an action's name renames it in place (Enter commits, Esc
/// cancels), elsewhere on an action row plays it, on a set toggles it. The include cell toggles a
/// step, or every step of an action (all on unless all already are); the dialog cell toggles the
/// step's dialog flag. A press on a cell sweeps down the column as on the Layers eye, reported as one
/// [`Toggle`](ActionsEvent::Toggle) gesture (one undo step; Esc restores the rows). An action
/// with no steps has no include cell. A drag moves an action (into or out of a set) or a step (within or between
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
/// the rows' toggle columns as `CheckBox`es (lead call L23; [`ToggleColumn::check`](craft_ui::list::ToggleColumn::check))
/// named [`ActionsLabels::include`] and [`ActionsLabels::dialog`], the include cell of a partial
/// action `Toggled::Mixed` ([`ToggleColumn::mixed`](craft_ui::list::ToggleColumn::mixed)).
///
/// **Direction.** The tree, disclosure and cells follow reading order (`List` rules); the counts
/// and shortcut sit at the end side.
#[must_use = "show the panel with `show`"]
pub struct ActionsPanel<'a> {
    pub(super) id_salt: IdSalt,
    pub(super) state: &'a mut ActionsPanelState,
    pub(super) list: &'a ActionList,
    pub(super) host: Option<&'a dyn ActionsHost>,
    pub(super) labels: Option<ActionsLabels>,
    pub(super) enabled: bool,
    pub(super) rename: Option<usize>,
}

impl<'a> ActionsPanel<'a> {
    /// A panel over `list` with `state`; `id_salt` keeps its scroll, focus, rename and drag.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut ActionsPanelState, list: &'a ActionList) -> Self {
        Self { id_salt: IdSalt::new(id_salt), state, list, host: None, labels: None, enabled: true, rename: None }
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

    /// Opens the in-place rename of `action`, committing any other open one (the app's Rename
    /// command). Call it in the pass that handles the command.
    pub fn start_rename(mut self, action: usize) -> Self {
        self.rename = Some(action);
        self
    }

    /// Whether the panel takes input (default true).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the tree (or the buttons) and the footer, and reports this pass's events in order:
    /// the list's, the cells', the context menu's, the footer's.
    pub fn show(self, ui: &mut Ui) -> ActionsPanelOutput {
        // pc: crates/ui-egui/src/panels/actions.rs:906 `panel` (rows, selection, rename, play, drag, auto-scroll :1153);
        // new: on `List` in tree mode (L20), keys and AccessKit from the list.
        let Self { id_salt, state, list, host, labels, enabled, rename } = self;
        let tokens = Tokens::of(ui);
        let labels = labels.map_or_else(|| ActionsLabels::shared(ui), Arc::new);
        let style = ActionsPanel::style(&tokens);
        let id = Id::NULL.with(id_salt);
        let footer_height = PanelFooter::reserved_height(&PanelFooter::style(&tokens, ui.pixels_per_point()));
        let height = (ui.available_height() - footer_height).max(0.0);
        let settings_before = state.settings.clone();
        let inner = ui.scope(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.add_enabled_ui(enabled, |ui| {
                let mut events = show_body(ui, id, state, list, host, &labels, &style, height, rename);
                events.extend(footer(ui, list_id(ui, id), state, list, host, &labels));
                events
            })
            .inner
        });
        let settings_changed = state.settings != settings_before;
        ActionsPanelOutput { response: inner.response, events: inner.inner, settings_changed }
    }
}

/// The id the tree's list shows under, which its rows' drags carry as `ListDrag::list`.
pub(super) fn list_id(ui: &Ui, id: Id) -> Id {
    List::id_in(ui, (id, "list"))
}

/// The part of the panel above the footer, `height` tall: the empty state, Button Mode's buttons or
/// the tree. Returns this pass's events.
#[expect(clippy::too_many_arguments, reason = "the panel's pieces; a struct would only carry them")]
pub(super) fn show_body(
    ui: &mut Ui,
    id: Id,
    state: &mut ActionsPanelState,
    list: &ActionList,
    host: Option<&dyn ActionsHost>,
    labels: &ActionsLabels,
    style: &ActionsPanelStyle,
    height: f32,
    rename: Option<usize>,
) -> Vec<ActionsEvent> {
    if list.actions().is_empty() && list.sets().is_empty() {
        let width = ui.available_width();
        ui.allocate_ui(vec2(width, height), |ui| {
            EmptyState::centred(labels.empty.clone()).icon(Icon::Circle).show(ui);
        });
        return Vec::new();
    }
    let recording = host.and_then(|host| host.recording());
    if state.settings.button_mode && recording.is_none() {
        let scroll = ScrollArea::vertical().id_salt((id, "buttons")).max_height(height).auto_shrink([false, false]);
        return scroll.show(ui, |ui| button_mode(ui, list, style)).inner;
    }
    show_tree(ui, id, state, list, host, labels, style, height, rename)
}

/// The tree on a [`List`] in tree mode, and the events of what happened on it.
#[expect(clippy::too_many_arguments, reason = "the panel's pieces; a struct would only carry them")]
fn show_tree(
    ui: &mut Ui,
    id: Id,
    state: &mut ActionsPanelState,
    list: &ActionList,
    host: Option<&dyn ActionsHost>,
    labels: &ActionsLabels,
    style: &ActionsPanelStyle,
    height: f32,
    rename: Option<usize>,
) -> Vec<ActionsEvent> {
    let editable = !host.is_some_and(|host| host.playing());
    let rows = ActionRows::build(list, state, host);
    let mut selection = Selection { items: state.selected.iter().cloned().collect(), lead: state.selected.clone(), anchor: state.selected.clone() };
    let mut widget = List::new((id, "list"), labels.name.clone(), &rows)
        .tree(true)
        .selection(&mut selection)
        .mode(SelectionMode::Single)
        .heights(|at| rows.height(at, style))
        .max_height(height)
        .can_rename(|key| editable && matches!(key, ActionSelection::Action(_)));
    if editable {
        widget = widget.reorder(Reorder::new(Axis::Vertical).accepts(|mv, _| accepts_move(list, mv)));
        if let Some(index) = rename {
            widget = widget.start_rename(ActionSelection::Action(index));
        }
    }
    if let Some(key) = reveal_key(ui, id, list, host, &rows) {
        widget = widget.reveal(key);
    }
    let ctx = ui.ctx().clone();
    let shown: &ActionsPanelState = state;
    let out = widget.show(ui, |_, key| row(&ctx, key, list, shown, host, labels, style));
    let focused = out.response.has_focus();
    let mut events = Vec::new();
    let mut request = None;
    let mut moves = Vec::new();
    let open_before = (state.expanded.len(), state.settings.collapsed_sets.len());
    for event in &out.events {
        match event {
            ListEvent::Open(change) => apply_open(state, list, change),
            ListEvent::Activate(key) => match key {
                ActionSelection::Action(action) if host.is_some() => events.push(ActionsEvent::Play { action: *action, from: 0 }),
                ActionSelection::Set(name) => {
                    apply_open(state, list, &OpenChange { row: key.clone(), open: state.settings.collapsed_sets.contains(name), all: false })
                }
                _ => {}
            },
            ListEvent::Row(RowEvent::Toggle(edit)) if editable => events.push(ActionsEvent::Toggle(edit.clone())),
            ListEvent::Row(RowEvent::Renamed { row: ActionSelection::Action(action), text }) if editable => {
                if !text.trim().is_empty() && list.action(*action).is_some_and(|current| current.name != text.trim()) {
                    events.push(ActionsEvent::Rename { action: *action, text: text.clone() });
                }
            }
            ListEvent::Reorder(edit) if editable => {
                if let EditEvent::Commit { value, changed: true, .. } | EditEvent::Set { value, .. } = edit
                    && let Some(place) = ActionPlace::of(list, value)
                {
                    moves.push((value.items[0].clone(), place));
                }
                events.push(ActionsEvent::Reorder(edit.clone()));
            }
            ListEvent::ContextMenu(asked) => request = Some(asked.clone()),
            _ => {}
        }
    }
    if open_before != (state.expanded.len(), state.settings.collapsed_sets.len()) {
        ui.ctx().request_repaint();
    }
    state.selected = selection.items.first().cloned();
    for (source, to) in &moves {
        state.selected = state.selected.take().map(|selected| remapped(selected, source, to));
    }
    if focused && editable && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Delete)) {
        events.extend(state.selected.as_ref().and_then(|selected| delete_event(selected, host.is_some_and(|host| host.recording().is_some()))));
    }
    events.extend(show_context_menu(ui, id.with("context-menu"), request, list, host, labels));
    events
}

/// The `Delete` event for `selected`: a step always; a set or an action not while recording.
pub(super) fn delete_event(selected: &ActionSelection, recording: bool) -> Option<ActionsEvent> {
    (!recording || matches!(selected, ActionSelection::Step(..))).then(|| ActionsEvent::Delete(selected.clone()))
}

/// Opens or closes what `change` names in `state`: an action's steps, a set's actions, or with
/// `all` every one of them.
fn apply_open(state: &mut ActionsPanelState, list: &ActionList, change: &OpenChange<ActionSelection>) {
    if change.all {
        if change.open {
            state.expanded.extend(list.actions().iter().map(|action| action.name.clone()));
            state.settings.collapsed_sets.clear();
        } else {
            state.expanded.clear();
            state.settings.collapsed_sets = list.sets().into_iter().map(str::to_owned).collect();
        }
        return;
    }
    match &change.row {
        ActionSelection::Action(index) => {
            if let Some(action) = list.action(*index) {
                if change.open {
                    state.expanded.insert(action.name.clone());
                } else {
                    state.expanded.remove(&action.name);
                }
            }
        }
        ActionSelection::Set(name) => {
            state.settings.collapsed_sets.retain(|set| set != name);
            if !change.open {
                state.settings.collapsed_sets.push(name.clone());
            }
        }
        ActionSelection::Step(..) => {}
    }
}

/// The row to scroll into view while recording: the latest step, whenever the recording action
/// or its step count differs from the last pass; `None` otherwise.
fn reveal_key(ui: &Ui, id: Id, list: &ActionList, host: Option<&dyn ActionsHost>, rows: &ActionRows) -> Option<ActionSelection> {
    let slot = id.with("revealed");
    let Some(recording) = host.and_then(|host| host.recording()) else {
        ui.data_mut(|data| data.remove::<(usize, usize)>(slot));
        return None;
    };
    let total = list.step_count(recording) + host.map_or(0, |host| host.pending_steps(recording).len());
    if ui.data(|data| data.get_temp::<(usize, usize)>(slot)) == Some((recording, total)) {
        return None;
    }
    ui.data_mut(|data| data.insert_temp(slot, (recording, total)));
    let key = if total > 0 { ActionSelection::Step(recording, total - 1) } else { ActionSelection::Action(recording) };
    rows.rows.contains(&key).then_some(key)
}

/// Where dropping `source` at `position` of `target` lands in the list, or `None` when the drop
/// is not allowed: an action goes before or after another action (into its set), into a set (as
/// its first), before the first set (loose) or before a later set (at the end of the set before
/// it, where the drop line is drawn); a step goes before or after a step of any action or into
/// an action as its first.
fn place(list: &ActionList, source: &ActionSelection, target: &ActionSelection, position: DropPosition) -> Option<ActionPlace> {
    // pc: crates/ui-egui/src/panels/actions.rs:1153 (drag), `actions.assignSet`, `actions.moveStep`.
    let len = list.actions().len();
    match (source, target) {
        (ActionSelection::Action(from), ActionSelection::Action(over)) if from != over && position != DropPosition::Into => {
            let at = over + usize::from(position == DropPosition::After);
            Some(ActionPlace::Action { index: at - usize::from(*from < at), set: list.set_of(*over).map(str::to_owned) })
        }
        (ActionSelection::Action(from), ActionSelection::Set(name)) => {
            let sets = list.sets();
            let previous_set = sets.iter().position(|set| set == name).and_then(|at| at.checked_sub(1)).map(|at| sets[at]);
            let at = match position {
                DropPosition::Into => list.actions().iter().position(|action| action.set.as_deref() == Some(name.as_str())).unwrap_or(len),
                DropPosition::Before => match previous_set {
                    Some(previous) => list.actions().iter().rposition(|action| action.set.as_deref() == Some(previous)).map_or(len, |last| last + 1),
                    None => list.actions().iter().rposition(|action| action.set.is_none()).map_or(0, |last| last + 1),
                },
                DropPosition::After => return None,
            };
            let set = match position {
                DropPosition::Into => Some(name.clone()),
                _ => previous_set.map(str::to_owned),
            };
            Some(ActionPlace::Action { index: at - usize::from(*from < at), set })
        }
        (ActionSelection::Step(from, step), ActionSelection::Step(over, at))
            if position != DropPosition::Into && *step < list.step_count(*from) && *at < list.step_count(*over) =>
        {
            let to = at + usize::from(position == DropPosition::After);
            Some(ActionPlace::Step { action: *over, index: to - usize::from(from == over && *step < to) })
        }
        (ActionSelection::Step(from, step), ActionSelection::Action(over)) if position == DropPosition::Into && *step < list.step_count(*from) => {
            Some(ActionPlace::Step { action: *over, index: 0 })
        }
        _ => None,
    }
}

/// Whether the list takes the move `mv`: one row, not a copy, to a place [`ActionPlace::of`] knows.
fn accepts_move(list: &ActionList, mv: &Move<ActionSelection>) -> bool {
    ActionPlace::of(list, mv).is_some()
}

/// Where `selected` is after `source` moved to `to`: the moved row and the rows it passed follow
/// the list's own shifts.
fn remapped(selected: ActionSelection, source: &ActionSelection, to: &ActionPlace) -> ActionSelection {
    match (source, to) {
        (ActionSelection::Action(from), ActionPlace::Action { index, .. }) => match selected {
            ActionSelection::Action(at) => ActionSelection::Action(moved_index(at, *from, *index)),
            ActionSelection::Step(at, step) => ActionSelection::Step(moved_index(at, *from, *index), step),
            set @ ActionSelection::Set(_) => set,
        },
        (ActionSelection::Step(from, moved), ActionPlace::Step { action: into, index }) => match selected {
            ActionSelection::Step(at, step) if at == *from && step == *moved => ActionSelection::Step(*into, *index),
            ActionSelection::Step(at, step) if at == *from && from == into => ActionSelection::Step(at, moved_index(step, *moved, *index)),
            ActionSelection::Step(at, step) if at == *from && step > *moved => ActionSelection::Step(at, step - 1),
            ActionSelection::Step(at, step) if at == *into && step >= *index => ActionSelection::Step(at, step + 1),
            other => other,
        },
        _ => selected,
    }
}

/// Button Mode's body: each action of `list` in display order as a full-width button in its
/// colour, set headers between; a click (not a double-click's second) reports `Play`. Returns
/// the pass's events.
fn button_mode(ui: &mut Ui, list: &ActionList, style: &ActionsPanelStyle) -> Vec<ActionsEvent> {
    // pc: crates/ui-egui/src/panels/actions.rs:991-992 (filled row), :1045-1048 (click plays once, not on a double-click), :297-313 (fills).
    let tokens = craft_ui::theme::current(ui.ctx());
    let row_style = ListRow::<ActionSelection>::style(&tokens);
    let gap = tokens.space.gap.stack;
    let rtl = craft_ui::direction::is_rtl(ui);
    let radius = CornerRadius::same(row_style.radius.round().clamp(0.0, 255.0) as u8);
    let mut events = Vec::new();
    let order = list.display_order();
    let mut shown: Vec<(Option<&str>, Vec<usize>)> = vec![(None, order.iter().copied().filter(|&index| list.set_of(index).is_none()).collect())];
    for set in list.sets() {
        shown.push((Some(set), order.iter().copied().filter(|&index| list.set_of(index) == Some(set)).collect()));
    }
    for (set, actions) in shown {
        if let Some(name) = set {
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), style.set_row_height), Sense::hover());
            let font = role_format(&tokens, row_style.label_role, style.label).font_id;
            let (anchor, x) =
                if rtl { (Align2::RIGHT_CENTER, rect.right() - row_style.pad_x) } else { (Align2::LEFT_CENTER, rect.left() + row_style.pad_x) };
            ui.painter().text(egui::pos2(x, rect.center().y), anchor, name, font, style.label);
        }
        for index in actions {
            let Some(action) = list.action(index) else { continue };
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), style.row_height), Sense::hover());
            let sensed = Interactive::with_id(ui.scope_id().with(("button-mode", index)), Role::Button, action.name.clone())
                .sense(Sense::click())
                .state_layer(false)
                .interact_at(ui, rect);
            let response = sensed.finish(ui).on_hover_cursor(CursorIcon::PointingHand);
            let color = style.label_color(action.color);
            ui.painter().rect_filled(rect, radius, style.fill(action.color));
            let font = role_format(&tokens, row_style.label_role, color).font_id;
            let (anchor, x) =
                if rtl { (Align2::RIGHT_CENTER, rect.right() - row_style.pad_x) } else { (Align2::LEFT_CENTER, rect.left() + row_style.pad_x) };
            ui.painter().text(egui::pos2(x, rect.center().y), anchor, &action.name, font, color);
            if response.clicked() && !response.double_clicked() && !response.triple_clicked() {
                events.push(ActionsEvent::Play { action: index, from: 0 });
            }
            ui.add_space(gap);
        }
    }
    events
}
