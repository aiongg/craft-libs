//! [`AdjustLabels`]: one word table for every adjustment editor (lead call L4), so the editors
//! share Preserve Luminosity, Reds and Hue (task AD-1 owns the file; every editor reads it).
//!
//! One word per English text: the Photo Filter presets Red, Yellow and the rest, Posterize's
//! Levels and Lab's Lightness reuse the channel and panel words. US spelling; Photoshop's words and capitalisation. Templates take `{key}` (a key name from the
//! platform's shortcut words, ⌘ or Ctrl, Shift).

craft_ui::words! {
    /// The words of the adjustment editors, in the installed language unless set.
    #[non_exhaustive]
    pub struct AdjustLabels {
        // Channels and the Channel select (Levels, Curves).
        channel: "adjust.channel" = "Channel";
        rgb: "adjust.rgb" = "RGB";
        cmyk: "adjust.cmyk" = "CMYK";
        red: "adjust.red" = "Red";
        green: "adjust.green" = "Green";
        blue: "adjust.blue" = "Blue";
        cyan: "adjust.cyan" = "Cyan";
        magenta: "adjust.magenta" = "Magenta";
        yellow: "adjust.yellow" = "Yellow";
        black: "adjust.black" = "Black";
        lab_a: "adjust.labA" = "a";
        lab_b: "adjust.labB" = "b";
        gray: "adjust.gray" = "Gray";
        /// The Levels panel's Auto button.
        auto: "adjust.auto" = "Auto";
        /// The Levels panel's group name (AccessKit) and Posterize's slider.
        levels: "adjust.levels" = "Levels";
        /// The Curves panel's group name (AccessKit).
        curves: "adjust.curves" = "Curves";

        // Point pickers (Levels, Curves).
        eyedroppers: "adjust.eyedroppers" = "Eyedroppers";
        set_black_point: "adjust.setBlackPoint" = "Set Black Point";
        set_gray_point: "adjust.setGrayPoint" = "Set Neutral Gray Point";
        set_white_point: "adjust.setWhitePoint" = "Set White Point";
        unavailable_mode: "adjust.unavailableMode" = "Unavailable in this color mode";

        // Curves.
        edit_points: "adjust.editPoints" = "Edit points to modify the curve";
        draw_curve: "adjust.drawCurve" = "Draw to modify the curve";
        smooth: "adjust.smooth" = "Smooth";
        smooth_tip: "adjust.smoothTip" = "Smooth the curve values";
        points_hint: "adjust.pointsHint" = "Click to add a point · drag off or {key}-click or Delete to remove";
        pencil_hint: "adjust.pencilHint" = "Drag to draw the curve · {key}-click draws a straight line";

        // Slider tables.
        brightness: "adjust.brightness" = "Brightness";
        contrast: "adjust.contrast" = "Contrast";
        use_legacy: "adjust.useLegacy" = "Use Legacy";
        exposure: "adjust.exposure" = "Exposure";
        offset: "adjust.offset" = "Offset";
        gamma_correction: "adjust.gammaCorrection" = "Gamma Correction";
        vibrance: "adjust.vibrance" = "Vibrance";
        saturation: "adjust.saturation" = "Saturation";
        threshold_level: "adjust.thresholdLevel" = "Threshold Level";
        histogram: "adjust.histogram" = "Histogram";

        // Color Balance.
        tone: "adjust.tone" = "Tone";
        shadows: "adjust.shadows" = "Shadows";
        midtones: "adjust.midtones" = "Midtones";
        highlights: "adjust.highlights" = "Highlights";
        preserve_luminosity: "adjust.preserveLuminosity" = "Preserve Luminosity";

        // Colour ranges (Hue/Saturation, Black & White, Selective Color).
        master: "adjust.master" = "Master";
        range: "adjust.range" = "Range";
        reds: "adjust.reds" = "Reds";
        yellows: "adjust.yellows" = "Yellows";
        greens: "adjust.greens" = "Greens";
        cyans: "adjust.cyans" = "Cyans";
        blues: "adjust.blues" = "Blues";
        magentas: "adjust.magentas" = "Magentas";
        whites: "adjust.whites" = "Whites";
        neutrals: "adjust.neutrals" = "Neutrals";
        blacks: "adjust.blacks" = "Blacks";

        // Hue/Saturation.
        hue: "adjust.hue" = "Hue";
        lightness: "adjust.lightness" = "Lightness";
        colorize: "adjust.colorize" = "Colorize";
        spectrum_before: "adjust.spectrumBefore" = "Hue spectrum, before";
        spectrum_after: "adjust.spectrumAfter" = "Hue spectrum, after";
        range_start: "adjust.rangeStart" = "Range start";
        full_start: "adjust.fullStart" = "Fully affected start";
        full_end: "adjust.fullEnd" = "Fully affected end";
        range_end: "adjust.rangeEnd" = "Range end";

        // Black & White.
        tint: "adjust.tint" = "Tint";
        tint_color: "adjust.tintColor" = "Tint Color";
        default_button: "adjust.default" = "Default";

        // Photo Filter.
        filter: "adjust.filter" = "Filter";
        color: "adjust.color" = "Color";
        filter_color: "adjust.filterColor" = "Filter Color";
        density: "adjust.density" = "Density";
        custom: "adjust.custom" = "Custom";
        warming_85: "adjust.warming85" = "Warming Filter (85)";
        warming_lba: "adjust.warmingLba" = "Warming Filter (LBA)";
        warming_81: "adjust.warming81" = "Warming Filter (81)";
        cooling_80: "adjust.cooling80" = "Cooling Filter (80)";
        cooling_lbb: "adjust.coolingLbb" = "Cooling Filter (LBB)";
        cooling_82: "adjust.cooling82" = "Cooling Filter (82)";
        orange: "adjust.orange" = "Orange";
        violet: "adjust.violet" = "Violet";
        sepia: "adjust.sepia" = "Sepia";
        deep_red: "adjust.deepRed" = "Deep Red";
        deep_blue: "adjust.deepBlue" = "Deep Blue";
        deep_emerald: "adjust.deepEmerald" = "Deep Emerald";
        deep_yellow: "adjust.deepYellow" = "Deep Yellow";
        underwater: "adjust.underwater" = "Underwater";

        // Channel Mixer.
        output_channel: "adjust.outputChannel" = "Output Channel";
        total: "adjust.total" = "Total";
        /// The Total readout's accessible value above 100 %: "{value}, over 100 percent".
        total_over: "adjust.totalOver" = "{value}, over 100 percent";
        constant: "adjust.constant" = "Constant";
        monochrome: "adjust.monochrome" = "Monochrome";

        // Selective Color.
        colors: "adjust.colors" = "Colors";
        relative: "adjust.relative" = "Relative";
        absolute: "adjust.absolute" = "Absolute";
    }
}
