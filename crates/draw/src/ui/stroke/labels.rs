//! The words of the stroke editor and its button ([`StrokeLabels`]).

craft_ui::words! {
    /// The words of [`StrokeEditor`](super::StrokeEditor) and [`StrokeButton`](super::StrokeButton),
    /// English by default (PhotoCraft's and Photoshop's).
    #[non_exhaustive]
    pub struct StrokeLabels {
        /// The popover's title, the opener's accessible name and tooltip.
        title: "strokeEditor.title" = "Stroke Options";
        /// The presets and the opener's value.
        solid: "strokeEditor.solid" = "Solid";
        #[allow(missing_docs)]
        dashed: "strokeEditor.dashed" = "Dashed";
        #[allow(missing_docs)]
        dotted: "strokeEditor.dotted" = "Dotted";
        #[allow(missing_docs)]
        custom: "strokeEditor.custom" = "Custom";
        /// The opener's text while the stroke is off.
        no_color: "strokeEditor.noColor" = "No Color";
        #[allow(missing_docs)]
        alignment: "strokeEditor.alignment" = "Alignment";
        #[allow(missing_docs)]
        inside: "strokeEditor.inside" = "Inside";
        #[allow(missing_docs)]
        center: "strokeEditor.center" = "Center";
        #[allow(missing_docs)]
        outside: "strokeEditor.outside" = "Outside";
        #[allow(missing_docs)]
        caps: "strokeEditor.caps" = "Caps";
        #[allow(missing_docs)]
        butt: "strokeEditor.butt" = "Butt / Flat";
        #[allow(missing_docs)]
        round_cap: "strokeEditor.roundCap" = "Round";
        #[allow(missing_docs)]
        square: "strokeEditor.square" = "Square";
        #[allow(missing_docs)]
        corners: "strokeEditor.corners" = "Corners";
        #[allow(missing_docs)]
        miter: "strokeEditor.miter" = "Miter";
        #[allow(missing_docs)]
        round_join: "strokeEditor.roundJoin" = "Round";
        #[allow(missing_docs)]
        bevel: "strokeEditor.bevel" = "Bevel";
        #[allow(missing_docs)]
        miter_limit: "strokeEditor.miterLimit" = "Miter limit";
        #[allow(missing_docs)]
        opacity: "strokeEditor.opacity" = "Opacity";
        /// Under the rows of an open path.
        open_paths: "strokeEditor.openPaths" = "Open paths use centered strokes";
        #[allow(missing_docs)]
        dashed_line: "strokeEditor.dashedLine" = "Dashed line";
        /// Over the pairs, for `DashUnit::Widths`.
        dash_widths: "strokeEditor.dashWidths" = "Dash / gap lengths × stroke width";
        /// Over the pairs, for `DashUnit::Document`.
        dash_document: "strokeEditor.dashDocument" = "Dash / gap lengths in document units";
        #[allow(missing_docs)]
        dash: "strokeEditor.dash" = "Dash";
        #[allow(missing_docs)]
        gap: "strokeEditor.gap" = "Gap";
        #[allow(missing_docs)]
        add_pair: "strokeEditor.addPair" = "Add pair";
        #[allow(missing_docs)]
        remove_pair: "strokeEditor.removePair" = "Remove pair";
        #[allow(missing_docs)]
        dash_offset: "strokeEditor.dashOffset" = "Dash offset";
        /// Under the dash rows while the style fails `StrokeStyle::validate`.
        invalid: "strokeEditor.invalid" = "Use finite lengths and a positive pattern total";
    }
}
