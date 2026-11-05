//! The picture fill editor's words, English by default (PowerPoint's, as DeckCraft copies them).

craft_ui::words! {
    /// The words of [`PictureFillEditor`](super::PictureFillEditor).
    #[non_exhaustive]
    pub struct PictureFillLabels {
        #[allow(missing_docs)]
        source: "pictureFill.source" = "Picture source";
        #[allow(missing_docs)]
        insert: "pictureFill.insert" = "Insert…";
        #[allow(missing_docs)]
        tile: "pictureFill.tile" = "Tile picture as texture";
        #[allow(missing_docs)]
        offset_x: "pictureFill.offsetX" = "Offset X";
        #[allow(missing_docs)]
        offset_y: "pictureFill.offsetY" = "Offset Y";
        #[allow(missing_docs)]
        scale_x: "pictureFill.scaleX" = "Scale X";
        #[allow(missing_docs)]
        scale_y: "pictureFill.scaleY" = "Scale Y";
        #[allow(missing_docs)]
        alignment: "pictureFill.alignment" = "Alignment";
        #[allow(missing_docs)]
        top_left: "pictureFill.topLeft" = "Top left";
        #[allow(missing_docs)]
        top: "pictureFill.top" = "Top";
        #[allow(missing_docs)]
        top_right: "pictureFill.topRight" = "Top right";
        #[allow(missing_docs)]
        left: "pictureFill.left" = "Left";
        #[allow(missing_docs)]
        center: "pictureFill.center" = "Center";
        #[allow(missing_docs)]
        right: "pictureFill.right" = "Right";
        #[allow(missing_docs)]
        bottom_left: "pictureFill.bottomLeft" = "Bottom left";
        #[allow(missing_docs)]
        bottom: "pictureFill.bottom" = "Bottom";
        #[allow(missing_docs)]
        bottom_right: "pictureFill.bottomRight" = "Bottom right";
        #[allow(missing_docs)]
        mirror: "pictureFill.mirror" = "Mirror type";
        #[allow(missing_docs)]
        mirror_none: "pictureFill.mirrorNone" = "None";
        #[allow(missing_docs)]
        horizontal: "pictureFill.horizontal" = "Horizontal";
        #[allow(missing_docs)]
        vertical: "pictureFill.vertical" = "Vertical";
        #[allow(missing_docs)]
        both: "pictureFill.both" = "Both";
        #[allow(missing_docs)]
        offset_left: "pictureFill.offsetLeft" = "Offset left";
        #[allow(missing_docs)]
        offset_right: "pictureFill.offsetRight" = "Offset right";
        #[allow(missing_docs)]
        offset_top: "pictureFill.offsetTop" = "Offset top";
        #[allow(missing_docs)]
        offset_bottom: "pictureFill.offsetBottom" = "Offset bottom";
        #[allow(missing_docs)]
        rotate: "pictureFill.rotate" = "Rotate with shape";
        #[allow(missing_docs)]
        transparency: "pictureFill.transparency" = "Transparency";
        /// A picked file that could not be read: `{name}`.
        unreadable: "pictureFill.unreadable" = "Could not read {name}";
    }
}
