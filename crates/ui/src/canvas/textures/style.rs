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

/// The style of `tokens`: the `thumbnail` family.
pub(super) fn style(tokens: &UiTokens) -> ThumbnailStyle {
    let thumbnail = &tokens.component.thumbnail;
    ThumbnailStyle {
        checker: [thumbnail.checker_a.to_egui(), thumbnail.checker_b.to_egui()],
        checker_cell: thumbnail.checker_cell,
        placeholder: thumbnail.placeholder.to_egui(),
        failed_icon: thumbnail.failed_icon.to_egui(),
    }
}
