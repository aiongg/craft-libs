//! The panel menu (`panel_menu`, `run_command`, `LayersMenuAction`), the eye colour menu and the
//! row menu slot.

use craft_theme::LabelColor;
use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::localized::{LabelColorLabels, Labels as _};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt, facts_id};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{AsIdSalt, Context, IdSalt, Response, Ui};

use super::LayersPanel;
use super::context::{PanelContext, RowMenu};
use super::events::{LayersEvent, MenuTarget};
use super::labels::LayersLabels;
use crate::ui::options::LayersSettings;
use crate::ui::thumbs::ThumbContents;
use crate::ui::tree::LayerTree;

/// The panel menu's Filter Options › Show.
pub(super) const SHOW_FILTERS: &str = "layers.filter.show";
/// The panel menu's Panel Options….
pub(super) const PANEL_OPTIONS: &str = "layers.options";
/// The panel menu's Collapse All Groups.
pub(super) const COLLAPSE_ALL: &str = "layers.collapseAll";
/// The panel menu's Clip Thumbnails to Layer Bounds.
pub(super) const CLIP_THUMBNAILS: &str = "layers.clipThumbnails";

/// What a panel-menu command asks of the app, from [`LayersPanel::run_command`].
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

impl<T: LayerTree> LayersPanel<'_, T> {
    /// The panel menu's library rows, for the docking host's `panel_menu_entries` hook; the app
    /// puts its own rows first (New Layer…, New Group…, Flatten Image, Lock Layers…, …).
    ///
    /// In PhotoCraft's order: Filter Options › Show (a check, `settings.show_filters`), Panel
    /// Options…, Collapse All Groups (enabled once the panel has been drawn; PhotoCraft: whenever
    /// a document is open), Clip Thumbnails to Layer Bounds (a check, Thumbnail Contents is Layer
    /// Bounds). Animation Options is not offered. `id_salt` is the panel's; before its first pass
    /// Collapse All Groups is disabled. [`run_command`](Self::run_command) carries the commands out.
    pub fn panel_menu(ctx: &Context, id_salt: impl AsIdSalt, settings: &LayersSettings) -> Vec<MenuEntry<PanelCommand>> {
        // pc: crates/ui-egui/src/panels/layer_row_ui.rs:296-334 (Filter Options › Show, Panel Options…,
        // Collapse All Groups, Clip Thumbnails to Layer Bounds); the facts the panel stores under its id
        // as `TextStylesPanel::panel_menu` (craft-text `ui/style_panel.rs`) does.
        let labels = LayersLabels::of_ctx(ctx);
        let drawn = ctx.data(|d| d.get_temp::<Drawn>(facts_id(IdSalt::new(id_salt)))).is_some();
        let item = |text: &str, command: &str| MenuItem::new(text, command.to_owned());
        vec![
            MenuItem::submenu(labels.filter_options.as_str(), vec![item(&labels.show_filters, SHOW_FILTERS).check(settings.show_filters).into()])
                .into(),
            item(&labels.panel_options, PANEL_OPTIONS).into(),
            MenuEntry::Separator,
            item(&labels.collapse_all, COLLAPSE_ALL).enabled(drawn).into(),
            MenuEntry::Separator,
            item(&labels.clip_thumbnails, CLIP_THUMBNAILS).check(settings.options.thumbnail_contents == ThumbContents::LayerBounds).into(),
        ]
    }

    /// Carries out a command of [`panel_menu`](Self::panel_menu): Show and Clip Thumbnails flip
    /// their setting in `settings` and report [`LayersMenuAction::SettingsChanged`]; Panel
    /// Options… and Collapse All Groups report what the app does. Any other command returns
    /// `None`. `ctx` and `id_salt` are unused; every family's `run_command` takes them.
    pub fn run_command(_ctx: &Context, _id_salt: impl AsIdSalt, settings: &mut LayersSettings, command: &PanelCommand) -> Option<LayersMenuAction> {
        // pc: crates/ui-egui/src/panels/layer_row_ui.rs:296-334.
        match command.as_str() {
            SHOW_FILTERS => {
                settings.show_filters = !settings.show_filters;
                Some(LayersMenuAction::SettingsChanged)
            }
            CLIP_THUMBNAILS => {
                settings.options.thumbnail_contents = match settings.options.thumbnail_contents {
                    ThumbContents::LayerBounds => ThumbContents::EntireDocument,
                    ThumbContents::EntireDocument => ThumbContents::LayerBounds,
                };
                Some(LayersMenuAction::SettingsChanged)
            }
            PANEL_OPTIONS => Some(LayersMenuAction::OpenOptions),
            COLLAPSE_ALL => Some(LayersMenuAction::CollapseAll),
            _ => None,
        }
    }
}

/// Marks the panel as drawn, which enables Collapse All Groups in the panel menu; stored under
/// [`facts_id`] of the panel's salt.
#[derive(Clone, Copy)]
struct Drawn;

/// Records that the panel `salt` has been drawn (`LayersPanel::show` calls it once per pass).
pub(super) fn note_drawn(ctx: &Context, salt: IdSalt) {
    ctx.data_mut(|d| d.insert_temp(facts_id(salt), Drawn));
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
    /// The eye cell's colour menu: a context-menu request with `toggle: Some(0)`.
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
    let ctx = ui.ctx();
    let key = cx.id.with("open-menu");
    let mut at = None;
    match request {
        Some(MenuRequest::Row { target, at: place }) if row_menu.is_some() => {
            ctx.data_mut(|d| d.insert_temp(key, OpenMenu::Row(target)));
            at = Some(place);
        }
        Some(MenuRequest::Eye { rows, current, at: place }) => {
            ctx.data_mut(|d| d.insert_temp(key, OpenMenu::Eye { rows, current }));
            at = Some(place);
        }
        _ => {}
    }
    let Some(open) = ctx.data(|d| d.get_temp::<OpenMenu<T::Id>>(key)) else { return Vec::new() };
    // One menu for both kinds: the entries of each carry the pick they stand for.
    let entries: Vec<MenuEntry<Pick>> = match &open {
        OpenMenu::Row(target) => {
            let host = row_menu.map(|menu| menu(target)).unwrap_or_default();
            host.iter().map(|entry| entry.map(&|command: &String| Pick::Command(command.clone()))).collect()
        }
        OpenMenu::Eye { current, .. } => {
            eye_entries(ui, *current).iter().map(|entry| entry.map(&|color: &Option<LabelColor>| Pick::Color(*color))).collect()
        }
    };
    let shown = ContextMenu::new(&entries).own_gestures(false).open_at(at).show(ui, list);
    let events: Vec<LayersEvent<T::Id>> = match (shown.chosen, open) {
        (Some(Pick::Command(command)), OpenMenu::Row(target)) => vec![LayersEvent::Menu { target, command }],
        (Some(Pick::Color(color)), OpenMenu::Eye { rows, .. }) => vec![LayersEvent::LabelColor { rows, color }],
        _ => Vec::new(),
    };
    if !(shown.open || at.is_some()) || !events.is_empty() {
        ctx.data_mut(|d| d.remove::<OpenMenu<T::Id>>(key));
    }
    events
}

/// What a pick of the shared menu stands for.
#[derive(Clone)]
enum Pick {
    /// The host's command in a row menu.
    Command(String),
    /// A label colour (or none) in an eye menu.
    Color(Option<LabelColor>),
}

/// The menu kept open across passes, with what it acts on.
#[derive(Clone)]
enum OpenMenu<Id> {
    Row(MenuTarget<Id>),
    Eye { rows: Vec<Id>, current: Option<LabelColor> },
}

/// None, a separator, then every label colour with its swatch and a check on `current`.
fn eye_entries(ui: &Ui, current: Option<LabelColor>) -> Vec<MenuEntry<Option<LabelColor>>> {
    let tokens = Tokens::of(ui);
    let names = LabelColorLabels::of(ui);
    let mut entries = vec![MenuItem::new(names.name(None), None).into(), MenuEntry::Separator];
    for color in LabelColor::ALL {
        let item = MenuItem::new(names.name(Some(color)), Some(color)).check(current == Some(color)).tint(tokens.component.list.label(color).swatch);
        entries.push(item.into());
    }
    entries
}
