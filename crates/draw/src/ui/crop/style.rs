//! [`CropBarStyle`]: the sizes of the Crop options bar's menus and preset select, from tokens. The frame's
//! own look is [`CropStyle`](super::CropStyle).

use super::bar::CropBar;
use craft_ui::theme::UiTokens;

/// What the overlay and gear menus of the Crop options bar take.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CropBarStyle {
    /// The overlay menu's and the gear popover's width: `crop.menu_width` (240 at both Sizes).
    pub menu_width: f32,
    /// The gear popover's widest: `crop.menu_max_width` (300 at both Sizes).
    pub menu_max_width: f32,
    /// The ratio preset select's width: `crop.preset_width` (140 at both Sizes).
    pub preset_width: f32,
}

impl CropBar<'_> {
    /// The menus' and the preset select's sizes in `tokens`.
    pub fn style(tokens: &UiTokens) -> CropBarStyle {
        // pc: crates/ui-egui/src/tools/crop_shield.rs:150 (240 and 300), crop_presets.rs:137 (140; the overlay menu's 220 at
        // crop_overlay.rs:285 is unified to 240, lead call 17).
        let c = &tokens.component.crop;
        CropBarStyle { menu_width: c.menu_width, menu_max_width: c.menu_max_width, preset_width: c.preset_width }
    }
}
