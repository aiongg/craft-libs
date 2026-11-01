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
        dashed: "strokeEditor.dashed" = "Dashed";
        dotted: "strokeEditor.dotted" = "Dotted";
        custom: "strokeEditor.custom" = "Custom";
        /// The opener's text while the stroke is off.
        no_color: "strokeEditor.noColor" = "No Color";
        alignment: "strokeEditor.alignment" = "Alignment";
        inside: "strokeEditor.inside" = "Inside";
        center: "strokeEditor.center" = "Center";
        outside: "strokeEditor.outside" = "Outside";
        caps: "strokeEditor.caps" = "Caps";
        butt: "strokeEditor.butt" = "Butt / Flat";
        round_cap: "strokeEditor.roundCap" = "Round";
        square: "strokeEditor.square" = "Square";
        corners: "strokeEditor.corners" = "Corners";
        miter: "strokeEditor.miter" = "Miter";
        round_join: "strokeEditor.roundJoin" = "Round";
        bevel: "strokeEditor.bevel" = "Bevel";
        miter_limit: "strokeEditor.miterLimit" = "Miter limit";
        opacity: "strokeEditor.opacity" = "Opacity";
        /// Under the rows of an open path.
        open_paths: "strokeEditor.openPaths" = "Open paths use centered strokes";
        dashed_line: "strokeEditor.dashedLine" = "Dashed line";
        /// Over the pairs, for `DashUnit::Widths`.
        dash_widths: "strokeEditor.dashWidths" = "Dash / gap lengths × stroke width";
        /// Over the pairs, for `DashUnit::Document`.
        dash_document: "strokeEditor.dashDocument" = "Dash / gap lengths in document units";
        dash: "strokeEditor.dash" = "Dash";
        gap: "strokeEditor.gap" = "Gap";
        add_pair: "strokeEditor.addPair" = "Add pair";
        remove_pair: "strokeEditor.removePair" = "Remove pair";
        dash_offset: "strokeEditor.dashOffset" = "Dash offset";
        /// Under the dash rows while the style fails `StrokeStyle::validate`.
        invalid: "strokeEditor.invalid" = "Use finite lengths and a positive pattern total";
    }
}
