//! [`ActionsPanelStyle`]: the Actions panel's look, from the semantic tokens.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

use super::panel::ActionsPanel;
use crate::actions::ActionColor;

/// The radius of the recording dot.
const RECORDING_DOT_RADIUS: f32 = 4.0;

/// Button Mode fills, red to violet: the default palette, the same in every theme.
pub const DEFAULT_FILLS: [Color32; 6] = [
    Color32::from_rgb(0xC4, 0x40, 0x40),
    Color32::from_rgb(0xCC, 0x78, 0x28),
    Color32::from_rgb(0xC4, 0xB0, 0x30),
    Color32::from_rgb(0x38, 0x8C, 0x48),
    Color32::from_rgb(0x30, 0x68, 0xB0),
    Color32::from_rgb(0x78, 0x48, 0xA8),
];

/// The Actions panel's sizes and colours. Rows otherwise take the `ListRow` look (`list.*`,
/// `state.hover`, `state.selected`); the footer is a `PanelFooter`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ActionsPanelStyle {
    /// A set header's height: `size.control.md - space.gap.tight`.
    pub set_row_height: f32,
    /// An action row's height: `size.control.md`.
    pub row_height: f32,
    /// A step row's height: `size.control.sm`.
    pub step_row_height: f32,
    /// The radius of the recording dot.
    pub recording_dot_radius: f32,
    /// Button Mode fills, red to violet: [`DEFAULT_FILLS`].
    pub fills: [Color32; 6],
    /// Button Mode's gray fill: `text.muted`.
    pub gray: Color32,
    /// Button Mode's fill for no colour: `surface.raised`.
    pub no_color: Color32,
    /// A label on a coloured fill: `text.on_status`.
    pub label_on_color: Color32,
    /// A label on no colour: `text.normal`.
    pub label: Color32,
    /// The recording dot and the Record button while recording: `status.bad.solid`.
    pub recording: Color32,
}

impl ActionsPanelStyle {
    /// The Button Mode fill of `color`.
    pub fn fill(&self, color: ActionColor) -> Color32 {
        match color {
            ActionColor::None => self.no_color,
            ActionColor::Red => self.fills[0],
            ActionColor::Orange => self.fills[1],
            ActionColor::Yellow => self.fills[2],
            ActionColor::Green => self.fills[3],
            ActionColor::Blue => self.fills[4],
            ActionColor::Violet => self.fills[5],
            ActionColor::Gray => self.gray,
        }
    }

    /// The Button Mode label colour on `color`'s fill.
    pub fn label_color(&self, color: ActionColor) -> Color32 {
        if color == ActionColor::None { self.label } else { self.label_on_color }
    }
}

impl ActionsPanel<'_> {
    /// The Actions panel's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> ActionsPanelStyle {
        let (c, size) = (&tokens.color, &tokens.size);
        ActionsPanelStyle {
            set_row_height: size.control.md - tokens.space.gap.tight,
            row_height: size.control.md,
            step_row_height: size.control.sm,
            recording_dot_radius: RECORDING_DOT_RADIUS,
            fills: DEFAULT_FILLS,
            gray: c.text.muted.to_egui(),
            no_color: c.surface.raised.to_egui(),
            label_on_color: c.text.on_status.to_egui(),
            label: c.text.normal.to_egui(),
            recording: c.status.bad.solid.to_egui(),
        }
    }
}
