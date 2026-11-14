//! The words of the paint bar ([`PaintBarWords`]).

craft_ui::words! {
    /// The words of [`PaintBar`](super::PaintBar), English by default (Photoshop's).
    pub struct PaintBarWords {
        /// The Size field's caption.
        size: "paintBar.size" = "Size";
        /// The Hardness field's caption.
        hardness: "paintBar.hardness" = "Hardness";
        /// The Opacity field's caption.
        opacity: "paintBar.opacity" = "Opacity";
        /// The Flow field's caption.
        flow: "paintBar.flow" = "Flow";
        /// The Mode field's caption.
        mode: "paintBar.mode" = "Mode";
        /// The Smoothing field's caption.
        smoothing: "paintBar.smoothing" = "Smoothing";
        /// The names of the mixer combinations, in [`MIXER_COMBINATIONS`](super::MIXER_COMBINATIONS) order.
        combinations: array [
        "paintBar.combination0" = "Custom",
        "paintBar.combination1" = "Dry, Light Load",
        "paintBar.combination2" = "Dry, Medium Load",
        "paintBar.combination3" = "Dry, Heavy Load",
        "paintBar.combination4" = "Moist, Light Load",
        "paintBar.combination5" = "Moist, Medium Load",
        "paintBar.combination6" = "Moist, Heavy Load",
        "paintBar.combination7" = "Wet, Light Load",
        "paintBar.combination8" = "Wet, Medium Load",
        "paintBar.combination9" = "Wet, Heavy Load",
        "paintBar.combination10" = "Very Wet, Light Load",
        "paintBar.combination11" = "Very Wet, Medium Load",
        "paintBar.combination12" = "Very Wet, Heavy Load"
        ];
    }
}
