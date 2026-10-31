//! Per-command modal rules: which of the app's menu commands still run while a dialog, a
//! full-window workspace or a pending operation has the window.
//!
//! [`Gate`] alone greys every app command while [`Gate::modal`] is set. Photoshop is finer: with a
//! dialog open the view commands (Zoom In, Zoom Out, Fit on Screen, 100 %) still run from their
//! shortcuts and menus; Camera Raw and the full-window workspaces (Liquify, Select and Mask) block
//! every command; a pending crop greys most of Image, Layer and Select but leaves the view and
//! Help commands. [`ModalRules`] says that per command, and [`Gate::allows_under`] applies it.
//! The app sets the rules every frame beside the gate ([`crate::NativeMenu::set_modal_rules`],
//! [`crate::MenuBarUi::modal_rules`], [`crate::CommandContextMenu::modal_rules`]).

#![expect(unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::fmt;
use std::sync::Arc;

use craft_keymap::CommandRef;

use crate::driver::Gate;
use crate::model::Item;

/// The view commands Photoshop runs while a dialog is open: Zoom In, Zoom Out, Fit on Screen and
/// 100 %, by PhotoCraft's ids. An app whose ids differ passes its own list.
pub const NAVIGATION_COMMANDS: [&str; 4] = ["view.zoomIn", "view.zoomOut", "view.fitOnScreen", "view.actualPixels"];

/// Whether a command may run while a pending operation (an uncommitted crop) is open.
pub type PendingRule = Arc<dyn Fn(&CommandRef) -> bool + Send + Sync>;

/// What runs from the menus, per command, while the window is modal. Built once by the app and
/// changed when a workspace or pending operation starts or ends.
///
/// ```no_run
/// use craft_menu::{Gate, ModalRules, NAVIGATION_COMMANDS};
///
/// let rules = ModalRules::default()
///     .in_dialogs(NAVIGATION_COMMANDS)
///     .pending(Some(std::sync::Arc::new(|c: &craft_menu::CommandRef| c.id.starts_with("view."))));
/// let gate = Gate { modal: true, ..Gate::default() };
/// assert!(gate.allows_under(&rules, &"view.zoomIn".into()));
/// assert!(!gate.allows_under(&rules, &"image.crop".into()));
/// ```
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct ModalRules {
    /// Commands that still run while [`Gate::modal`] is set (a dialog or prompt is open), by
    /// command id. Empty: none of the app's commands do, as with the gate alone.
    pub in_dialogs: Vec<String>,
    /// A full-window workspace (Camera Raw, Liquify, Select and Mask) has the window: no app
    /// command runs from the menus, [`Self::in_dialogs`] included. The built-in items still do.
    pub workspace: bool,
    /// A pending operation is open (an uncommitted crop): only the commands this returns true for
    /// run. `None`: no pending operation.
    pub pending: Option<PendingRule>,
}

// `NativeMenu` keeps the rules in its shared dispatch state, which the platform's menu handler
// reads from another thread.
const _: () = {
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<ModalRules>()
};

impl ModalRules {
    /// The commands that run while a dialog is open: [`NAVIGATION_COMMANDS`] for PhotoCraft.
    // pc: crates/ui-egui/src/input/shortcuts.rs:170 `NAV_COMMANDS`.
    pub fn in_dialogs<S: Into<String>>(mut self, ids: impl IntoIterator<Item = S>) -> Self {
        todo!()
    }

    /// Sets [`Self::workspace`].
    // pc: crates/ui-egui/src/dialogs/distort_ui.rs:105 `modal_workspace`; the Camera Raw check in
    // menus.rs:46 `modal_allows`.
    pub fn workspace(mut self, open: bool) -> Self {
        todo!()
    }

    /// Sets [`Self::pending`].
    // pc: crates/ui-egui/src/tools/crop_ui.rs:555 `blocks`, :559 `available_while_pending`.
    pub fn pending(mut self, rule: Option<PendingRule>) -> Self {
        todo!()
    }
}

impl fmt::Debug for ModalRules {
    // new: `PendingRule` is a closure; print whether one is set.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Gate {
    /// Whether `command` may run now under `rules`. The built-in items (Hide, Minimize, the
    /// built-in Quit) always may. Otherwise, in this order: a workspace blocks every app command;
    /// a pending operation blocks what its rule refuses; while a shortcut is being recorded
    /// nothing runs; while a dialog is open ([`Gate::modal`]) only [`ModalRules::in_dialogs`]
    /// runs. A text field having the keys stops nothing, as with [`Gate::allows`].
    ///
    /// With `ModalRules::default()` this is [`Gate::allows_command`].
    // pc: crates/ui-egui/src/menus.rs:46 `modal_allows`.
    pub fn allows_under(&self, rules: &ModalRules, command: &CommandRef) -> bool {
        todo!()
    }

    /// [`Gate::allows_under`] for an item.
    pub fn allows_item_under(&self, rules: &ModalRules, item: &Item) -> bool {
        todo!()
    }
}
