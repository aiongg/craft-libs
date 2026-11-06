//! The dock's panels, one file each, drawn into the body the dock gives them.

mod adjustments;
mod channels;
mod history;
mod layers;
mod properties;
mod swatches;
#[cfg(test)]
mod tests;

use craft_ui::edit::{CancelReason, EditEvent};
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
    /// Returns the panel the body asks to bring to the front.
    pub fn show(self, ui: &mut Ui, mock: &mut Mock, state: &mut State) -> Option<Panel> {
        match self {
            Panel::Canvas => {}
            Panel::Swatches => swatches::show(ui, mock),
            Panel::Properties => properties::show(ui, mock, &mut state.properties),
            Panel::Adjustments => {
                if adjustments::show(ui, mock) {
                    return Some(Panel::Properties);
                }
            }
            Panel::Layers => layers::show(ui, mock, &mut state.layers),
            Panel::Channels => channels::show(ui, &mut state.channels),
            Panel::History => history::show(ui, mock, &mut state.history),
        }
        None
    }
}

/// What the host does with one event of a numeric edit.
pub(super) enum Outcome<T> {
    /// Show `value`; `record` is true when it ends a gesture that changed the document, so the
    /// host adds a History row.
    Apply { value: f64, record: bool },
    /// The gesture was abandoned: put the value from before it back.
    Restore(T),
}

/// Reads one event of a gesture. `current` is what the host holds now; it is kept in `before` when
/// the gesture begins, so a cancel can restore it. A cancel by the host's own change keeps that
/// change.
pub(super) fn resolve<T: Clone>(event: &EditEvent<f64>, current: &T, before: &mut Option<T>) -> Option<Outcome<T>> {
    match event {
        EditEvent::Begin { value, .. } => {
            *before = Some(current.clone());
            value.map(|value| Outcome::Apply { value, record: false })
        }
        EditEvent::Update { value, .. } => Some(Outcome::Apply { value: *value, record: false }),
        EditEvent::Commit { value, changed, .. } => {
            *before = None;
            Some(Outcome::Apply { value: *value, record: *changed })
        }
        EditEvent::Set { value, .. } => Some(Outcome::Apply { value: *value, record: true }),
        EditEvent::Cancel { reason: CancelReason::External, .. } => {
            *before = None;
            None
        }
        EditEvent::Cancel { .. } => before.take().map(Outcome::Restore),
        _ => None,
    }
}
