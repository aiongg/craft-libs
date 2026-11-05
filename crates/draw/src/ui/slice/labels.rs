//! The words of the Slice tools, their bar and dialogs, English by default.

craft_ui::words! {
    /// What a screen reader and the status bar read for the Slice tools.
    #[non_exhaustive]
    pub struct SliceToolLabels {
        #[allow(missing_docs)]
        group: "sliceTool.group" = "Slices";
        /// A slice part: `{number}` two digits, `{origin}` one of the origin words, `{width}`,
        /// `{height}`, `{x}`, `{y}` in pixels.
        slice: "sliceTool.slice" = "Slice {number}, {origin}, {width} by {height} at {x}, {y}";
        #[allow(missing_docs)]
        user: "sliceTool.user" = "user slice";
        #[allow(missing_docs)]
        auto: "sliceTool.auto" = "auto slice";
        #[allow(missing_docs)]
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
        #[allow(missing_docs)]
        from_guides: "slice.fromGuides" = "Slices From Guides";
        #[allow(missing_docs)]
        promote: "slice.promote" = "Promote";
        #[allow(missing_docs)]
        divide: "slice.divide" = "Divide…";
        #[allow(missing_docs)]
        delete: "slice.delete" = "Delete";
        #[allow(missing_docs)]
        hide_auto: "slice.hideAuto" = "Hide Auto Slices";
        #[allow(missing_docs)]
        options: "slice.options" = "Slice Options…";
        #[allow(missing_docs)]
        options_title: "slice.optionsTitle" = "Slice Options";
        #[allow(missing_docs)]
        kind: "slice.kind" = "Slice Type";
        #[allow(missing_docs)]
        image: "slice.image" = "Image";
        #[allow(missing_docs)]
        no_image: "slice.noImage" = "No Image";
        #[allow(missing_docs)]
        html_text: "slice.htmlText" = "HTML Text";
        #[allow(missing_docs)]
        name: "slice.name" = "Name";
        #[allow(missing_docs)]
        url: "slice.url" = "URL";
        #[allow(missing_docs)]
        target: "slice.target" = "Target";
        #[allow(missing_docs)]
        message: "slice.message" = "Message Text";
        #[allow(missing_docs)]
        alt: "slice.alt" = "Alt Tag";
        #[allow(missing_docs)]
        cell_text: "slice.cellText" = "Text";
        #[allow(missing_docs)]
        background: "slice.background" = "Background";
        #[allow(missing_docs)]
        background_none: "slice.backgroundNone" = "None";
        #[allow(missing_docs)]
        divide_title: "slice.divideTitle" = "Divide Slice";
        #[allow(missing_docs)]
        horizontally: "slice.horizontally" = "Divide Horizontally Into";
        #[allow(missing_docs)]
        vertically: "slice.vertically" = "Divide Vertically Into";
        /// `{count}` slices across or down, evenly spaced.
        count: "slice.count" = "slices, evenly spaced";
        /// Pixels per slice.
        size: "slice.size" = "pixels per slice";
    }
}
