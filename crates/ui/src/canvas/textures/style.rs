//! The thumbnail look, from [`Thumbnails::style`](super::Thumbnails::style).

use egui::Color32;

use crate::theme::{ToEgui as _, UiTokens};

/// What [`Thumbnails::paint`](super::Thumbnails::paint) draws besides the picture. The
/// thumbnail's frame is its row's (`list.thumb.border`), not part of this.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ThumbnailStyle {
    /// The two cells of the checkerboard behind a picture's transparency: `overlay.checker_light`
    /// (top left) and `overlay.checker_dark`. The same pinned neutrals in every theme: a picture's
    /// backing is not interface chrome.
    pub checker: [Color32; 2],
    /// Side of a checkerboard cell: `size.checker`.
    pub checker_cell: f32,
    /// The cell while no picture has arrived, and behind the failure icon: `surface.sunken`.
    pub placeholder: Color32,
    /// The failure icon (`Icon::Warning`): `text.muted`.
    pub failed_icon: Color32,
}

/// The style of `tokens`: the semantic tokens.
pub(super) fn style(tokens: &UiTokens) -> ThumbnailStyle {
    let (overlay, color) = (&tokens.color.overlay, &tokens.color);
    ThumbnailStyle {
        checker: [overlay.checker_light.to_egui(), overlay.checker_dark.to_egui()],
        checker_cell: tokens.size.checker,
        placeholder: color.surface.sunken.to_egui(),
        failed_icon: color.text.muted.to_egui(),
    }
}
