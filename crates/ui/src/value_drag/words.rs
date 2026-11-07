//! Built-in words of the value drag family.

craft_ui::words! {
    /// The words of range sliders.
    pub struct ValueDragWords {
        lower: "valueDrag.lower" = "Minimum";
        upper: "valueDrag.upper" = "Maximum";
        thumb_low: template "thumbRow.low" = "{name} low";
        thumb_high: template "thumbRow.high" = "{name} high";
    }
}
