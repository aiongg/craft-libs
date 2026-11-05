//! [`SelectMaskLabels`]: Select and Mask's words. The view controls' words are
//! `MaskViewLabels`; OK, Cancel and Reset the dialog's (`DialogLabels`).

use craft_ui::localized::Words;
use craft_ui::localized::words::Counted;
use egui::Context;

use super::session::LimitRefused;

pub(crate) const LIMIT_EDITS: Counted =
    Counted::new("selectMask.limitEdits", "at most {n} edit: undo some, or reset", "at most {n} edits: undo some, or reset");
pub(crate) const LIMIT_EDIT_POINTS: Counted = Counted::new(
    "selectMask.limitEditPoints",
    "at most {n} point in all edits: undo some, or reset",
    "at most {n} points in all edits: undo some, or reset",
);
pub(crate) const LIMIT_STROKES: Counted = Counted::new(
    "selectMask.limitStrokes",
    "at most {n} Refine Edge Brush stroke: undo some, or reset",
    "at most {n} Refine Edge Brush strokes: undo some, or reset",
);
pub(crate) const LIMIT_STROKE_POINTS: Counted = Counted::new(
    "selectMask.limitStrokePoints",
    "at most {n} point in all Refine Edge Brush strokes: undo some, or reset",
    "at most {n} points in all Refine Edge Brush strokes: undo some, or reset",
);

impl LimitRefused {
    /// The message under the panel, in the installed words and numbers of `ctx`.
    pub fn message(self, ctx: &Context) -> String {
        let (count, n) = match self {
            LimitRefused::Edits(n) => (LIMIT_EDITS, n),
            LimitRefused::EditPoints(n) => (LIMIT_EDIT_POINTS, n),
            LimitRefused::Strokes(n) => (LIMIT_STROKES, n),
            LimitRefused::StrokePoints(n) => (LIMIT_STROKE_POINTS, n),
        };
        let words = Words::of_ctx(ctx);
        words.counted(ctx, count, &words.get(count.word), n as u64)
    }
}

craft_ui::words! {
    /// Select and Mask's words, English by default (Photoshop's names).
    #[non_exhaustive]
    pub struct SelectMaskLabels {
        /// The title's name.
        name: "selectMask.name" = "Select and Mask";
        tools: "selectMask.tools" = "Tools";
        quick: "selectMask.quick" = "Quick Selection Tool";
        refine_edge: "selectMask.refineEdge" = "Refine Edge Brush Tool";
        brush: "selectMask.brush" = "Brush Tool";
        object: "selectMask.object" = "Object Selection Tool";
        lasso: "selectMask.lasso" = "Lasso Tool";
        polygonal_lasso: "selectMask.polygonalLasso" = "Polygonal Lasso Tool";
        hand: "selectMask.hand" = "Hand Tool";
        zoom: "selectMask.zoom" = "Zoom Tool";
        add: "selectMask.add" = "Add";
        subtract: "selectMask.subtract" = "Subtract";
        size: "selectMask.size" = "Size";
        hardness: "selectMask.hardness" = "Hardness";
        sample_all_layers: "selectMask.sampleAllLayers" = "Sample All Layers";
        select_subject: "selectMask.selectSubject" = "Select Subject";
        refine_hair: "selectMask.refineHair" = "Refine Hair";
        refine_hair_missing: "selectMask.refineHairMissing" = "Refine Hair needs hair detection, which isn't available yet";
        properties: "selectMask.properties" = "Properties";
        remember: "selectMask.remember" = "Remember Settings";
        view_mode: "selectMask.viewMode" = "View Mode";
        real_time: "selectMask.realTime" = "Real-Time Refinement";
        high_quality: "selectMask.highQuality" = "High Quality Preview";
        overlay_color: "selectMask.overlayColor" = "Color";
        indicates: "selectMask.indicates" = "Indicates";
        masked_areas: "selectMask.maskedAreas" = "Masked Areas";
        selected_areas: "selectMask.selectedAreas" = "Selected Areas";
        refine_mode: "selectMask.refineMode" = "Refine Mode";
        color_aware: "selectMask.colorAware" = "Color Aware";
        object_aware: "selectMask.objectAware" = "Object Aware";
        object_aware_missing: "selectMask.objectAwareMissing" = "Object Aware needs a segmentation model, which isn't available yet";
        edge_detection: "selectMask.edgeDetection" = "Edge Detection";
        radius: "selectMask.radius" = "Radius";
        smart_radius: "selectMask.smartRadius" = "Smart Radius";
        global_refinements: "selectMask.globalRefinements" = "Global Refinements";
        smooth: "selectMask.smooth" = "Smooth";
        feather: "selectMask.feather" = "Feather";
        contrast: "selectMask.contrast" = "Contrast";
        shift_edge: "selectMask.shiftEdge" = "Shift Edge";
        clear_selection: "selectMask.clearSelection" = "Clear Selection";
        invert: "selectMask.invert" = "Invert";
        output_settings: "selectMask.outputSettings" = "Output Settings";
        decontaminate: "selectMask.decontaminate" = "Decontaminate Colors";
        amount: "selectMask.amount" = "Amount";
        output_to: "selectMask.outputTo" = "Output To";
        output_selection: "selectMask.outputSelection" = "Selection";
        output_layer_mask: "selectMask.outputLayerMask" = "Layer Mask";
        output_new_layer: "selectMask.outputNewLayer" = "New Layer";
        output_new_layer_with_mask: "selectMask.outputNewLayerWithMask" = "New Layer with Layer Mask";
        output_new_document: "selectMask.outputNewDocument" = "New Document";
        output_new_document_over_mask: "selectMask.outputNewDocumentWithMask" = "New Document with Layer Mask";
    }
}
