//! [`WideAngleLabels`]: Adaptive Wide Angle's words.

craft_ui::words! {
    /// Adaptive Wide Angle's words, English by default (Photoshop's names).
    #[non_exhaustive]
    pub struct WideAngleLabels {
        /// The title's name.
        name: "wideAngle.name" = "Adaptive Wide Angle";
        tools: "wideAngle.tools" = "Tools";
        constraint_tool: "wideAngle.constraintTool" = "Constraint Tool";
        hand: "wideAngle.hand" = "Hand Tool";
        zoom: "wideAngle.zoom" = "Zoom Tool";
        settings: "wideAngle.settings" = "Settings";
        correction: "wideAngle.correction" = "Correction";
        auto: "wideAngle.auto" = "Auto";
        fisheye: "wideAngle.fisheye" = "Fisheye";
        perspective: "wideAngle.perspective" = "Perspective";
        full_spherical: "wideAngle.fullSpherical" = "Full Spherical";
        scale: "wideAngle.scale" = "Scale";
        focal_length: "wideAngle.focalLength" = "Focal Length";
        crop_factor: "wideAngle.cropFactor" = "Crop Factor";
        orientation: "wideAngle.orientation" = "Orientation";
        free: "wideAngle.free" = "Free";
        horizontal: "wideAngle.horizontal" = "Horizontal";
        vertical: "wideAngle.vertical" = "Vertical";
        delete: "wideAngle.delete" = "Delete";
        preview: "wideAngle.preview" = "Preview";
        hint: "wideAngle.hint" = "Drag on the image to add a constraint; Shift for horizontal/vertical; right-click a constraint to change or delete it.";
        /// A constraint as a keyboard part: `{n}` its number, `{orientation}` its orientation.
        constraint_part: "wideAngle.constraintPart" = "Constraint {n}, {orientation}";
    }
}

/// "Constraints (N)": the list's header.
#[expect(dead_code, reason = "skeleton: the panel (task WS-WA1) reads it; RG1 registers it")]
pub(crate) const CONSTRAINTS: craft_ui::localized::words::Counted =
    craft_ui::localized::words::Counted::new("wideAngle.constraints", "Constraints ({n})", "Constraints ({n})");
