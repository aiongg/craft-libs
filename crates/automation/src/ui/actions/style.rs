//! [`ActionsPanelStyle`]: the Actions panel's look, from the `actions` tokens.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

use super::panel::ActionsPanel;
use crate::actions::ActionColor;

/// The Actions panel's sizes and colours. Rows otherwise take the `ListRow` look (`list.*`,
/// `state.hover`, `state.selected`); the footer is a `PanelFooter`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ActionsPanelStyle {
    /// A set header's height: `actions.set_row_height` (24).
    pub set_row_height: f32,
    /// An action row's height: `actions.row_height` (26).
    pub row_height: f32,
    /// A step row's height: `actions.step_row_height` (20).
    pub step_row_height: f32,
    /// The indent of an action inside a set: `actions.indent` (14).
    pub indent: f32,
    /// Button Mode fills, red to violet: `actions.color.*`, the same in every theme.
    pub fills: [Color32; 6],
    /// Button Mode's gray fill: `text.muted`.
    pub gray: Color32,
    /// Button Mode's fill for no colour: `surface.raised` (lead call L34).
    pub no_color: Color32,
    /// A label on a coloured fill: black 20 (`rgb(20, 20, 20)`).
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
    ///
    /// Skeleton values are PhotoCraft's literals; TK2 swaps them for the `actions.*` tokens.
    pub fn style(tokens: &UiTokens) -> ActionsPanelStyle {
        // pc: crates/ui-egui/src/panels/actions.rs:297-313 `button_fill`, `button_label_color`; :831 (24), :1040 and :1114 (230/60/60).
        let rgb = Color32::from_rgb;
        ActionsPanelStyle {
            set_row_height: 24.0,
            row_height: 26.0,
            step_row_height: 20.0,
            indent: 14.0,
            fills: [rgb(196, 64, 64), rgb(204, 120, 40), rgb(196, 176, 48), rgb(56, 140, 72), rgb(48, 104, 176), rgb(120, 72, 168)],
            gray: tokens.color.text.muted.to_egui(),
            no_color: tokens.color.surface.raised.to_egui(),
            label_on_color: rgb(20, 20, 20),
            label: tokens.color.text.normal.to_egui(),
            recording: tokens.color.status.bad.solid.to_egui(),
        }
    }
}
