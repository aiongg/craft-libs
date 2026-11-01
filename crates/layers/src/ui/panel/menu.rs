//! The panel menu (`panel_menu`, `run_command`, `LayersMenuAction`), the eye colour menu and the
//! row menu slot.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::LabelColor;
use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::menu::{MenuEntry, OpenAt};
use egui::{AsIdSalt, Context, Response, Ui};

use super::context::{PanelContext, RowMenu};
use super::events::{LayersEvent, MenuTarget};
use super::labels::LayersLabels;
use crate::ui::options::LayersSettings;
use crate::ui::tree::LayerTree;

/// The panel menu's Filter Options › Show.
pub const SHOW_FILTERS: &str = "layers.filter.show";
/// The panel menu's Panel Options….
pub const PANEL_OPTIONS: &str = "layers.options";
/// The panel menu's Collapse All Groups.
pub const COLLAPSE_ALL: &str = "layers.collapseAll";
/// The panel menu's Clip Thumbnails to Layer Bounds.
pub const CLIP_THUMBNAILS: &str = "layers.clipThumbnails";

/// What a panel-menu command asks of the app, from [`run_command`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LayersMenuAction {
    /// The settings changed (Show Filters, Clip Thumbnails to Layer Bounds); an app that shares
    /// Thumbnail Contents with another preference mirrors it.
    SettingsChanged,
    /// Open Layers Panel Options: the app opens
    /// [`LayersOptionsDialog`](crate::ui::options::LayersOptionsDialog) with the settings'
    /// options and stores its result.
    OpenOptions,
    /// Collapse every group: the app closes them in its open state.
    CollapseAll,
}

/// The panel menu's library rows, for the docking host's `panel_menu_entries` hook; the app puts
/// its own rows first (New Layer…, New Group…, Flatten Image, Lock Layers…, …).
///
/// In PhotoCraft's order: Filter Options › Show (a check, `settings.show_filters`), Panel
/// Options…, Collapse All Groups (enabled once the panel has been drawn; PhotoCraft: whenever a document is open),
/// Clip Thumbnails to Layer Bounds (a check, Thumbnail Contents is Layer Bounds). Animation
/// Options is not offered. `id_salt` is the panel's; before its first pass Collapse All Groups is
/// disabled. Commands: [`SHOW_FILTERS`], [`PANEL_OPTIONS`], [`COLLAPSE_ALL`],
/// [`CLIP_THUMBNAILS`].
pub fn panel_menu(ctx: &Context, id_salt: impl AsIdSalt, settings: &LayersSettings) -> Vec<MenuEntry<PanelCommand>> {
    // pc: crates/ui-egui/src/panels/layer_row_ui.rs:296-334 (Filter Options › Show, Panel Options…,
    // Collapse All Groups, Clip Thumbnails to Layer Bounds); the facts the panel stores under its id
    // as `TextStylesPanel::panel_menu` (craft-text `ui/style_panel.rs`) does.
    todo!()
}

/// Carries out a command of [`panel_menu`]: Show and Clip Thumbnails flip their setting in
/// `settings` and report [`LayersMenuAction::SettingsChanged`]; Panel Options… and Collapse All
/// Groups report what the app does. Any other command returns `None`.
pub fn run_command(ctx: &Context, id_salt: impl AsIdSalt, settings: &mut LayersSettings, command: &PanelCommand) -> Option<LayersMenuAction> {
    // pc: crates/ui-egui/src/panels/layer_row_ui.rs:296-334.
    todo!()
}

/// A menu the body asks for this pass.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum MenuRequest<Id> {
    /// The host's row menu on a row, a mask thumbnail or an effects row.
    Row {
        /// Where it was opened.
        target: MenuTarget<Id>,
        /// The pointer or the row's rect (Menu key, Shift+F10).
        at: OpenAt,
    },
    /// The eye cell's colour menu.
    // needs: A `ContextMenuRequest::toggle` (craft-ui `list/view.rs`): a secondary click on a
    // toggle cell reports a context-menu request with the column, so the body knows it is the eye.
    Eye {
        /// The layers it colours: the selection when the row is in it, else the row.
        rows: Vec<Id>,
        /// The colour they share, checked; `None` when they differ or have none.
        current: Option<LabelColor>,
        /// Where it opens.
        at: OpenAt,
    },
}

/// Shows the row menus on the list's response: opens the one `request` asks for (it stays open
/// over later passes, its target kept in egui memory under the panel's id) and reports a pick.
///
/// - **Row menu**: the host's `row_menu(target)` entries, rebuilt each pass while open; a pick
///   reports [`LayersEvent::Menu`]. Without a host menu nothing opens.
/// - **Eye menu**: None, a separator, then [`LabelColor::ALL`] with a swatch each
///   (`list.label.<color>` fill), a check on `current`; a pick reports
///   [`LayersEvent::LabelColor`] for `rows`.
/// - Keyboard: the menus take arrows, Enter and Esc ([`ContextMenu`](craft_ui::menu::ContextMenu));
///   Esc gives the list the focus back.
pub(super) fn show_row_menus<T: LayerTree>(
    ui: &Ui,
    cx: &PanelContext<'_, T>,
    list: &Response,
    row_menu: Option<&RowMenu<'_, T::Id>>,
    request: Option<MenuRequest<T::Id>>,
) -> Vec<LayersEvent<T::Id>> {
    // pc: crates/ui-egui/src/panels/layer_row.rs:470-496 `row_context_menu`;
    // pc: crates/ui-egui/src/panels/layer_menu_ui.rs:266-287 `color_choices` (None first, a separator,
    // then the colours; the current one checked).
    todo!()
}

/// The menu name of `color` (`None`: "None").
pub(super) fn color_name(labels: &LayersLabels, color: Option<LabelColor>) -> &str {
    // new: `LayersLabels::{no_color, red, …, gray}` in `LabelColor::ALL` order.
    todo!()
}
