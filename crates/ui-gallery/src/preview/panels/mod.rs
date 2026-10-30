//! The dock's panels, one file each, drawn into the body the dock gives them.

mod adjustments;
mod channels;
mod history;
mod layers;
mod properties;
mod swatches;
#[cfg(test)]
mod tests;

use egui::Ui;

use super::model::Mock;

/// The panels the dock holds, in the order of the Window menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Panel {
    /// The protected centre panel of the dock. Not in [`Panel::ALL`]: the Window menu cannot hide it.
    Canvas,
    Swatches,
    Properties,
    Adjustments,
    Layers,
    Channels,
    History,
}

impl Panel {
    pub const ALL: [Panel; 6] = [Panel::Swatches, Panel::Properties, Panel::Adjustments, Panel::Layers, Panel::Channels, Panel::History];

    pub fn label(self) -> &'static str {
        match self {
            Panel::Canvas => "Canvas",
            Panel::Swatches => "Swatches",
            Panel::Properties => "Properties",
            Panel::Adjustments => "Adjustments",
            Panel::Layers => "Layers",
            Panel::Channels => "Channels",
            Panel::History => "History",
        }
    }
}

/// What the panels keep between frames besides the document: list selections and the channels'
/// eyes.
#[derive(Clone, Debug, Default)]
pub struct State {
    layers: layers::State,
    channels: channels::State,
    properties: properties::State,
    history: history::State,
}

impl Panel {
    /// The panel's body. The canvas is the dock's own.
    pub fn show(self, ui: &mut Ui, mock: &mut Mock, state: &mut State) {
        match self {
            Panel::Canvas => {}
            Panel::Swatches => swatches::show(ui, mock),
            Panel::Properties => properties::show(ui, mock, &mut state.properties),
            Panel::Adjustments => adjustments::show(ui, mock),
            Panel::Layers => layers::show(ui, mock, &mut state.layers),
            Panel::Channels => channels::show(ui, &mut state.channels),
            Panel::History => history::show(ui, mock, &mut state.history),
        }
    }
}
