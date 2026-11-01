//! [`FillLabels`]: Content-Aware Fill's words. OK, Apply, Cancel and Reset are the dialog's
//! (`DialogLabels`).

craft_ui::words! {
    /// Content-Aware Fill's words, English by default (Photoshop's names).
    #[non_exhaustive]
    pub struct FillLabels {
        /// The title's name and the settings column's header.
        name: "fill.name" = "Content-Aware Fill";
        tools: "fill.tools" = "Tools";
        sampling_brush: "fill.samplingBrush" = "Sampling Brush Tool";
        lasso: "fill.lasso" = "Lasso Tool";
        hand: "fill.hand" = "Hand Tool";
        zoom: "fill.zoom" = "Zoom Tool";
        add: "fill.add" = "Add";
        subtract: "fill.subtract" = "Subtract";
        size: "fill.size" = "Size";
        new_selection: "fill.newSelection" = "New selection";
        /// `{key}` is the platform's name for Shift.
        add_to_selection: "fill.addToSelection" = "Add to selection ({key})";
        /// `{key}` is the platform's name for Alt.
        subtract_from_selection: "fill.subtractFromSelection" = "Subtract from selection ({key})";
        /// `{key}` is the platform's names for Shift+Alt.
        intersect_with_selection: "fill.intersectWithSelection" = "Intersect with selection ({key})";
        expand: "fill.expand" = "Expand";
        contract: "fill.contract" = "Contract";
        preview: "fill.preview" = "Preview";
        sampling_overlay: "fill.samplingOverlay" = "Sampling Area Overlay";
        show_sampling_area: "fill.showSamplingArea" = "Show Sampling Area";
        opacity: "fill.opacity" = "Opacity";
        color: "fill.color" = "Color";
        indicates: "fill.indicates" = "Indicates";
        sampling_area: "fill.samplingArea" = "Sampling Area";
        excluded_area: "fill.excludedArea" = "Excluded Area";
        sampling_options: "fill.samplingOptions" = "Sampling Area Options";
        auto: "fill.auto" = "Auto";
        rectangular: "fill.rectangular" = "Rectangular";
        custom: "fill.custom" = "Custom";
        sample_all_layers: "fill.sampleAllLayers" = "Sample All Layers";
        fill_settings: "fill.fillSettings" = "Fill Settings";
        color_adaptation: "fill.colorAdaptation" = "Color Adaptation";
        rotation_adaptation: "fill.rotationAdaptation" = "Rotation Adaptation";
        none: "fill.none" = "None";
        default: "fill.default" = "Default";
        low: "fill.low" = "Low";
        medium: "fill.medium" = "Medium";
        high: "fill.high" = "High";
        very_high: "fill.veryHigh" = "Very High";
        full: "fill.full" = "Full";
        scale: "fill.scale" = "Scale";
        mirror: "fill.mirror" = "Mirror";
        output_settings: "fill.outputSettings" = "Output Settings";
        output_to: "fill.outputTo" = "Output To";
        current_layer: "fill.currentLayer" = "Current Layer";
        new_layer: "fill.newLayer" = "New Layer";
        duplicate_layer: "fill.duplicateLayer" = "Duplicate Layer";
    }
}
