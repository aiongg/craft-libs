//! [`SelectMaskStyle`]: Select and Mask's canvas marks, from tokens (task FX7-41).

use craft_ui::canvas::draft::DraftReach;
use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// What Select and Mask draws on the canvas and how near a press counts, from the
/// `select_mask` family (the same at every Size: the marks sit on the image).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectMaskStyle {
    /// The Polygonal Lasso's reach: `select_mask.close_reach` (6, PhotoCraft's, not the kernel
    /// `outline_draft.close` of 8) to close, the kernel's same-point reach for the repeated click.
    pub reach: DraftReach,
    /// An Object Selection drag shorter than this many screen points is a click:
    /// `select_mask.click_reach` (4).
    pub click_reach: f64,
    /// The brush stroke being drawn: `select_mask.trail`.
    pub trail: Color32,
}

impl SelectMaskStyle {
    /// The look in `tokens`.
    pub fn of(tokens: &UiTokens) -> Self {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:603 (the close reach 6; the same-point
        // reach is the Lasso's 3).
        let t = &tokens.component.select_mask;
        Self {
            reach: DraftReach { close: f64::from(t.close_reach), ..DraftReach::of(tokens) },
            click_reach: f64::from(t.click_reach),
            trail: t.trail.to_egui(),
        }
    }
}
