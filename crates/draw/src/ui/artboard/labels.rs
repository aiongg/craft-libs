//! The words of the Artboard tool, its bar and the Properties section, English by default.

craft_ui::words! {
    /// What a screen reader reads for the Artboard tool's parts.
    #[non_exhaustive]
    pub struct ArtboardToolLabels {
        #[allow(missing_docs)]
        group: "artboardTool.group" = "Artboards";
        /// A board part: `{index}` from 1, `{width}`, `{height}`, `{x}`, `{y}` in pixels.
        board: "artboardTool.board" = "Artboard {index}, {width} by {height} at {x}, {y}";
        /// The "+" buttons' group.
        add_beside: "artboardTool.addBeside" = "Add artboard beside";
        /// A handle part's names, one per handle of the active board.
        handle_top_left: "artboardTool.handleTopLeft" = "Top left handle";
        #[allow(missing_docs)]
        handle_top: "artboardTool.handleTop" = "Top handle";
        #[allow(missing_docs)]
        handle_top_right: "artboardTool.handleTopRight" = "Top right handle";
        #[allow(missing_docs)]
        handle_right: "artboardTool.handleRight" = "Right handle";
        #[allow(missing_docs)]
        handle_bottom_right: "artboardTool.handleBottomRight" = "Bottom right handle";
        #[allow(missing_docs)]
        handle_bottom: "artboardTool.handleBottom" = "Bottom handle";
        #[allow(missing_docs)]
        handle_bottom_left: "artboardTool.handleBottomLeft" = "Bottom left handle";
        #[allow(missing_docs)]
        handle_left: "artboardTool.handleLeft" = "Left handle";
        /// A handle part's value: `{x}`, `{y}` in pixels.
        handle_at: "artboardTool.handleAt" = "{x}, {y}";
    }
}

craft_ui::words! {
    /// The words of [`ArtboardBar`](super::ArtboardBar) and
    /// [`ArtboardProperties`](super::ArtboardProperties) (PhotoCraft's, no colons).
    #[non_exhaustive]
    pub struct ArtboardLabels {
        #[allow(missing_docs)]
        artboard: "artboard.artboard" = "Artboard";
        #[allow(missing_docs)]
        size: "artboard.size" = "Size";
        #[allow(missing_docs)]
        preset: "artboard.preset" = "Preset";
        #[allow(missing_docs)]
        custom: "artboard.custom" = "Custom";
        #[allow(missing_docs)]
        width: "artboard.width" = "W";
        #[allow(missing_docs)]
        width_name: "artboard.widthName" = "Width";
        #[allow(missing_docs)]
        height: "artboard.height" = "H";
        #[allow(missing_docs)]
        height_name: "artboard.heightName" = "Height";
        #[allow(missing_docs)]
        x: "artboard.x" = "X";
        #[allow(missing_docs)]
        y: "artboard.y" = "Y";
        #[allow(missing_docs)]
        make_portrait: "artboard.makePortrait" = "Make Portrait";
        #[allow(missing_docs)]
        make_landscape: "artboard.makeLandscape" = "Make Landscape";
        #[allow(missing_docs)]
        add_new: "artboard.addNew" = "Add New Artboard";
        #[allow(missing_docs)]
        background: "artboard.background" = "Background";
        #[allow(missing_docs)]
        white: "artboard.white" = "White";
        #[allow(missing_docs)]
        black: "artboard.black" = "Black";
        #[allow(missing_docs)]
        transparent: "artboard.transparent" = "Transparent";
        #[allow(missing_docs)]
        other: "artboard.other" = "Other…";
        /// The color picker's title for Other….
        background_color: "artboard.backgroundColor" = "Artboard Background Color";
    }
}
