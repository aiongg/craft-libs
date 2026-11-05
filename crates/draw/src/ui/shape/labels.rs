//! The words of the shape panel and the Shape tools' options bar, English by default
//! (PhotoCraft's, without colons; US spelling).

craft_ui::words! {
    /// The words of [`ShapePanel`](super::ShapePanel) and [`ShapeBar`](super::ShapeBar).
    #[non_exhaustive]
    pub struct ShapeLabels {
        /// The panel's accessible name.
        properties: "shape.properties" = "Shape properties";
        /// The Appearance section (the shape's look, not the app's theme).
        appearance: "shape.appearance" = "Appearance";
        /// The live shape section.
        shape: "shape.shape" = "Shape";
        #[allow(missing_docs)]
        fill: "shape.fill" = "Fill";
        #[allow(missing_docs)]
        stroke: "shape.stroke" = "Stroke";
        /// The fill chip's accessible name.
        fill_color: "shape.fillColor" = "Fill color";
        /// The stroke chip's accessible name.
        stroke_color: "shape.strokeColor" = "Stroke color";
        /// The stroke width field's accessible name.
        stroke_width: "shape.strokeWidth" = "Stroke width";
        /// The fill chip's tooltip.
        set_fill_type: "shape.setFillType" = "Set shape fill type";
        /// The stroke chip's tooltip.
        set_stroke_type: "shape.setStrokeType" = "Set shape stroke type";
        #[allow(missing_docs)]
        width: "shape.width" = "W";
        #[allow(missing_docs)]
        width_name: "shape.widthName" = "Width";
        #[allow(missing_docs)]
        height: "shape.height" = "H";
        #[allow(missing_docs)]
        height_name: "shape.heightName" = "Height";
        #[allow(missing_docs)]
        corner_radius: "shape.cornerRadius" = "Corner radius";
        #[allow(missing_docs)]
        top_left: "shape.topLeft" = "Top Left";
        #[allow(missing_docs)]
        top_right: "shape.topRight" = "Top Right";
        #[allow(missing_docs)]
        bottom_right: "shape.bottomRight" = "Bottom Right";
        #[allow(missing_docs)]
        bottom_left: "shape.bottomLeft" = "Bottom Left";
        #[allow(missing_docs)]
        sides: "shape.sides" = "Sides";
        #[allow(missing_docs)]
        star_ratio: "shape.starRatio" = "Star ratio";
        #[allow(missing_docs)]
        weight: "shape.weight" = "Weight";
        /// The options bar's corner radius cell.
        radius: "shape.radius" = "Radius";
    }
}
