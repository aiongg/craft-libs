//! The words of the Layer Style dialog, the Blend If bar and the contour field.

use crate::layer_style::EffectKind;

craft_ui::words! {
    /// Every word of the Layer Style dialog, English by default: pages, effect names, field
    /// labels, choice labels and the Blending Options page. The built-in tables
    /// ([`EffectSpec::photoshop`](crate::layer_style::EffectSpec::photoshop)) are shown through
    /// it ([`field`](Self::field), [`choice`](Self::choice), [`effect`](Self::effect)); an
    /// app's own specs show their own labels.
    // Every field is one self-evident word; its English text is the default.
    #[allow(missing_docs)]
    #[non_exhaustive]
    pub struct LayerStyleLabels {
        /// The effect list's accessible name.
        effects: "layerStyle.effects" = "Effects";
        styles: "layerStyle.styles" = "Styles";
        blending_options: "layerStyle.blendingOptions" = "Blending Options";
        bevel_emboss: "layerStyle.effect.bevelEmboss" = "Bevel & Emboss";
        stroke: "layerStyle.effect.stroke" = "Stroke";
        inner_shadow: "layerStyle.effect.innerShadow" = "Inner Shadow";
        inner_glow: "layerStyle.effect.innerGlow" = "Inner Glow";
        satin: "layerStyle.effect.satin" = "Satin";
        color_overlay: "layerStyle.effect.colorOverlay" = "Color Overlay";
        gradient_overlay: "layerStyle.effect.gradientOverlay" = "Gradient Overlay";
        pattern_overlay: "layerStyle.effect.patternOverlay" = "Pattern Overlay";
        outer_glow: "layerStyle.effect.outerGlow" = "Outer Glow";
        drop_shadow: "layerStyle.effect.dropShadow" = "Drop Shadow";
        /// The `+` button's accessible name; `{effect}` is the effect's name.
        add_instance: "layerStyle.addInstance" = "Add another {effect}";
        /// The `−` button's accessible name.
        remove_instance: "layerStyle.removeInstance" = "Remove this {effect}";
        /// The name of the instance `{number}` of an effect that has several: "Drop Shadow 2".
        instance_name: "layerStyle.instanceName" = "{effect} {number}";
        preview: "layerStyle.preview" = "Preview";
        new_style: "layerStyle.newStyle" = "New Style…";
        make_default: "layerStyle.makeDefault" = "Make Default";
        reset_to_default: "layerStyle.resetToDefault" = "Reset to Default";
        /// The colour picker's title for an effect colour.
        color_picker_title: "layerStyle.colorPickerTitle" = "Color Picker (Layer Style Color)";
        /// One line over a page whose host values have the wrong type; `{field}` names the first.
        invalid_value: "layerStyle.invalidValue" = "{field} has a value this page can't show. It is kept as it is.";
        blend_mode: "layerStyle.field.blend" = "Blend Mode";
        color: "layerStyle.field.color" = "Color";
        opacity: "layerStyle.field.opacity" = "Opacity";
        fill_opacity: "layerStyle.field.fillOpacity" = "Fill Opacity";
        angle: "layerStyle.field.angle" = "Angle";
        use_global_light: "layerStyle.field.useGlobalLight" = "Use Global Light";
        distance: "layerStyle.field.distance" = "Distance";
        spread: "layerStyle.field.spread" = "Spread";
        choke: "layerStyle.field.choke" = "Choke";
        size: "layerStyle.field.size" = "Size";
        contour: "layerStyle.field.contour" = "Contour";
        noise: "layerStyle.field.noise" = "Noise";
        knocks_out: "layerStyle.field.knocksOut" = "Layer Knocks Out Drop Shadow";
        technique: "layerStyle.field.technique" = "Technique";
        source: "layerStyle.field.source" = "Source";
        range: "layerStyle.field.range" = "Range";
        position: "layerStyle.field.position" = "Position";
        overprint: "layerStyle.field.overprint" = "Overprint";
        from: "layerStyle.field.from" = "From";
        to: "layerStyle.field.to" = "To";
        reverse: "layerStyle.field.reverse" = "Reverse";
        style: "layerStyle.field.style" = "Style";
        scale: "layerStyle.field.scale" = "Scale";
        method: "layerStyle.field.method" = "Method";
        pattern: "layerStyle.field.pattern" = "Pattern";
        link: "layerStyle.field.link" = "Link with Layer";
        depth: "layerStyle.field.depth" = "Depth";
        direction: "layerStyle.field.direction" = "Direction";
        soften: "layerStyle.field.soften" = "Soften";
        altitude: "layerStyle.field.altitude" = "Altitude";
        gloss_contour: "layerStyle.field.glossContour" = "Gloss Contour";
        highlight_mode: "layerStyle.field.highlightBlend" = "Highlight Mode";
        highlight_color: "layerStyle.field.highlightColor" = "Highlight Color";
        highlight_opacity: "layerStyle.field.highlightOpacity" = "Highlight Opacity";
        shadow_mode: "layerStyle.field.shadowBlend" = "Shadow Mode";
        shadow_color: "layerStyle.field.shadowColor" = "Shadow Color";
        shadow_opacity: "layerStyle.field.shadowOpacity" = "Shadow Opacity";
        invert: "layerStyle.field.invert" = "Invert";
        softer: "layerStyle.choice.softer" = "Softer";
        precise: "layerStyle.choice.precise" = "Precise";
        edge: "layerStyle.choice.edge" = "Edge";
        center: "layerStyle.choice.center" = "Center";
        outside: "layerStyle.choice.outside" = "Outside";
        inside: "layerStyle.choice.inside" = "Inside";
        linear: "layerStyle.choice.linear" = "Linear";
        radial: "layerStyle.choice.radial" = "Radial";
        angle_style: "layerStyle.choice.angle" = "Angle";
        reflected: "layerStyle.choice.reflected" = "Reflected";
        diamond: "layerStyle.choice.diamond" = "Diamond";
        inner_bevel: "layerStyle.choice.inner" = "Inner Bevel";
        outer_bevel: "layerStyle.choice.outer" = "Outer Bevel";
        emboss: "layerStyle.choice.emboss" = "Emboss";
        pillow_emboss: "layerStyle.choice.pillow" = "Pillow Emboss";
        smooth: "layerStyle.choice.smooth" = "Smooth";
        chisel_hard: "layerStyle.choice.chiselHard" = "Chisel Hard";
        chisel_soft: "layerStyle.choice.chiselSoft" = "Chisel Soft";
        up: "layerStyle.choice.up" = "Up";
        down: "layerStyle.choice.down" = "Down";
        perceptual: "layerStyle.choice.perceptual" = "Perceptual";
        classic: "layerStyle.choice.classic" = "Classic";
        custom: "layerStyle.choice.custom" = "Custom";
        general_blending: "layerStyle.generalBlending" = "General Blending";
        advanced_blending: "layerStyle.advancedBlending" = "Advanced Blending";
        channels: "layerStyle.channels" = "Channels:";
        knockout: "layerStyle.knockout" = "Knockout";
        knockout_none: "layerStyle.knockoutNone" = "None";
        knockout_shallow: "layerStyle.knockoutShallow" = "Shallow";
        knockout_deep: "layerStyle.knockoutDeep" = "Deep";
        blend_interior_effects: "layerStyle.blendInteriorEffectsAsGroup" = "Blend Interior Effects as Group";
        blend_clipped_layers: "layerStyle.blendClippedLayersAsGroup" = "Blend Clipped Layers as Group";
        transparency_shapes_layer: "layerStyle.transparencyShapesLayer" = "Transparency Shapes Layer";
        layer_mask_hides_effects: "layerStyle.layerMaskHidesEffects" = "Layer Mask Hides Effects";
        vector_mask_hides_effects: "layerStyle.vectorMaskHidesEffects" = "Vector Mask Hides Effects";
        blend_if: "layerStyle.blendIf" = "Blend If:";
        /// The Blend If entry of a colour document's composite.
        gray: "layerStyle.gray" = "Gray";
        this_layer: "layerStyle.thisLayer" = "This Layer";
        underlying_layer: "layerStyle.underlyingLayer" = "Underlying Layer";
    }
}

impl LayerStyleLabels {
    /// The name of `kind` ("Drop Shadow").
    pub fn effect(&self, kind: EffectKind) -> &str {
        // pc: crates/ui-egui/src/dialogs/layer_style.rs:53-64 `KINDS` (the second column).
        match kind {
            EffectKind::BevelEmboss => &self.bevel_emboss,
            EffectKind::Stroke => &self.stroke,
            EffectKind::InnerShadow => &self.inner_shadow,
            EffectKind::InnerGlow => &self.inner_glow,
            EffectKind::Satin => &self.satin,
            EffectKind::ColorOverlay => &self.color_overlay,
            EffectKind::GradientOverlay => &self.gradient_overlay,
            EffectKind::PatternOverlay => &self.pattern_overlay,
            EffectKind::OuterGlow => &self.outer_glow,
            EffectKind::DropShadow => &self.drop_shadow,
        }
    }

    /// The label of a built-in field key (`"distance"` → "Distance"); `None` for a key the
    /// built-in tables don't have, so the page shows the spec's own label. Bevel & Emboss's
    /// `contour` check is "Contour", its `contourName` row has no label, `contourRange` is
    /// "Range".
    pub fn field(&self, key: &str) -> Option<&str> {
        // pc: crates/ui-egui/src/dialogs/layer_style.rs:90-216 `spec` (keys and labels).
        let label = match key {
            "blend" => &self.blend_mode,
            "color" => &self.color,
            "opacity" => &self.opacity,
            "fillOpacity" => &self.fill_opacity,
            "angle" => &self.angle,
            "useGlobalLight" => &self.use_global_light,
            "distance" => &self.distance,
            "spread" => &self.spread,
            "choke" => &self.choke,
            "size" => &self.size,
            "contour" => &self.contour,
            "noise" => &self.noise,
            "knocksOut" => &self.knocks_out,
            "technique" => &self.technique,
            "source" => &self.source,
            "range" | "contourRange" => &self.range,
            "position" => &self.position,
            "overprint" => &self.overprint,
            "from" => &self.from,
            "to" => &self.to,
            "reverse" => &self.reverse,
            "style" => &self.style,
            "scale" => &self.scale,
            "method" => &self.method,
            "pattern" => &self.pattern,
            "link" => &self.link,
            "depth" => &self.depth,
            "direction" => &self.direction,
            "soften" => &self.soften,
            "altitude" => &self.altitude,
            "glossContour" => &self.gloss_contour,
            "highlightBlend" => &self.highlight_mode,
            "highlightColor" => &self.highlight_color,
            "highlightOpacity" => &self.highlight_opacity,
            "shadowBlend" => &self.shadow_mode,
            "shadowColor" => &self.shadow_color,
            "shadowOpacity" => &self.shadow_opacity,
            "invert" => &self.invert,
            _ => return None,
        };
        Some(label)
    }

    /// The label of a built-in choice value (`"chiselHard"` → "Chisel Hard"; Gradient Overlay's
    /// `"angle"` style → "Angle"); `None` for another value.
    pub fn choice(&self, value: &str) -> Option<&str> {
        // pc: crates/ui-egui/src/dialogs/layer_style.rs:91-97 (choice tables), :1084-1094 (methods).
        let label = match value {
            "softer" => &self.softer,
            "precise" => &self.precise,
            "edge" => &self.edge,
            "center" => &self.center,
            "outside" => &self.outside,
            "inside" => &self.inside,
            "linear" => &self.linear,
            "radial" => &self.radial,
            "angle" => &self.angle_style,
            "reflected" => &self.reflected,
            "diamond" => &self.diamond,
            "inner" => &self.inner_bevel,
            "outer" => &self.outer_bevel,
            "emboss" => &self.emboss,
            "pillow" => &self.pillow_emboss,
            "smooth" => &self.smooth,
            "chiselHard" => &self.chisel_hard,
            "chiselSoft" => &self.chisel_soft,
            "up" => &self.up,
            "down" => &self.down,
            "perceptual" => &self.perceptual,
            "classic" => &self.classic,
            "custom" => &self.custom,
            _ => return None,
        };
        Some(label)
    }
}

craft_ui::words! {
    /// The accessible names of a [`BlendIfBar`](super::BlendIfBar)'s handles, English by default.
    // Every field is one self-evident word; its English text is the default.
    #[allow(missing_docs)]
    #[non_exhaustive]
    pub struct BlendIfBarLabels {
        black_low: "blendIf.blackLow" = "Black low";
        black_high: "blendIf.blackHigh" = "Black high";
        white_low: "blendIf.whiteLow" = "White low";
        white_high: "blendIf.whiteHigh" = "White high";
        /// A handle's name: `{handle}` is one of the four above, `{layer}` the bar's name ("This
        /// Layer").
        handle: "blendIf.handle" = "{handle}, {layer}";
        /// A split side's levels in the readout: `{low}` and `{high}`.
        split_levels: "blendIf.splitLevels" = "{low} / {high}";
        /// The readout over the bar: `{black}` is the black handles' side, `{white}` the white
        /// handles'.
        readout: "blendIf.readout" = "{black}    {white}";
    }
}

craft_ui::words! {
    /// The words of a [`ContourField`](super::ContourField), its preset picker and the Contour
    /// Editor, English by default.
    // Every field is one self-evident word; its English text is the default.
    #[allow(missing_docs)]
    #[non_exhaustive]
    pub struct ContourFieldLabels {
        /// The thumbnail's accessible name and tooltip: it opens the editor.
        editor: "contourField.editor" = "Contour Editor";
        /// The picker arrow's accessible name.
        presets: "contourField.presets" = "Contour presets";
        anti_aliased: "contourField.antiAliased" = "Anti-aliased";
        load_contours: "contourField.loadContours" = "Load Contours…";
        replace_contours: "contourField.replaceContours" = "Replace Contours…";
        save_contours: "contourField.saveContours" = "Save Contours…";
        delete_contour: "contourField.deleteContour" = "Delete Contour";
        preset: "contourField.preset" = "Preset";
        input: "contourField.input" = "Input:";
        output: "contourField.output" = "Output:";
        corner: "contourField.corner" = "Corner";
        new: "contourField.new" = "New…";
        load: "contourField.load" = "Load…";
        save: "contourField.save" = "Save…";
        name: "contourField.name" = "Name";
        /// The name New… suggests for the edited curve.
        suggested_name: "contourField.suggestedName" = "Contour";
        /// The built-in presets' names (Photoshop's), and the name of an edited preset.
        preset_linear: "contourField.presetLinear" = "Linear";
        preset_cone: "contourField.presetCone" = "Cone";
        preset_cone_inverted: "contourField.presetConeInverted" = "Cone - Inverted";
        preset_cove_deep: "contourField.presetCoveDeep" = "Cove - Deep";
        preset_cove_shallow: "contourField.presetCoveShallow" = "Cove - Shallow";
        preset_gaussian: "contourField.presetGaussian" = "Gaussian";
        preset_half_round: "contourField.presetHalfRound" = "Half Round";
        preset_ring: "contourField.presetRing" = "Ring";
        preset_ring_double: "contourField.presetRingDouble" = "Ring - Double";
        preset_rolling_slope: "contourField.presetRollingSlope" = "Rolling Slope - Descending";
        preset_rounded_steps: "contourField.presetRoundedSteps" = "Rounded Steps";
        preset_sawtooth: "contourField.presetSawtooth" = "Sawtooth 1";
        preset_custom: "contourField.presetCustom" = "Custom";
        ok: "" = "OK";
        cancel: "" = "Cancel";
    }
}
