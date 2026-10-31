#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! The Preferences appearance page over the suite selection: [`AppearancePage`] and its
//! [`ThemeChip`]s.

use craft_theme::{ThemeDef, ThemeId};
use egui::{Response, Ui};

use crate::{SelectionChange, SuiteClient};

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

craft_ui::words! {
    /// The appearance page's words, English by default.
    #[non_exhaustive]
    pub struct AppearanceLabels {
        appearance: "appearance.appearance" = "Appearance";
        mode: "appearance.mode" = "Appearance Mode";
        mode_system: "appearance.modeSystem" = "Sync with system";
        mode_dark: "appearance.modeDark" = "Dark";
        mode_light: "appearance.modeLight" = "Light";
        light_theme: "appearance.lightTheme" = "Light Theme";
        dark_theme: "appearance.darkTheme" = "Dark Theme";
        active: "appearance.active" = "Active";
        favorite: "appearance.favorite" = "Favorite";
        add_favorite: "appearance.addFavorite" = "Add to Favorites";
        remove_favorite: "appearance.removeFavorite" = "Remove from Favorites";
        size: "appearance.size" = "Size";
        compact: "appearance.compact" = "Compact";
        comfortable: "appearance.comfortable" = "Comfortable";
        text_size: "appearance.textSize" = "UI Font Size";
        contrast: "appearance.contrast" = "Contrast";
        contrast_system: "appearance.contrastSystem" = "Sync with system";
        contrast_standard: "appearance.contrastStandard" = "Standard";
        contrast_high: "appearance.contrastHigh" = "High";
        motion: "appearance.motion" = "Motion";
        motion_system: "appearance.motionSystem" = "Sync with system";
        motion_full: "appearance.motionFull" = "Full";
        motion_reduced: "appearance.motionReduced" = "Reduced";
    }
}

/// What the page reported this pass.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct AppearanceResponse {
    /// The draft changed this pass (the window's Apply becomes enabled).
    pub changed: bool,
    /// The favourite star of this theme was toggled; the app updates its favourites list.
    pub favorite_toggled: Option<ThemeId>,
}

/// The Appearance group of Preferences › Interface over the suite's shared selection, after
/// PhotoCraft's page (gap 64).
///
/// Rows, top to bottom, labels left-aligned (31 Oct decision): Appearance Mode (Sync with system,
/// Dark, Light; a 220 px list), Light Theme and Dark Theme (a row of [`ThemeChip`]s each: the
/// built-in themes, then the library's, each in the rows for the modes it has; "Active" after the
/// chip the app shows now), Size (Compact, Comfortable), UI Font Size (the [`TextSize`]
/// steps), Contrast and Motion (Sync with system, Standard or Full, High or Reduced).
///
/// The page edits `draft`, a [`SelectionChange`] the Preferences model keeps; the chips and lists
/// show [`SuiteClient::shown`] with the draft applied. Nothing reaches the suite until the window
/// applies: the model's `apply` calls `client.change(draft)`, its `cancel` clears the draft
/// (PhotoCraft applies on Apply and OK too). Each chip row is one radio group: one Tab stop,
/// arrows move and pick, Space picks; a chip's context menu (and Shift+F10) has Add to Favorites
/// / Remove from Favorites, the keyboard path to the star.
///
/// ```ignore
/// impl CustomSettings<Prefs> for MyCustom {
///     fn row(&mut self, ui: &mut Ui, key: &str, _draft: &mut SettingsDraft) {
///         if key == "interface.appearance" {
///             AppearancePage::new(&self.client, &mut self.appearance).favorites(&self.favorites).show(ui);
///         }
///     }
///     fn apply(&mut self, _: &Prefs) -> Result<(), String> { self.client.change(std::mem::take(&mut self.appearance)); Ok(()) }
///     fn cancel(&mut self) { self.appearance = SelectionChange::default(); }
///     fn differs(&self) -> bool { !self.appearance.is_empty() }
/// }
/// ```
///
/// [`TextSize`]: craft_theme::TextSize
pub struct AppearancePage<'a> {
    client: &'a SuiteClient,
    draft: &'a mut SelectionChange,
    favorites: &'a [ThemeId],
    text_size: bool,
}

impl<'a> AppearancePage<'a> {
    pub fn new(client: &'a SuiteClient, draft: &'a mut SelectionChange) -> Self {
        Self { client, draft, favorites: &[], text_size: true }
    }

    /// The themes marked with a filled star.
    pub fn favorites(mut self, favorites: &'a [ThemeId]) -> Self {
        self.favorites = favorites;
        self
    }

    /// Shows the UI Font Size row (default on); an app with its own text-size control turns it
    /// off.
    pub fn text_size(mut self, show: bool) -> Self {
        self.text_size = show;
        self
    }

    // pc: crates/ui-egui/src/prefs/prefs_ui/fields.rs:315 interface_groups (appearance group);
    // pc: crates/ui-egui/src/prefs/prefs_ui/appearance.rs:311 (mode list), :268 theme_row
    // needs: S4-text TextSize step names (Photoshop's Tiny, Small, Medium, Large) for the UI Font Size row
    pub fn show(self, ui: &mut Ui) -> AppearanceResponse {
        todo!()
    }
}

/// A 30 × 26 miniature of a theme, after PhotoCraft's theme swatch (gap 64).
///
/// The outline (corner radius 3) holds, inset 3 px, a fill of the theme's panel colour (radius
/// 2) with a 5 px strip of its chrome colour along the top and an accent dot (radius 2.5) 6 px in
/// from the bottom-right corner, all resolved for the row's mode (PhotoCraft samples `dock`,
/// `chrome` and `accent`). The outline is 2 px accent when selected, 1 px dim text on hover, else
/// 1 px card border. A star badge (radius 7, at the top-right corner offset by (−3, +3)) shows
/// filled in the accent for a favourite, and as an outline while the pointer is over the chip;
/// a click on the badge toggles the favourite, a click elsewhere picks the theme. Tooltip and
/// accessible name: the theme's name; AccessKit role `RadioButton` with its selected state.
pub struct ThemeChip<'a> {
    theme: &'a ThemeDef,
    dark: bool,
    selected: bool,
    favorite: bool,
}

/// What a chip reported.
#[derive(Debug)]
pub struct ThemeChipResponse {
    pub response: Response,
    /// The star badge was clicked.
    pub favorite_clicked: bool,
}

impl<'a> ThemeChip<'a> {
    /// The chip of `theme` in its dark variant when `dark`, else its light one.
    pub fn new(theme: &'a ThemeDef, dark: bool) -> Self {
        Self { theme, dark, selected: false, favorite: false }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn favorite(mut self, favorite: bool) -> Self {
        self.favorite = favorite;
        self
    }

    // pc: crates/ui-egui/src/prefs/prefs_ui/appearance.rs:218 theme_swatch (badge :255, star :264, toggle :276)
    pub fn show(self, ui: &mut Ui) -> ThemeChipResponse {
        todo!()
    }
}
