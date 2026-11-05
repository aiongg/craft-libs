//! The picture fill editor's words, English by default (PowerPoint's, as DeckCraft copies them).

craft_ui::words! {
    /// The words of [`PictureFillEditor`](super::PictureFillEditor).
    #[non_exhaustive]
    pub struct PictureFillLabels {
        source: "pictureFill.source" = "Picture source";
        insert: "pictureFill.insert" = "Insert…";
        tile: "pictureFill.tile" = "Tile picture as texture";
        offset_x: "pictureFill.offsetX" = "Offset X";
        offset_y: "pictureFill.offsetY" = "Offset Y";
        scale_x: "pictureFill.scaleX" = "Scale X";
        scale_y: "pictureFill.scaleY" = "Scale Y";
        alignment: "pictureFill.alignment" = "Alignment";
        top_left: "pictureFill.topLeft" = "Top left";
        top: "pictureFill.top" = "Top";
        top_right: "pictureFill.topRight" = "Top right";
        left: "pictureFill.left" = "Left";
        center: "pictureFill.center" = "Center";
        right: "pictureFill.right" = "Right";
        bottom_left: "pictureFill.bottomLeft" = "Bottom left";
        bottom: "pictureFill.bottom" = "Bottom";
        bottom_right: "pictureFill.bottomRight" = "Bottom right";
        mirror: "pictureFill.mirror" = "Mirror type";
        mirror_none: "pictureFill.mirrorNone" = "None";
        horizontal: "pictureFill.horizontal" = "Horizontal";
        vertical: "pictureFill.vertical" = "Vertical";
        both: "pictureFill.both" = "Both";
        offset_left: "pictureFill.offsetLeft" = "Offset left";
        offset_right: "pictureFill.offsetRight" = "Offset right";
        offset_top: "pictureFill.offsetTop" = "Offset top";
        offset_bottom: "pictureFill.offsetBottom" = "Offset bottom";
        rotate: "pictureFill.rotate" = "Rotate with shape";
        transparency: "pictureFill.transparency" = "Transparency";
        /// A picked file that could not be read: `{name}`.
        unreadable: "pictureFill.unreadable" = "Could not read {name}";
    }
}
