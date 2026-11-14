//! [`ColorRangeWords`]: the Color Range dialog's words.

craft_ui::words! {
    /// The words of the Color Range dialog, in the installed language unless set.
    pub struct ColorRangeWords {
        /// The dialog's title.
        name: "colorRange.name" = "Color Range";
        select: "colorRange.select" = "Select";
        fuzziness: "colorRange.fuzziness" = "Fuzziness";
        range: "colorRange.range" = "Range";
        invert: "colorRange.invert" = "Invert";
        sampled: "colorRange.sampled" = "Sampled Colors";
    }
}
