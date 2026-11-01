//! The panel menu and the context menus (Keyboard Shortcuts, Color; Save Set…).

use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::list::ContextMenuRequest;
use craft_ui::localized::{LabelColorLabels, SharedLabels as _};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use egui::{Context, Id, Popup, Sense, Ui};

use super::footer::{play_target, selected_action};
use super::labels::ActionsLabels;
use super::panel::{ActionSelection, ActionsEvent, ActionsHost, ActionsPanel, ActionsPanelState, ShortcutChoice, delete_event};
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

/// What a panel-menu command asks of the app, from [`ActionsPanel::run_command`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ActionsMenuAction {
    /// The row is one of the panel's events.
    Event(ActionsEvent),
    /// Button Mode toggled in the state: persist the [`ActionsSettings`](super::ActionsSettings).
    SettingsChanged,
}

/// The set the selection is in: the selected set, or the selected action's (or step's) set.
fn current_set(state: &ActionsPanelState, list: &ActionList) -> Option<String> {
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
    pub fn panel_menu(ctx: &Context, state: &ActionsPanelState, list: &ActionList, recording: bool) -> Vec<MenuEntry<PanelCommand>> {
        // pc: crates/ui-egui/src/panels/actions.rs:624 `panel_menu`; vectorcraft: crates/ui-egui/src/panels/actions.rs:55-80 (New Set…, Set Options…, Batch…).
        let labels = ActionsLabels::shared_ctx(ctx);
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

    /// Runs a chosen panel-menu row: Button Mode toggles it in `state` and returns
    /// [`ActionsMenuAction::SettingsChanged`]; every other row returns its event (New Action… is
    /// [`ActionsEvent::NewAction`]; Play plays the selection as the footer's Play does; Delete is
    /// refused during a recording as the footer's is). `None` when the command is not the
    /// panel's. `recording` is what [`panel_menu`](Self::panel_menu) was given.
    pub fn run_command(state: &mut ActionsPanelState, command: &PanelCommand, list: &ActionList, recording: bool) -> Option<ActionsMenuAction> {
        // new: as craft-color crates/color/src/ui/panel.rs:318 `ColorPanel::run_command`.
        let selected = state.selected.as_ref();
        let event = match command.as_str() {
            NEW_ACTION => ActionsEvent::NewAction,
            NEW_SET => ActionsEvent::NewSet,
            SET_OPTIONS => ActionsEvent::SetOptions(current_set(state, list)?),
            PLAY => {
                let (action, from) = play_target(selected, list)?;
                ActionsEvent::Play { action, from }
            }
            RECORD => ActionsEvent::BeginRecording { into: selected_action(selected, list) },
            STOP => ActionsEvent::StopRecording,
            DELETE => delete_event(selected?, recording)?,
            INSERT_STOP => selected_action(selected, list).map(|_| ActionsEvent::InsertStop)?,
            BATCH => ActionsEvent::Batch(play_target(selected, list)?.0),
            LOAD => ActionsEvent::Load,
            SAVE => ActionsEvent::Save { set: None },
            BUTTON_MODE => {
                state.settings.button_mode = !state.settings.button_mode;
                return Some(ActionsMenuAction::SettingsChanged);
            }
            _ => return None,
        };
        Some(ActionsMenuAction::Event(event))
    }
}

/// Shows the context menu for `asked` (`ListEvent::ContextMenu`), and keeps it in egui memory
/// under `slot` on the passes after, until its popup closes. A new request always starts a menu
/// from the host's shortcut. At the request's pointer or anchor, with
/// [`ContextMenu`](craft_ui::menu::ContextMenu):
///
/// - On an action: Keyboard Shortcuts (a caption), None and F1 to F12 as radio rows (the current
///   key marked), then Shift and Command (Ctrl off macOS) check rows, enabled only with a key;
///   check rows keep the menu open so both can be ticked. A separator, then Color: a submenu of
///   None, Red, Orange, Yellow, Green, Blue, Violet, Gray (the current one marked). A row reports
///   [`ActionsEvent::AssignShortcut`] or [`ActionsEvent::SetColor`]; each Shift or Command tick
///   is reported as it is made, with the shortcut as it stands.
/// - On a set: Save Set… ([`ActionsEvent::Save`] with the set).
/// - On a step or nothing: no menu.
pub(super) fn show_context_menu(
    ui: &mut Ui,
    slot: Id,
    asked: Option<ContextMenuRequest<ActionSelection>>,
    list: &ActionList,
    host: Option<&dyn ActionsHost>,
    labels: &ActionsLabels,
) -> Option<ActionsEvent> {
    // pc: crates/ui-egui/src/panels/actions.rs:864 `action_context_menu`, :830 `draw_set_header` (Save Set…).
    let ctx = ui.ctx().clone();
    let kept = ctx.data(|data| data.get_temp::<Pending>(slot));
    let mut pending = match (asked, kept) {
        (Some(asked), _) => Pending::opened(asked, list, host),
        (None, Some(kept)) if Popup::is_any_open(&ctx) => Some(kept),
        _ => None,
    };
    let Some(open) = pending.as_mut() else {
        ctx.data_mut(|data| data.remove::<Pending>(slot));
        return None;
    };
    let entries = match open.action {
        Some((_, color)) => action_entries(labels, &LabelColorLabels::shared(ui), open, color),
        None => open.set_entries(labels),
    };
    let widget = ui.interact(open.request.anchor, slot.with("anchor"), Sense::hover());
    let at = open.fresh.then(|| open.request.pointer.map_or(OpenAt::Rect(open.request.anchor), OpenAt::Pointer));
    open.fresh = false;
    let shown = ContextMenu::new(&entries).own_gestures(false).open_at(at).show(ui, &widget);
    open.seen_open |= shown.open;
    let mut event = None;
    match (shown.chosen, open.action) {
        (Some(MenuCommand::Save(set)), _) => event = Some(ActionsEvent::Save { set: Some(set) }),
        (Some(MenuCommand::Color(color)), Some((index, _))) => event = Some(ActionsEvent::SetColor { action: index, color }),
        (Some(MenuCommand::Key(key)), Some((index, _))) => {
            open.key = key;
            event = Some(ActionsEvent::AssignShortcut { action: index, choice: open.choice() });
        }
        (Some(MenuCommand::Shift), Some((index, _))) => {
            open.shift = !open.shift;
            event = Some(ActionsEvent::AssignShortcut { action: index, choice: open.choice() });
        }
        (Some(MenuCommand::Command), Some((index, _))) => {
            open.command = !open.command;
            event = Some(ActionsEvent::AssignShortcut { action: index, choice: open.choice() });
        }
        _ => {}
    }
    if !shown.open && open.seen_open {
        ctx.data_mut(|data| data.remove::<Pending>(slot));
    } else if let Some(open) = pending {
        ctx.data_mut(|data| data.insert_temp(slot, open));
    }
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

/// An open context menu, kept in egui memory while its popup is open: the request and, for an
/// action, the shortcut as the menu has made it.
#[derive(Clone, Debug)]
struct Pending {
    request: ContextMenuRequest<ActionSelection>,
    /// The action and its colour; `None` for a set.
    action: Option<(usize, ActionColor)>,
    /// The set's name for a set's menu.
    set: Option<String>,
    /// 0 for none, else 1..=12.
    key: u8,
    shift: bool,
    command: bool,
    /// Not opened yet: the next draw opens it.
    fresh: bool,
    seen_open: bool,
}

impl Pending {
    /// The menu for `request` with the ticks of the host's shortcut; `None` for a step, nothing
    /// or an action the list no longer has.
    fn opened(request: ContextMenuRequest<ActionSelection>, list: &ActionList, host: Option<&dyn ActionsHost>) -> Option<Self> {
        let (action, set, current) = match request.row.as_ref()? {
            ActionSelection::Set(name) => (None, Some(name.clone()), None),
            ActionSelection::Action(index) => {
                let action = list.action(*index)?;
                (Some((*index, action.color)), None, host.and_then(|host| host.shortcut(action)))
            }
            ActionSelection::Step(..) => return None,
        };
        Some(Self {
            request,
            action,
            set,
            key: current.map_or(0, |choice| choice.key),
            shift: current.is_some_and(|choice| choice.shift),
            command: current.is_some_and(|choice| choice.command),
            fresh: true,
            seen_open: false,
        })
    }

    fn choice(&self) -> Option<ShortcutChoice> {
        (self.key > 0).then_some(ShortcutChoice { key: self.key, shift: self.shift, command: self.command })
    }

    /// A set's menu: Save Set….
    fn set_entries(&self, labels: &ActionsLabels) -> Vec<MenuEntry<MenuCommand>> {
        self.set.iter().map(|name| MenuItem::new(labels.save_set.clone(), MenuCommand::Save(name.clone())).into()).collect()
    }
}

/// The rows of an action's menu: Keyboard Shortcuts, then Color.
fn action_entries(labels: &ActionsLabels, color_names: &LabelColorLabels, pending: &Pending, color: ActionColor) -> Vec<MenuEntry<MenuCommand>> {
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
    let children = ActionColor::ALL
        .into_iter()
        .map(|each| MenuItem::new(color_names.name(each.label_color()).to_owned(), MenuCommand::Color(each)).radio(each == color).into())
        .collect();
    entries.push(MenuItem::submenu(labels.color.clone(), children).into());
    entries
}
