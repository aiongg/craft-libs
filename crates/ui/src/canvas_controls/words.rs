//! Built-in words of the canvas controls family.

// Guides and smart guides.
craft_ui::words! {
    /// The words of the guides.
    pub struct GuideWords {
        vertical: "guides.vertical" = "Vertical guide";
        horizontal: "guides.horizontal" = "Horizontal guide";
        x: "guides.x" = "X";
        y: "guides.y" = "Y";
    }
}

craft_ui::words! {
    /// The words of the selection outlines: the marquee's accessible description.
    pub struct SelectionWords {
        shape_rect: "marquee.shapeRect" = "Rectangle";
        shape_ellipse: "marquee.shapeEllipse" = "Ellipse";
        shape_polygon: "marquee.shapePolygon" = "Polygon";
        shape_polyline: "marquee.shapePolyline" = "Open path";
        shape_segments: "marquee.shapeSegments" = "Outline edges";
        bounds: template "marquee.bounds" = "{width} × {height} at {x}, {y}";
    }
}
