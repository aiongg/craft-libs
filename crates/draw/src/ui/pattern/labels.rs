//! The pattern fill editor's words, English by default (Photoshop's and PowerPoint's, no colons).

craft_ui::words! {
    /// The words of [`PatternFillEditor`](super::PatternFillEditor) and
    /// [`PatternFillDialog`](super::PatternFillDialog).
    #[non_exhaustive]
    pub struct PatternFillLabels {
        title: "patternFill.title" = "Pattern Fill";
        pattern: "patternFill.pattern" = "Pattern";
        /// The pattern field's value for a screen reader: `{name}`.
        pattern_named: "patternFill.patternNamed" = "Pattern: {name}";
        no_patterns: "patternFill.noPatterns" = "No patterns";
        snap_to_origin: "patternFill.snapToOrigin" = "Snap to Origin";
        angle: "patternFill.angle" = "Angle";
        scale: "patternFill.scale" = "Scale";
        link: "patternFill.link" = "Link with Layer";
        foreground: "patternFill.foreground" = "Foreground";
        background: "patternFill.background" = "Background";
    }
}
