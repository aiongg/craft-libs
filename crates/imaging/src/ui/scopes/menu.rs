//! [`ScopePanel::panel_menu`], [`ScopePanel::run_command`] and [`ScopePanel::scope_menu`].

use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::localized::SharedLabels as _;
use craft_ui::menu::{MenuEntry, MenuItem};
use egui::{AsIdSalt, Context};

use super::labels::ScopePanelLabels;
use super::panel::{ScopePanel, ScopePanelState, ScopeView};

const REFRESH: &str = "scopes.refresh";
const STATISTICS: &str = "scopes.showStatistics";
const CHANNELS_IN_COLOR: &str = "scopes.showChannelsInColor";
const LAB_READOUTS: &str = "scopes.labReadouts";
const VECTORSCOPE: &str = "scopes.showVectorscope";
const SELECTED_REGION: &str = "scopes.selectedRegion";
const RED_RIGHT: &str = "scopes.redRight";
const SKIN_TONE: &str = "scopes.skinTone";

fn view_command(view: ScopeView) -> PanelCommand {
    match view {
        ScopeView::Compact => "scopes.view.compact",
        ScopeView::Expanded => "scopes.view.expanded",
        ScopeView::AllChannels => "scopes.view.allChannels",
    }
    .to_owned()
}

/// A check row that leaves the menu open.
fn check(label: &str, command: &str, on: bool) -> MenuEntry<PanelCommand> {
    MenuItem::new(label, command.to_owned()).check(on).keep_open(true).into()
}

/// What a chosen menu row asks the app to do beyond the settings change it made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScopeMenuAction {
    /// The row changed [`ScopeSettings`](super::ScopeSettings): persist them.
    SettingsChanged,
    /// Uncached Refresh: recount at cache level 1.
    Refresh,
}

impl ScopePanel<'_> {
    /// The panel menu's rows, for the docking host's `panel_menu_entries`: Uncached Refresh; a
    /// separator; Compact View, Expanded View, All Channels View (radio marks); a separator; Show
    /// Statistics, Show Channels in Color (check marks; check rows keep the menu open). Run the
    /// chosen row with [`run_command`](Self::run_command).
    pub fn panel_menu(ctx: &Context, _id_salt: impl AsIdSalt, state: &ScopePanelState) -> Vec<MenuEntry<PanelCommand>> {
        // pc: crates/ui-egui/src/panels/histogram_panel.rs:809 `panel_menu` (PhotoCraft closes on a check; the library keeps it open).
        let labels = ScopePanelLabels::shared_ctx(ctx);
        let settings = &state.settings;
        let view = |view: ScopeView, label: &str| -> MenuEntry<PanelCommand> {
            MenuItem::new(label, view_command(view)).radio(settings.view == view).into()
        };
        vec![
            MenuItem::new(labels.uncached_refresh.as_str(), REFRESH.to_owned()).into(),
            MenuEntry::Separator,
            view(ScopeView::Compact, &labels.compact),
            view(ScopeView::Expanded, &labels.expanded),
            view(ScopeView::AllChannels, &labels.all_channels),
            MenuEntry::Separator,
            check(&labels.show_statistics, STATISTICS, settings.show_statistics),
            check(&labels.show_channels_in_color, CHANNELS_IN_COLOR, settings.show_channels_in_color),
        ]
    }

    /// The scope menu (right-click on a plot, the Menu key, Shift+F10): Show Lab Color Readouts,
    /// Show Vectorscope; with the vectorscope on, Show Selected Region (only when
    /// `has_selection`), Show Red at 3 o'clock, Show Skin Tone Indicator. Check rows keep it open,
    /// so the rows that depend on Show Vectorscope appear in place. The panel opens it itself; a
    /// host shows the same rows elsewhere (a floating scope's header) with this call.
    pub fn scope_menu(ctx: &Context, _id_salt: impl AsIdSalt, state: &ScopePanelState, has_selection: bool) -> Vec<MenuEntry<PanelCommand>> {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:362-378 `context_menu`, `context`.
        let labels = ScopePanelLabels::shared_ctx(ctx);
        let settings = &state.settings;
        let mut entries = vec![
            check(&labels.lab_readouts, LAB_READOUTS, settings.lab_readouts),
            check(&labels.show_vectorscope, VECTORSCOPE, settings.vectorscope),
        ];
        if settings.vectorscope {
            if has_selection {
                entries.push(check(&labels.selected_region, SELECTED_REGION, settings.selected_region));
            }
            entries.push(check(&labels.red_right, RED_RIGHT, settings.red_right));
            entries.push(check(&labels.skin_tone, SKIN_TONE, settings.skin_tone));
        }
        entries
    }

    /// Runs a chosen row of either menu on `state`: a view, a check or Refresh. `None` when the
    /// command is not the scope panel's.
    pub fn run_command(_ctx: &Context, _id_salt: impl AsIdSalt, state: &mut ScopePanelState, command: &PanelCommand) -> Option<ScopeMenuAction> {
        // new: as craft-color crates/color/src/ui/panel.rs:318 `ColorPanel::run_command`.
        let settings = &mut state.settings;
        let flip = |flag: &mut bool| *flag = !*flag;
        match command.as_str() {
            REFRESH => return Some(ScopeMenuAction::Refresh),
            STATISTICS => flip(&mut settings.show_statistics),
            CHANNELS_IN_COLOR => flip(&mut settings.show_channels_in_color),
            LAB_READOUTS => flip(&mut settings.lab_readouts),
            VECTORSCOPE => flip(&mut settings.vectorscope),
            SELECTED_REGION => flip(&mut settings.selected_region),
            RED_RIGHT => flip(&mut settings.red_right),
            SKIN_TONE => flip(&mut settings.skin_tone),
            other => {
                let view = [ScopeView::Compact, ScopeView::Expanded, ScopeView::AllChannels].into_iter().find(|v| view_command(*v) == other)?;
                settings.view = view;
            }
        }
        Some(ScopeMenuAction::SettingsChanged)
    }
}
