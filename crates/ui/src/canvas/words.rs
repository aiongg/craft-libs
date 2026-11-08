//! Built-in words of the canvas family.

craft_ui::words! {
    /// The words of the canvas controls.
    pub struct CanvasWords {
        crop_group: "canvas.cropGroup" = "Crop frame";
        crop_whole: "canvas.cropWhole" = "Whole frame";
        crop_top_left: "canvas.cropTopLeft" = "Crop corner, top left";
        crop_top_right: "canvas.cropTopRight" = "Crop corner, top right";
        crop_bottom_right: "canvas.cropBottomRight" = "Crop corner, bottom right";
        crop_bottom_left: "canvas.cropBottomLeft" = "Crop corner, bottom left";
        crop_top_edge: "canvas.cropTopEdge" = "Crop edge, top";
        crop_right_edge: "canvas.cropRightEdge" = "Crop edge, right";
        crop_bottom_edge: "canvas.cropBottomEdge" = "Crop edge, bottom";
        crop_left_edge: "canvas.cropLeftEdge" = "Crop edge, left";
        crop_width: "canvas.cropWidth" = "W:";
        crop_height: "canvas.cropHeight" = "H:";
        crop_angle: "canvas.cropAngle" = "Angle:";
        // canvas-navigate-a
        rotate_compass: "canvas.rotateCompass" = "Rotation compass";

        ruler_horizontal: "canvas.rulerHorizontal" = "Horizontal ruler";
        ruler_vertical: "canvas.rulerVertical" = "Vertical ruler";
        ruler_origin: "canvas.rulerOrigin" = "Ruler origin";
        ruler_menu: "canvas.rulerMenu" = "Ruler units";
        ruler_reset_zero: "canvas.rulerResetZero" = "Reset Zero Point";

        scroll_horizontal: "canvas.scrollHorizontal" = "Horizontal scroll bar";
        scroll_vertical: "canvas.scrollVertical" = "Vertical scroll bar";

        // canvas-navigator-a
        navigator_horizontal: "canvas.navigatorHorizontal" = "Visible area horizontal position";
        navigator_vertical: "canvas.navigatorVertical" = "Visible area vertical position";
        navigator_zoom: "canvas.navigatorZoom" = "Zoom";
        // l4e-navpanel
        navigator_preview: "canvas.navigatorPreview" = "Document preview";
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
