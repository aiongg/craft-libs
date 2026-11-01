//! [`CropBarStyle`]: the sizes of the Crop options bar's menus, from tokens (task TK2). The frame's
//! own look is [`CropStyle`](super::CropStyle).

use super::bar::CropOptionsBar;
use craft_ui::theme::UiTokens;

/// What the overlay and gear menus of the Crop options bar take.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CropBarStyle {
    /// The overlay menu's and the gear popover's width: `crop.menu_width` (240 at both Sizes).
    pub menu_width: f32,
    /// The gear popover's widest: `crop.menu_max_width` (300 at both Sizes).
    pub menu_max_width: f32,
}

impl CropOptionsBar<'_> {
    /// The menus' sizes in `tokens`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: the leaves `crop.menu_*` land in task TK2")]
    pub fn style(tokens: &UiTokens) -> CropBarStyle {
        // pc: crates/ui-egui/src/tools/crop_shield.rs:150 (240 and 300; the overlay menu's 220 at
        // crop_overlay.rs:285 is unified to 240, lead call 17).
        todo!()
    }
}
