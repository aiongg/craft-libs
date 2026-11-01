//! [`LiquifyLabels`]: Liquify's words. OK and Cancel are the dialog's (`DialogLabels`); the frame's
//! words are `FullWindowLabels`. The workspace reads both with `Labels::of_ctx`.

craft_ui::words! {
    /// Liquify's words, English by default (Photoshop's names).
    #[non_exhaustive]
    pub struct LiquifyLabels {
        /// The title's name: "Liquify (layer, 66%)".
        name: "liquify.name" = "Liquify";
        /// The strip's accessible name.
        tools: "liquify.tools" = "Tools";
        forward_warp: "liquify.forwardWarp" = "Forward Warp Tool";
        reconstruct_tool: "liquify.reconstructTool" = "Reconstruct Tool";
        smooth: "liquify.smooth" = "Smooth Tool";
        twirl_clockwise: "liquify.twirlClockwise" = "Twirl Clockwise Tool";
        twirl_counterclockwise: "liquify.twirlCounterclockwise" = "Twirl Counterclockwise Tool";
        pucker: "liquify.pucker" = "Pucker Tool";
        bloat: "liquify.bloat" = "Bloat Tool";
        push_left: "liquify.pushLeft" = "Push Left Tool";
        freeze: "liquify.freeze" = "Freeze Mask Tool";
        thaw: "liquify.thaw" = "Thaw Mask Tool";
        /// The Freeze Lasso's tooltip names the Alt rule.
        freeze_lasso: "liquify.freezeLasso" = "Freeze Lasso: drag to freeze an area, Alt-drag to thaw it";
        hand: "liquify.hand" = "Hand Tool";
        zoom: "liquify.zoom" = "Zoom Tool";
        /// The side column's accessible name.
        properties: "liquify.properties" = "Properties";
        brush_options: "liquify.brushOptions" = "Brush Tool Options";
        size: "liquify.size" = "Size";
        density: "liquify.density" = "Density";
        pressure: "liquify.pressure" = "Pressure";
        rate: "liquify.rate" = "Rate";
        reconstruct_options: "liquify.reconstructOptions" = "Brush Reconstruct Options";
        amount: "liquify.amount" = "Amount";
        reconstruct: "liquify.reconstruct" = "Reconstruct";
        restore_all: "liquify.restoreAll" = "Restore All";
        mask_options: "liquify.maskOptions" = "Mask Options";
        mask_none: "liquify.maskNone" = "None";
        mask_all: "liquify.maskAll" = "Mask All";
        invert_all: "liquify.invertAll" = "Invert All";
        view_options: "liquify.viewOptions" = "View Options";
        show_mesh: "liquify.showMesh" = "Show Mesh";
        mesh_size: "liquify.meshSize" = "Mesh Size";
        small: "liquify.small" = "Small";
        medium: "liquify.medium" = "Medium";
        large: "liquify.large" = "Large";
        show_mask: "liquify.showMask" = "Show Mask";
        show_backdrop: "liquify.showBackdrop" = "Show Backdrop";
        backdrop_use: "liquify.backdropUse" = "Use";
        all_layers: "liquify.allLayers" = "All Layers";
        backdrop_mode: "liquify.backdropMode" = "Mode";
        in_front: "liquify.inFront" = "In Front";
        behind: "liquify.behind" = "Behind";
        blend: "liquify.blend" = "Blend";
        opacity: "liquify.opacity" = "Opacity";
    }
}
