//! The panel menu and the context menus (Keyboard Shortcuts, Color; Save Set…).

use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::list::ContextMenuRequest;
use craft_ui::menu::MenuEntry;
use egui::{AsIdSalt, Context, Ui};

use super::labels::ActionsLabels;
use super::panel::{ActionSelection, ActionsEvent, ActionsHost, ActionsPanel, ActionsState};
use crate::actions::ActionList;

impl ActionsPanel<'_> {
    /// The panel menu's rows, for the docking host's `panel_menu_entries`: New Action… (not while
    /// recording), New Set…, Set Options… (needs a current set: the selected set, or the selected
    /// action's), a separator, Play (needs an action), Start Recording or Stop Recording, Delete
    /// (as the footer's), a separator, Insert Stop… (needs a selected action or step), Batch…
    /// (needs an action), a separator, Load Actions…, Save Actions… (needs an action), a separator,
    /// Button Mode (a check row, the footer button's state). Run the chosen row with
    /// [`run_command`](Self::run_command).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn panel_menu(
        ctx: &Context,
        id_salt: impl AsIdSalt,
        state: &ActionsState,
        list: &ActionList,
        recording: bool,
    ) -> Vec<MenuEntry<PanelCommand>> {
        // pc: crates/ui-egui/src/panels/actions.rs:624 `panel_menu`; vectorcraft: crates/ui-egui/src/panels/actions.rs:55-80 (New Set…, Set Options…, Batch…).
        todo!()
    }

    /// Runs a chosen panel-menu row: Button Mode toggles it in `state` and returns `None`; every
    /// other row returns its event (New Action… is [`ActionsEvent::NewAction`]; Play plays the
    /// selection as the footer's Play does). `None` too when the command is not the panel's.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn run_command(
        ctx: &Context,
        id_salt: impl AsIdSalt,
        state: &mut ActionsState,
        command: &PanelCommand,
        list: &ActionList,
    ) -> Option<ActionsEvent> {
        // new: as craft-color crates/color/src/ui/panel.rs:318 `ColorPanel::run_command`.
        todo!()
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
#[expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
pub(super) fn context_menu(
    ui: &mut Ui,
    request: &ContextMenuRequest<ActionSelection>,
    list: &ActionList,
    host: Option<&dyn ActionsHost>,
    labels: &ActionsLabels,
) -> Option<ActionsEvent> {
    // pc: crates/ui-egui/src/panels/actions.rs:864 `action_context_menu`, :830 `draw_set_header` (Save Set…).
    todo!()
}
