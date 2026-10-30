//! The dock's panels. Each has its own file, filled by the dock step.

/// The panels the dock holds, in the order of the Window menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Panel {
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
            Panel::Swatches => "Swatches",
            Panel::Properties => "Properties",
            Panel::Adjustments => "Adjustments",
            Panel::Layers => "Layers",
            Panel::Channels => "Channels",
            Panel::History => "History",
        }
    }
}
