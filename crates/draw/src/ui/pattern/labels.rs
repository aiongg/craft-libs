//! The pattern fill editor's words, English by default (Photoshop's and PowerPoint's, no colons).

craft_ui::words! {
    /// The words of [`PatternFillEditor`](super::PatternFillEditor) and
    /// [`PatternFillDialog`](super::PatternFillDialog).
    #[non_exhaustive]
    pub struct PatternFillLabels {
        #[allow(missing_docs)]
        title: "patternFill.title" = "Pattern Fill";
        #[allow(missing_docs)]
        pattern: "patternFill.pattern" = "Pattern";
        /// The pattern field's value for a screen reader: `{name}`.
        pattern_named: "patternFill.patternNamed" = "Pattern: {name}";
        #[allow(missing_docs)]
        no_patterns: "patternFill.noPatterns" = "No patterns";
        #[allow(missing_docs)]
        snap_to_origin: "patternFill.snapToOrigin" = "Snap to Origin";
        #[allow(missing_docs)]
        angle: "patternFill.angle" = "Angle";
        #[allow(missing_docs)]
        scale: "patternFill.scale" = "Scale";
        #[allow(missing_docs)]
        link: "patternFill.link" = "Link with Layer";
        #[allow(missing_docs)]
        foreground: "patternFill.foreground" = "Foreground";
        #[allow(missing_docs)]
        background: "patternFill.background" = "Background";
    }
}
