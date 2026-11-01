//! The words of the Artboard tool, its bar and the Properties section, English by default.

craft_ui::words! {
    /// What a screen reader reads for the Artboard tool's parts.
    #[non_exhaustive]
    pub struct ArtboardToolLabels {
        group: "artboardTool.group" = "Artboards";
        /// A board part: `{index}` from 1, `{width}`, `{height}`, `{x}`, `{y}` in pixels.
        board: "artboardTool.board" = "Artboard {index}, {width} by {height} at {x}, {y}";
        /// The "+" buttons' group.
        add_beside: "artboardTool.addBeside" = "Add artboard beside";
    }
}

craft_ui::words! {
    /// The words of [`ArtboardBar`](super::ArtboardBar) and
    /// [`ArtboardProperties`](super::ArtboardProperties) (PhotoCraft's, no colons).
    #[non_exhaustive]
    pub struct ArtboardLabels {
        artboard: "artboard.artboard" = "Artboard";
        size: "artboard.size" = "Size";
        preset: "artboard.preset" = "Preset";
        custom: "artboard.custom" = "Custom";
        width: "artboard.width" = "W";
        width_name: "artboard.widthName" = "Width";
        height: "artboard.height" = "H";
        height_name: "artboard.heightName" = "Height";
        x: "artboard.x" = "X";
        y: "artboard.y" = "Y";
        make_portrait: "artboard.makePortrait" = "Make Portrait";
        make_landscape: "artboard.makeLandscape" = "Make Landscape";
        add_new: "artboard.addNew" = "Add New Artboard";
        background: "artboard.background" = "Background";
        white: "artboard.white" = "White";
        black: "artboard.black" = "Black";
        transparent: "artboard.transparent" = "Transparent";
        other: "artboard.other" = "Other…";
        /// The colour picker's title for Other….
        background_color: "artboard.backgroundColor" = "Artboard Background Color";
    }
}
