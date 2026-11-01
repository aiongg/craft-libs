//! The thumbnail look, from [`Thumbnails::style`](super::Thumbnails::style).

use egui::Color32;

use crate::theme::{ToEgui as _, UiTokens};

/// What [`Thumbnails::paint`](super::Thumbnails::paint) draws besides the picture. The
/// thumbnail's frame is its row's (`list.thumb.border`), not part of this.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ThumbnailStyle {
    /// The two cells of the checkerboard behind a picture's transparency: `thumbnail.checker.a`
    /// (top left) and `thumbnail.checker.b`. The same pinned neutrals in every theme: a picture's
    /// backing is not interface chrome.
    pub checker: [Color32; 2],
    /// Side of a checkerboard cell: `thumbnail.checker_cell`.
    pub checker_cell: f32,
    /// The cell while no picture has arrived, and behind the failure icon: `thumbnail.placeholder`.
    pub placeholder: Color32,
    /// The failure icon (`Icon::Warning`): `thumbnail.failed_icon`.
    pub failed_icon: Color32,
}

/// The style of `tokens`. Until the `thumbnail` token family exists (task TK1), the values come
/// from the tokens they will derive from: the swatch checkerboard's pinned pair (250 and 214
/// grey, `values::SWATCH_CHECKER`), PhotoCraft's cell 5 (`layer_row.rs:534`), `field.fill` and
/// `text.muted`.
pub(super) fn style(tokens: &UiTokens) -> ThumbnailStyle {
    let swatch = &tokens.component.swatch;
    ThumbnailStyle {
        checker: [swatch.checker_a.to_egui(), swatch.checker_b.to_egui()],
        checker_cell: 5.0,
        placeholder: tokens.component.field.fill.rest.to_egui(),
        failed_icon: tokens.color.text.muted.to_egui(),
    }
}
