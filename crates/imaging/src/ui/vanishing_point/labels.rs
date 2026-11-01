//! [`VanishingPointLabels`]: Vanishing Point's words.

craft_ui::words! {
    /// Vanishing Point's words, English by default (Photoshop's names).
    #[non_exhaustive]
    pub struct VanishingPointLabels {
        /// The title's name.
        name: "vanishingPoint.name" = "Vanishing Point";
        tools: "vanishingPoint.tools" = "Tools";
        edit_plane: "vanishingPoint.editPlane" = "Edit Plane Tool";
        create_plane: "vanishingPoint.createPlane" = "Create Plane Tool";
        stamp: "vanishingPoint.stamp" = "Stamp Tool";
        hand: "vanishingPoint.hand" = "Hand Tool";
        zoom: "vanishingPoint.zoom" = "Zoom Tool";
        diameter: "vanishingPoint.diameter" = "Diameter";
        hardness: "vanishingPoint.hardness" = "Hardness";
        opacity: "vanishingPoint.opacity" = "Opacity";
        aligned: "vanishingPoint.aligned" = "Aligned";
        grid_size: "vanishingPoint.gridSize" = "Grid Size";
        paste: "vanishingPoint.paste" = "Paste";
        width: "vanishingPoint.width" = "Width";
        new_layer: "vanishingPoint.newLayer" = "New Layer";
        /// A plane's corner as a keyboard part: `{n}` the plane's number, `{corner}` the corner.
        corner_part: "vanishingPoint.cornerPart" = "Plane {n}, {corner}";
        top_left: "vanishingPoint.topLeft" = "top-left corner";
        top_right: "vanishingPoint.topRight" = "top-right corner";
        bottom_right: "vanishingPoint.bottomRight" = "bottom-right corner";
        bottom_left: "vanishingPoint.bottomLeft" = "bottom-left corner";
        /// The canvas's description of a plane's check.
        questionable: "vanishingPoint.questionable" = "This plane's perspective is questionable";
        invalid: "vanishingPoint.invalid" = "This plane's perspective is invalid";
    }
}
