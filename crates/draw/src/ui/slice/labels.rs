//! The words of the Slice tools, their bar and dialogs, English by default.

craft_ui::words! {
    /// What a screen reader and the status bar read for the Slice tools.
    #[non_exhaustive]
    pub struct SliceToolLabels {
        group: "sliceTool.group" = "Slices";
        /// A slice part: `{number}` two digits, `{origin}` one of the origin words, `{width}`,
        /// `{height}`, `{x}`, `{y}` in pixels.
        slice: "sliceTool.slice" = "Slice {number}, {origin}, {width} by {height} at {x}, {y}";
        user: "sliceTool.user" = "user slice";
        auto: "sliceTool.auto" = "auto slice";
        layer: "sliceTool.layer" = "layer slice";
        /// The status text of [`SliceRefusal::Locked`](super::SliceRefusal::Locked) (PhotoCraft's).
        locked: "sliceTool.locked" = "Slices are locked (View › Lock Slices)";
    }
}

craft_ui::words! {
    /// The words of [`SliceBar`](super::SliceBar), [`SliceOptionsDialog`](super::SliceOptionsDialog)
    /// and [`DivideSlicesDialog`](super::DivideSlicesDialog) (PhotoCraft's and Photoshop's, no
    /// colons; US spelling).
    #[non_exhaustive]
    pub struct SliceLabels {
        from_guides: "slice.fromGuides" = "Slices From Guides";
        promote: "slice.promote" = "Promote";
        divide: "slice.divide" = "Divide…";
        delete: "slice.delete" = "Delete";
        hide_auto: "slice.hideAuto" = "Hide Auto Slices";
        options: "slice.options" = "Slice Options…";
        options_title: "slice.optionsTitle" = "Slice Options";
        kind: "slice.kind" = "Slice Type";
        image: "slice.image" = "Image";
        no_image: "slice.noImage" = "No Image";
        html_text: "slice.htmlText" = "HTML Text";
        name: "slice.name" = "Name";
        url: "slice.url" = "URL";
        target: "slice.target" = "Target";
        message: "slice.message" = "Message Text";
        alt: "slice.alt" = "Alt Tag";
        cell_text: "slice.cellText" = "Text";
        background: "slice.background" = "Background";
        background_none: "slice.backgroundNone" = "None";
        divide_title: "slice.divideTitle" = "Divide Slice";
        horizontally: "slice.horizontally" = "Divide Horizontally Into";
        vertically: "slice.vertically" = "Divide Vertically Into";
        /// `{count}` slices across or down, evenly spaced.
        count: "slice.count" = "slices, evenly spaced";
        /// Pixels per slice.
        size: "slice.size" = "pixels per slice";
    }
}
