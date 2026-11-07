//! Built-in words of the canvas family.

craft_ui::words! {
    /// The words of the canvas controls.
    pub struct CanvasWords {
        crop_top_left: "canvas.cropTopLeft" = "Crop corner, top left";
        crop_top_right: "canvas.cropTopRight" = "Crop corner, top right";
        crop_bottom_left: "canvas.cropBottomLeft" = "Crop corner, bottom left";
        crop_bottom_right: "canvas.cropBottomRight" = "Crop corner, bottom right";
        // canvas-navigate-a
        rotate_compass: "canvas.rotateCompass" = "Rotation compass";

        ruler_horizontal: "canvas.rulerHorizontal" = "Horizontal ruler";
        ruler_vertical: "canvas.rulerVertical" = "Vertical ruler";
        ruler_origin: "canvas.rulerOrigin" = "Ruler origin";
        ruler_menu: "canvas.rulerMenu" = "Ruler units";
        ruler_reset_zero: "canvas.rulerResetZero" = "Reset Zero Point";

        scroll_horizontal: "canvas.scrollHorizontal" = "Horizontal scroll bar";
        scroll_vertical: "canvas.scrollVertical" = "Vertical scroll bar";
    }
}

craft_ui::words! {
    /// The words of the split views.
    pub struct SplitWords {
        /// A pane's accessible name. `{n}` stands for its number, counted from 1 in reading order.
        view: template "canvas.splitView" = "View {n}";
        /// The name of a divider between panes.
        resize: "canvas.splitResize" = "Resize views";
    }
}
