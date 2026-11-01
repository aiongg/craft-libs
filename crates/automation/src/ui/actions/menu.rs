//! The panel menu and the context menus (Keyboard Shortcuts, Color; Save Set…).

use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::list::ContextMenuRequest;
use craft_ui::localized::SharedLabels as _;
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use egui::{AsIdSalt, Context, Id, IdSalt, Sense, Ui};

use super::footer::{play_target, selected_action};
use super::labels::ActionsLabels;
use super::panel::{ActionSelection, ActionsEvent, ActionsHost, ActionsPanel, ActionsState, ShortcutChoice, delete_event};
use crate::actions::{ActionColor, ActionList};

const NEW_ACTION: &str = "actions.newAction";
const NEW_SET: &str = "actions.newSet";
const SET_OPTIONS: &str = "actions.setOptions";
const PLAY: &str = "actions.play";
const RECORD: &str = "actions.record";
const STOP: &str = "actions.stop";
const DELETE: &str = "actions.delete";
const INSERT_STOP: &str = "actions.insertStop";
const BATCH: &str = "actions.batch";
const LOAD: &str = "actions.load";
const SAVE: &str = "actions.save";
const BUTTON_MODE: &str = "actions.buttonMode";

/// Whether the panel is recording, kept by `panel_menu` for `run_command` (which is not told).
fn recording_slot(id_salt: impl AsIdSalt) -> Id {
    Id::NULL.with(IdSalt::new(id_salt)).with("menu-recording")
}

/// The set the selection is in: the selected set, or the selected action's (or step's) set.
fn current_set(state: &ActionsState, list: &ActionList) -> Option<String> {
    match state.selected.as_ref()? {
        ActionSelection::Set(name) => Some(name.clone()),
        ActionSelection::Action(index) | ActionSelection::Step(index, _) => list.set_of(*index).map(str::to_owned),
    }
}

impl ActionsPanel<'_> {
    /// The panel menu's rows, for the docking host's `panel_menu_entries`: New Action… (not while
    /// recording), New Set…, Set Options… (needs a current set: the selected set, or the selected
    /// action's), a separator, Play (needs an action), Start Recording or Stop Recording, Delete
    /// (as the footer's), a separator, Insert Stop… (needs a selected action or step), Batch…
    /// (needs an action), a separator, Load Actions…, Save Actions… (needs an action), a separator,
    /// Button Mode (a check row, the footer button's state). Run the chosen row with
    /// [`run_command`](Self::run_command).
    pub fn panel_menu(
        ctx: &Context,
        id_salt: impl AsIdSalt,
        state: &ActionsState,
        list: &ActionList,
        recording: bool,
    ) -> Vec<MenuEntry<PanelCommand>> {
        // pc: crates/ui-egui/src/panels/actions.rs:624 `panel_menu`; vectorcraft: crates/ui-egui/src/panels/actions.rs:55-80 (New Set…, Set Options…, Batch…).
        let labels = ActionsLabels::shared_ctx(ctx);
        ctx.data_mut(|data| data.insert_temp(recording_slot(id_salt), recording));
        let any = !list.actions().is_empty();
        let can_delete = state.selected.as_ref().is_some_and(|selected| delete_event(selected, recording).is_some());
        let on_row = matches!(state.selected, Some(ActionSelection::Action(_) | ActionSelection::Step(..)));
        let item = |label: &str, command: &str, enabled: bool| MenuItem::new(label, command.to_owned()).enabled(enabled).into();
        let mut entries: Vec<MenuEntry<PanelCommand>> = vec![
            item(&labels.menu_new_action, NEW_ACTION, !recording),
            item(&labels.menu_new_set, NEW_SET, !recording),
            item(&labels.set_options, SET_OPTIONS, current_set(state, list).is_some()),
            MenuEntry::Separator,
            item(&labels.menu_play, PLAY, any && !recording),
        ];
        entries.push(if recording { item(&labels.stop_recording, STOP, true) } else { item(&labels.start_recording, RECORD, true) });
        entries.extend([
            item(&labels.menu_delete, DELETE, can_delete),
            MenuEntry::Separator,
            item(&labels.insert_stop, INSERT_STOP, on_row),
            item(&labels.batch, BATCH, any),
            MenuEntry::Separator,
            item(&labels.load, LOAD, true),
            item(&labels.save, SAVE, any),
            MenuEntry::Separator,
            MenuItem::new(labels.button_mode.clone(), BUTTON_MODE.to_owned()).check(state.settings.button_mode).enabled(!recording).into(),
        ]);
        entries
    }

    /// Runs a chosen panel-menu row: Button Mode toggles it in `state` and returns `None`; every
    /// other row returns its event (New Action… is [`ActionsEvent::NewAction`]; Play plays the
    /// selection as the footer's Play does). `None` too when the command is not the panel's.
    pub fn run_command(
        ctx: &Context,
        id_salt: impl AsIdSalt,
        state: &mut ActionsState,
        command: &PanelCommand,
        list: &ActionList,
    ) -> Option<ActionsEvent> {
        // new: as craft-color crates/color/src/ui/panel.rs:318 `ColorPanel::run_command`.
        let recording = ctx.data(|data| data.get_temp::<bool>(recording_slot(id_salt))).unwrap_or(false);
        let selected = state.selected.as_ref();
        match command.as_str() {
            NEW_ACTION => Some(ActionsEvent::NewAction),
            NEW_SET => Some(ActionsEvent::NewSet),
            SET_OPTIONS => current_set(state, list).map(ActionsEvent::SetOptions),
            PLAY => play_target(selected, list).map(|(action, from)| ActionsEvent::Play { action, from }),
            RECORD => Some(ActionsEvent::BeginRecording { into: selected_action(selected, list) }),
            STOP => Some(ActionsEvent::StopRecording),
            DELETE => selected.and_then(|selected| delete_event(selected, recording)),
            INSERT_STOP => selected_action(selected, list).map(|_| ActionsEvent::InsertStop),
            BATCH => play_target(selected, list).map(|(action, _)| ActionsEvent::Batch(action)),
            LOAD => Some(ActionsEvent::Load),
            SAVE => Some(ActionsEvent::Save { set: None }),
            BUTTON_MODE => {
                state.settings.button_mode = !state.settings.button_mode;
                None
            }
            _ => None,
        }
    }
}

/// The context menu the list asked for (`ListEvent::ContextMenu`), shown with
/// [`ContextMenu`](craft_ui::menu::ContextMenu) at the request's pointer or anchor:
///
/// - On an action: Keyboard Shortcuts (a caption), None and F1 to F12 as radio rows (the current
///   key marked), then Shift and Command (Ctrl off macOS) check rows, enabled only with a key;
///   check rows keep the menu open so both can be ticked. A separator, then Color: a submenu of
///   None, Red, Orange, Yellow, Green, Blue, Violet, Gray (the current one marked). A row reports
///   [`ActionsEvent::AssignShortcut`] or [`ActionsEvent::SetColor`].
/// - On a set: Save Set… ([`ActionsEvent::Save`] with the set).
/// - On a step or nothing: no menu.
///
/// The current shortcut is parsed from [`ActionsHost::shortcut`] by the host's rule; the panel
/// keeps the menu's ticks in egui memory under the menu's id while it is open.
pub(super) fn context_menu(
    ui: &mut Ui,
    request: &ContextMenuRequest<ActionSelection>,
    list: &ActionList,
    host: Option<&dyn ActionsHost>,
    labels: &ActionsLabels,
) -> Option<ActionsEvent> {
    // pc: crates/ui-egui/src/panels/actions.rs:864 `action_context_menu`, :830 `draw_set_header` (Save Set…).
    let (entries, action) = match request.row.as_ref()? {
        ActionSelection::Set(name) => (vec![MenuItem::new(labels.save_set.clone(), MenuCommand::Save(name.clone())).into()], None),
        ActionSelection::Action(index) => {
            let current = list.action(*index)?.color;
            (Vec::new(), Some((*index, current)))
        }
        ActionSelection::Step(..) => return None,
    };
    let ctx = ui.ctx().clone();
    let memory = ui.scope_id().with("context-menu");
    let pass = ctx.cumulative_pass_nr();
    let mut pending = match ctx.data(|data| data.get_temp::<Pending>(memory)) {
        Some(kept) if kept.request == *request => kept,
        _ => Pending::opened(request, action.and_then(|(index, _)| list.action(index)).and_then(|a| host?.shortcut(a)).as_deref(), pass),
    };
    if pending.closed {
        // The pass after the menu closed, with another popup still open: not a new request.
        let stale = pass <= pending.pass + 1;
        pending.pass = pass;
        ctx.data_mut(|data| data.insert_temp(memory, pending.clone()));
        if stale {
            return None;
        }
        pending = Pending::opened(request, None, pass);
    }
    let entries = match action {
        Some((_, color)) => action_entries(labels, &pending, color),
        None => entries,
    };
    let widget = ui.interact(request.anchor, memory.with("anchor"), Sense::hover());
    let at = pending.fresh.then(|| request.pointer.map_or(OpenAt::Rect(request.anchor), OpenAt::Pointer));
    pending.fresh = false;
    let shown = ContextMenu::new(&entries).own_gestures(false).open_at(at).show(ui, &widget);
    pending.seen_open |= shown.open;
    pending.pass = pass;
    let mut event = None;
    match (shown.chosen, action) {
        (Some(MenuCommand::Save(set)), _) => event = Some(ActionsEvent::Save { set: Some(set) }),
        (Some(MenuCommand::Color(color)), Some((index, _))) => event = Some(ActionsEvent::SetColor { action: index, color }),
        (Some(MenuCommand::Key(key)), Some((index, _))) => {
            pending.key = key;
            pending.dirty = false;
            event = Some(ActionsEvent::AssignShortcut { action: index, choice: pending.choice() });
        }
        (Some(MenuCommand::Shift), _) => {
            pending.shift = !pending.shift;
            pending.dirty = true;
        }
        (Some(MenuCommand::Command), _) => {
            pending.command = !pending.command;
            pending.dirty = true;
        }
        _ => {}
    }
    if !shown.open && pending.seen_open {
        // Closed without a row of its own: the ticks stand (PhotoCraft applies them as they are made).
        if event.is_none()
            && pending.dirty
            && let Some((index, _)) = action
        {
            event = Some(ActionsEvent::AssignShortcut { action: index, choice: pending.choice() });
        }
        pending.closed = true;
    }
    ctx.data_mut(|data| data.insert_temp(memory, pending));
    event
}

/// What a context-menu row does.
#[derive(Clone, Debug, PartialEq)]
enum MenuCommand {
    /// The caption.
    Caption,
    /// A function key: 0 for None, 1..=12.
    Key(u8),
    Shift,
    Command,
    Color(ActionColor),
    Save(String),
}

/// What an open action menu keeps in egui memory: its ticks, until it closes.
#[derive(Clone, Debug)]
struct Pending {
    request: ContextMenuRequest<ActionSelection>,
    /// 0 for none, else 1..=12.
    key: u8,
    shift: bool,
    command: bool,
    /// A check row changed since the key was last chosen.
    dirty: bool,
    /// Not opened yet: the next draw opens it.
    fresh: bool,
    seen_open: bool,
    closed: bool,
    /// The last pass that asked for this menu.
    pass: u64,
}

impl Pending {
    /// The menu for `request`, ticks from the action's `shortcut` text.
    fn opened(request: &ContextMenuRequest<ActionSelection>, shortcut: Option<&str>, pass: u64) -> Self {
        let current = shortcut.and_then(parse_shortcut);
        Self {
            request: request.clone(),
            key: current.map_or(0, |choice| choice.key),
            shift: current.is_some_and(|choice| choice.shift),
            command: current.is_some_and(|choice| choice.command),
            dirty: false,
            fresh: true,
            seen_open: false,
            closed: false,
            pass,
        }
    }

    fn choice(&self) -> Option<ShortcutChoice> {
        (self.key > 0).then_some(ShortcutChoice { key: self.key, shift: self.shift, command: self.command })
    }
}

/// The rows of an action's menu: Keyboard Shortcuts, then Color.
fn action_entries(labels: &ActionsLabels, pending: &Pending, color: ActionColor) -> Vec<MenuEntry<MenuCommand>> {
    let mut entries: Vec<MenuEntry<MenuCommand>> = vec![MenuItem::new(labels.keyboard_shortcuts.clone(), MenuCommand::Caption).enabled(false).into()];
    entries.push(MenuItem::new(labels.none.clone(), MenuCommand::Key(0)).radio(pending.key == 0).into());
    for key in 1..=12u8 {
        entries.push(MenuItem::new(format!("F{key}"), MenuCommand::Key(key)).radio(pending.key == key).into());
    }
    let modifier = if cfg!(target_os = "macos") { &labels.command } else { &labels.ctrl };
    let has_key = pending.key > 0;
    entries.push(MenuItem::new(labels.shift.clone(), MenuCommand::Shift).check(pending.shift).keep_open(true).enabled(has_key).into());
    entries.push(MenuItem::new(modifier.clone(), MenuCommand::Command).check(pending.command).keep_open(true).enabled(has_key).into());
    entries.push(MenuEntry::Separator);
    let colors = [
        (ActionColor::None, &labels.none),
        (ActionColor::Red, &labels.red),
        (ActionColor::Orange, &labels.orange),
        (ActionColor::Yellow, &labels.yellow),
        (ActionColor::Green, &labels.green),
        (ActionColor::Blue, &labels.blue),
        (ActionColor::Violet, &labels.violet),
        (ActionColor::Gray, &labels.gray),
    ];
    let children =
        colors.into_iter().map(|(each, label)| MenuItem::new(label.clone(), MenuCommand::Color(each)).radio(each == color).into()).collect();
    entries.push(MenuItem::submenu(labels.color.clone(), children).into());
    entries
}

/// The function key and modifiers of the host's shortcut text: `F2`, `Shift+F2`, `Cmd+Shift+F2`,
/// `Ctrl+F2`, or with the glyphs `⇧`, `⌘`, `⌃`. `None` for any other text or a key beyond F12.
fn parse_shortcut(text: &str) -> Option<ShortcutChoice> {
    let mut flags = [false; 2];
    let mut rest = text.trim();
    'strip: loop {
        for (prefix, flag) in [("Shift+", 0), ("⇧", 0), ("Cmd+", 1), ("Ctrl+", 1), ("⌘", 1), ("⌃", 1)] {
            if let Some(after) = rest.strip_prefix(prefix) {
                flags[flag] = true;
                rest = after;
                continue 'strip;
            }
        }
        break;
    }
    let [shift, command] = flags;
    let key = rest.strip_prefix('F')?.parse::<u8>().ok().filter(|key| (1..=12).contains(key))?;
    Some(ShortcutChoice { key, shift, command })
}
