//! The words of the Shape tools and the Create Shape dialog, English by default.

craft_ui::words! {
    /// What a screen reader and the status bar read for a Shape tool. The tool has no parts; its
    /// keyboard route is Enter, which reports a click at the view centre.
    #[non_exhaustive]
    pub struct ShapeDragToolLabels {
        /// The readout's first row label: the shape's width.
        width: "shapeDragTool.width" = "Width";
        /// The readout's second row label: the shape's height.
        height: "shapeDragTool.height" = "Height";
        /// The canvas value while a shape is being drawn: `{width}` and `{height}` in the ruler unit.
        drawing: "shapeDragTool.drawing" = "Drawing {width} by {height}";
        /// The status hint while a Shape tool is active.
        hint: "shapeDragTool.hint" = "Drag to draw · Click or press Enter to set a size";
    }
}

craft_ui::words! {
    /// The Create Shape dialog's words (PhotoCraft's, without colons; US spelling).
    #[non_exhaustive]
    pub struct CreateShapeLabels {
        #[allow(missing_docs)]
        rectangle: "createShape.rectangle" = "Create Rectangle";
        #[allow(missing_docs)]
        rounded_rectangle: "createShape.roundedRectangle" = "Create Rounded Rectangle";
        #[allow(missing_docs)]
        ellipse: "createShape.ellipse" = "Create Ellipse";
        #[allow(missing_docs)]
        triangle: "createShape.triangle" = "Create Triangle";
        #[allow(missing_docs)]
        polygon: "createShape.polygon" = "Create Polygon";
        #[allow(missing_docs)]
        star: "createShape.star" = "Create Star";
        #[allow(missing_docs)]
        width: "createShape.width" = "Width";
        #[allow(missing_docs)]
        height: "createShape.height" = "Height";
        #[allow(missing_docs)]
        radius: "createShape.radius" = "Radius";
        #[allow(missing_docs)]
        top_left: "createShape.topLeft" = "Top Left";
        #[allow(missing_docs)]
        top_right: "createShape.topRight" = "Top Right";
        #[allow(missing_docs)]
        bottom_right: "createShape.bottomRight" = "Bottom Right";
        #[allow(missing_docs)]
        bottom_left: "createShape.bottomLeft" = "Bottom Left";
        #[allow(missing_docs)]
        link_radii: "createShape.linkRadii" = "Link radii";
        #[allow(missing_docs)]
        sides: "createShape.sides" = "Sides";
        #[allow(missing_docs)]
        star_ratio: "createShape.starRatio" = "Star Ratio";
        #[allow(missing_docs)]
        from_center: "createShape.fromCenter" = "From Center";
        /// The click lies more than 300,000 px from the origin on an axis.
        bad_position: "createShape.badPosition" = "The shape's position is out of range";
        /// A width or height outside 1–300,000 px; `{field}` is the field's label, `{max}` the limit.
        bad_size: "createShape.badSize" = "{field} must be between 1 and {max} pixels";
    }
}
