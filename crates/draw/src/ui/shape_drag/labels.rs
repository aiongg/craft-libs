//! The words of the Shape tools and the Create Shape dialog, English by default.

craft_ui::words! {
    /// What a screen reader and the status bar read for a Shape tool. The tool has no parts; its
    /// keyboard route is Enter, which reports a click at the view centre.
    #[non_exhaustive]
    pub struct ShapeDragNames {
        /// The canvas value while a shape is being drawn: `{width}` and `{height}` in the ruler unit.
        drawing: "shapeDrag.drawing" = "Drawing {width} by {height}";
        /// The status hint while a Shape tool is active.
        hint: "shapeDrag.hint" = "Drag to draw · Click or press Enter to set a size";
    }
}

craft_ui::words! {
    /// The Create Shape dialog's words (PhotoCraft's, without colons; US spelling).
    #[non_exhaustive]
    pub struct CreateShapeLabels {
        rectangle: "createShape.rectangle" = "Create Rectangle";
        rounded_rectangle: "createShape.roundedRectangle" = "Create Rounded Rectangle";
        ellipse: "createShape.ellipse" = "Create Ellipse";
        triangle: "createShape.triangle" = "Create Triangle";
        polygon: "createShape.polygon" = "Create Polygon";
        star: "createShape.star" = "Create Star";
        width: "createShape.width" = "Width";
        height: "createShape.height" = "Height";
        radius: "createShape.radius" = "Radius";
        top_left: "createShape.topLeft" = "Top Left";
        top_right: "createShape.topRight" = "Top Right";
        bottom_right: "createShape.bottomRight" = "Bottom Right";
        bottom_left: "createShape.bottomLeft" = "Bottom Left";
        link_radii: "createShape.linkRadii" = "Link radii";
        sides: "createShape.sides" = "Sides";
        star_ratio: "createShape.starRatio" = "Star Ratio";
        from_center: "createShape.fromCenter" = "From Center";
        /// A width or height outside 1–300,000 px; `{field}` is the field's label, `{max}` the limit.
        bad_size: "createShape.badSize" = "{field} must be between 1 and {max} pixels";
    }
}
