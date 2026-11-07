//! What the viewer keeps between runs, in the gallery's own `config.toml`.

use super::appearance::{CONTRASTS, DENSITIES, DIRECTIONS, MODES, MOTIONS, SCALES, SIZES, TEXT_SIZES};
use super::state::Viewer;
use craft_settings::Settings;
use serde::{Deserialize, Serialize};

/// The viewer's saved state. Choices are saved by name; a name this build does not know falls back to the default
/// of its field, and a layout it cannot read to the default layout, without costing the other fields.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct GallerySettings {
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
        let appearance = viewer.appearance;
        GallerySettings {
            mode: MODES.name(appearance.mode).to_owned(),
            contrast: CONTRASTS.name(appearance.contrast).to_owned(),
            text_size: TEXT_SIZES.name(appearance.text_size).to_owned(),
            density: DENSITIES.name(appearance.density).to_owned(),
            motion: MOTIONS.name(appearance.motion).to_owned(),
            scale: SCALES.name(appearance.scale).to_owned(),
            direction: DIRECTIONS.name(appearance.direction).to_owned(),
            size: SIZES.name(appearance.size).to_owned(),
            filter: viewer.filter.clone(),
            selected: viewer.selected_type().unwrap_or_default().to_owned(),
            open_modules: viewer.open_modules(),
            layout: viewer.layout().save(),
        }
    }

    /// The appearance these settings name; a field with an unknown name keeps its default.
    pub fn appearance(&self) -> super::Appearance {
        let default = super::Appearance::default();
        super::Appearance {
            mode: MODES.parse(&self.mode).unwrap_or(default.mode),
            contrast: CONTRASTS.parse(&self.contrast).unwrap_or(default.contrast),
            text_size: TEXT_SIZES.parse(&self.text_size).unwrap_or(default.text_size),
            density: DENSITIES.parse(&self.density).unwrap_or(default.density),
            motion: MOTIONS.parse(&self.motion).unwrap_or(default.motion),
            scale: SCALES.parse(&self.scale).unwrap_or(default.scale),
            direction: DIRECTIONS.parse(&self.direction).unwrap_or(default.direction),
            size: SIZES.parse(&self.size).unwrap_or(default.size),
        }
    }
}
