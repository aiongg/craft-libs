//! [`CropBarStyle`]: the sizes of the Crop options bar's menus and preset select, from tokens. The frame's
//! own look is [`CropStyle`](super::CropStyle).

use super::bar::CropBar;
use craft_theme::Rgba;
use craft_ui::theme::UiTokens;

/// The frame, brackets and edge bars.
pub(super) const CROP: Rgba = Rgba::rgb(0xff, 0xff, 0xff);
/// The composition guide lines.
pub(super) const CROP_GUIDE: Rgba = Rgba([0xff, 0xff, 0xff, 0x5a]);
/// A perspective quad that cannot be applied.
pub(super) const CROP_INVALID: Rgba = Rgba::rgb(0xe5, 0x3e, 0x3e);

const MENU_WIDTH: f32 = 240.0;
const MENU_MAX_WIDTH: f32 = 300.0;
const PRESET_WIDTH: f32 = 140.0;

/// What the overlay and gear menus of the Crop options bar take.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CropBarStyle {
    /// The overlay menu's and the gear popover's width.
    pub menu_width: f32,
    /// The gear popover's widest.
    pub menu_max_width: f32,
    /// The ratio preset select's width.
    pub preset_width: f32,
}

impl CropBar<'_> {
    /// The menus' and the preset select's sizes in `tokens`.
    pub fn style(_tokens: &UiTokens) -> CropBarStyle {
        CropBarStyle { menu_width: MENU_WIDTH, menu_max_width: MENU_MAX_WIDTH, preset_width: PRESET_WIDTH }
    }
}
