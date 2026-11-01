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
    /// A set header's height: `actions.set_row_height`.
    pub set_row_height: f32,
    /// An action row's height: `actions.row_height`.
    pub row_height: f32,
    /// A step row's height: `actions.step_row_height`.
    pub step_row_height: f32,
    /// The indent of an action inside a set: `actions.indent`.
    pub indent: f32,
    /// Button Mode fills, red to violet: `actions.color.*`, the same in every theme.
    pub fills: [Color32; 6],
    /// Button Mode's gray fill: `text.muted`.
    pub gray: Color32,
    /// Button Mode's fill for no colour: `surface.raised` (lead call L34).
    pub no_color: Color32,
    /// A label on a coloured fill: `actions.color.label`.
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
        let (a, c) = (&tokens.component.actions, &tokens.color);
        ActionsPanelStyle {
            set_row_height: a.set_row_height,
            row_height: a.row_height,
            step_row_height: a.step_row_height,
            indent: a.indent,
            fills: a.colors.map(|rgba| rgba.to_egui()),
            gray: c.text.muted.to_egui(),
            no_color: c.surface.raised.to_egui(),
            label_on_color: a.label_on_color.to_egui(),
            label: c.text.normal.to_egui(),
            recording: c.status.bad.solid.to_egui(),
        }
    }
}
