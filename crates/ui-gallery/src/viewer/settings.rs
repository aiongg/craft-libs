//! What the viewer keeps between runs, in the gallery's own `config.toml`.

use super::appearance::{CONTRASTS, CellFilter, DENSITIES, DIRECTIONS, MODES, MOTIONS, SCALES, SIZES, TEXT_SIZES, theme_named};
use super::state::Viewer;
use craft_settings::Settings;
use craft_ui::runtime::env::RootEnv;
use serde::{Deserialize, Serialize};

/// The viewer's saved state. Choices are saved by name; a name this build does not know falls back to the default
/// of its field, and a layout it cannot read to the default layout, without costing the other fields.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct GallerySettings {
    /// The id of the theme the stories are shown in.
    pub theme: String,
    /// Light or dark.
    pub mode: String,
    /// Standard or high contrast.
    pub contrast: String,
    /// The text-size step.
    pub text_size: String,
    /// The density preset.
    pub density: String,
    /// The motion mode.
    pub motion: String,
    /// The interface scale in percent.
    pub scale: String,
    /// The direction the cells show.
    pub direction: String,
    /// The control size the cells show.
    pub size: String,
    /// The text that narrows the component tree.
    pub filter: String,
    /// The registered type name of the selected component; empty when none is.
    pub selected: String,
    /// The module groups that are open in the tree.
    pub open_modules: Vec<String>,
    /// The panel layout, as `PanelShares::save` writes it.
    pub layout: String,
}

impl Settings for GallerySettings {
    const APP_ID: &'static str = "craft-ui-gallery";
    const SCHEMA_VERSION: u32 = 1;
}

impl GallerySettings {
    /// The saved state of `viewer`.
    pub fn of(viewer: &Viewer) -> GallerySettings {
        let env = &viewer.env;
        let filter = viewer.cell_filter;
        GallerySettings {
            theme: env.theme.as_str().to_owned(),
            mode: MODES.name(env.mode).to_owned(),
            contrast: CONTRASTS.name(env.contrast).to_owned(),
            text_size: TEXT_SIZES.name(env.text_size).to_owned(),
            density: DENSITIES.name(env.density).to_owned(),
            motion: MOTIONS.name(env.motion).to_owned(),
            scale: SCALES.name(env.scale).to_owned(),
            direction: DIRECTIONS.name(filter.direction).to_owned(),
            size: SIZES.name(filter.size).to_owned(),
            filter: viewer.browser.filter.clone(),
            selected: viewer.selected_type().unwrap_or_default().to_owned(),
            open_modules: viewer.open_modules(),
            layout: viewer.layout().save(),
        }
    }

    /// Sets the root values these settings name on `env`; a field with an unknown name takes its default.
    pub fn restore_env(&self, env: &mut RootEnv) {
        let default = RootEnv::default();
        env.theme = theme_named(&self.theme).unwrap_or(default.theme);
        env.mode = MODES.parse(&self.mode).unwrap_or(default.mode);
        env.contrast = CONTRASTS.parse(&self.contrast).unwrap_or(default.contrast);
        env.text_size = TEXT_SIZES.parse(&self.text_size).unwrap_or(default.text_size);
        env.density = DENSITIES.parse(&self.density).unwrap_or(default.density);
        env.motion = MOTIONS.parse(&self.motion).unwrap_or(default.motion);
        env.scale = SCALES.parse(&self.scale).unwrap_or(default.scale);
    }

    /// The cells these settings show; a field with an unknown name shows both.
    pub fn cell_filter(&self) -> CellFilter {
        CellFilter { direction: DIRECTIONS.parse(&self.direction).unwrap_or_default(), size: SIZES.parse(&self.size).unwrap_or_default() }
    }
}
