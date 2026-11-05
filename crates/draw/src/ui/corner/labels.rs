//! The words of the corner widgets ([`CornerWidgetsLabels`]).

craft_ui::words! {
    /// The words of [`CornerWidgets`](super::CornerWidgets), English by default. Templates take
    /// `{i}`, `{n}` and `{radius}` (the radius in the host's ruler units, with the unit).
    #[non_exhaustive]
    pub struct CornerWidgetsLabels {
        /// The AccessKit group of widgets.
        group: "cornerWidgets.group" = "Corner radius";
        /// A widget's accessible name: "Corner 2 of 4".
        corner: "cornerWidgets.corner" = "Corner {i} of {n}";
        /// A widget's accessible value: "radius 12 px".
        value: "cornerWidgets.value" = "radius {radius}";
        /// The readout's label beside the pointer while dragging ("R: 25.0 px").
        readout: "cornerWidgets.readout" = "R:";
    }
}
