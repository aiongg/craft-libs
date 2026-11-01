//! [`GradientMapEditor`]: Photoshop's Gradient Map adjustment: the preset picker, the
//! [`GradientEditor`](super::GradientEditor) stop bar with colour stops only, Reverse and Dither (task AD-6).
//!
//! It lives here, beside the gradient editor it builds on, rather than with the other adjustment
//! editors in craft-imaging: a family crate depends on no other family crate but craft-color
//! (lead call L5).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Id, IdSalt, Ui};

use craft_ui::edit::{EditPolicy, EditResponse};
use craft_ui::localized::LocaleNumbers;

use super::{GradientGroup, GradientLabels, GradientStops, Sampler};

/// A Gradient Map value: the map from tone (0 black … 1 white) to colour.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientMap {
    /// Colour stops only ([`StopRows::ColorOnly`](super::StopRows::ColorOnly)): every alpha 1,
    /// no opacity stops; 2..=64 stops, locations rounded to 0.001.
    pub stops: GradientStops,
    /// Reverse: the map runs white to black.
    pub reverse: bool,
    /// Dither: the host adds noise to hide banding.
    pub dither: bool,
}

craft_ui::words! {
    /// The Gradient Map editor's words, in the installed language unless set.
    #[non_exhaustive]
    pub struct GradientMapLabels {
        /// The editor's group name (AccessKit) and the stop bar's label.
        gradient_map: "gradientMap.title" = "Gradient Map";
        preset: "gradientMap.preset" = "Preset";
        /// The preset button's text while the stops match no preset (always: the editor keeps no
        /// preset identity).
        custom: "gradientMap.custom" = "Custom";
        preset_options: "gradientMap.presetOptions" = "Gradient Preset Options";
        import_presets: "gradientMap.importPresets" = "Import .GRD";
        reverse: "gradientMap.reverse" = "Reverse";
        dither: "gradientMap.dither" = "Dither";
    }
}

/// What [`GradientMapEditor::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct GradientMapOutput {
    /// The map's edit: the stop bar's gestures and sets with the stops replaced, a preset pick,
    /// Reverse and Dither as `Set`s. The response is the editor's group.
    pub edit: EditResponse<GradientMap>,
    /// Without [`color_popover`](GradientMapEditor::color_popover): the colour stop whose colour
    /// the host is asked to edit ([`GradientEditorResponse::color_requested`](super::GradientEditorResponse::color_requested)).
    pub color_requested: Option<usize>,
    /// With the popover: its eyedropper was pressed, and the popover's picker id while shown
    /// (as [`GradientEditorResponse`](super::GradientEditorResponse) reports them).
    pub eyedropper_requested: bool,
    pub picker_id: Option<Id>,
    /// The user chose Import .GRD in the preset menu: the host opens its gradient-file dialog
    /// and adds the file's presets to its library; the value is unchanged.
    pub import_presets: bool,
}

/// Gradient Map: Image › Adjustments › Gradient Map and a Gradient Map layer's Properties.
///
/// Top to bottom (PhotoCraft's order):
/// 1. **Preset**: a button reading "Custom" opens a popover with the host's
///    [`GradientGroup`]s in a [`GradientPalette`](super::GradientPalette) (List layout), a gear
///    menu with Import .GRD above it. A pick replaces the stops with the preset's colour stops
///    (alpha forced to 1, opacity stops dropped) as one `Set`, closes the popover and selects the
///    first stop. The host resolves Foreground and Background stops to its current colours
///    before passing the groups.
/// 2. **The stop bar**: [`GradientEditor`](super::GradientEditor) with [`StopRows::ColorOnly`](super::StopRows::ColorOnly),
///    2 to 64 stops, no midpoints, location fields in percent with PhotoCraft's 0.001 rounding of
///    locations. Its edits come out as edits of the map.
/// 3. **Reverse** and **Dither** check boxes, each one `Set`.
///
/// Keyboard and AccessKit: the preset button, the stop bar (as `GradientEditor`), then the two
/// check boxes are Tab stops; the editor is a `Group` named "Gradient Map".
#[must_use = "show the editor with `show`"]
pub struct GradientMapEditor<'a> {
    id_salt: IdSalt,
    value: &'a GradientMap,
    numbers: &'a LocaleNumbers,
    presets: &'a [GradientGroup],
    sampler: Option<&'a Sampler>,
    color_popover: bool,
    policy: EditPolicy,
    labels: Option<GradientMapLabels>,
    stop_labels: Option<GradientLabels>,
    enabled: bool,
}

impl<'a> GradientMapEditor<'a> {
    /// The editor over `value`.
    pub fn new(id_salt: impl AsIdSalt, value: &'a GradientMap, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            numbers,
            presets: &[],
            sampler: None,
            color_popover: false,
            policy: EditPolicy::default(),
            labels: None,
            stop_labels: None,
            enabled: true,
        }
    }

    /// The host's gradient presets (the same library as the Gradient tool and the Gradients
    /// panel); the preset button is disabled without any.
    pub fn presets(mut self, groups: &'a [GradientGroup]) -> Self {
        self.presets = groups;
        self
    }

    /// The host's colour mixing for the bar and the presets ([`GradientEditor::sampler`](super::GradientEditor::sampler)).
    pub fn sampler(mut self, sampler: &'a Sampler) -> Self {
        self.sampler = Some(sampler);
        self
    }

    /// Edits a stop's colour in the bar's own colour popover ([`GradientEditor::color_popover`](super::GradientEditor::color_popover))
    /// instead of asking the host; off by default (PhotoCraft opens its Color Picker).
    pub fn color_popover(mut self, popover: bool) -> Self {
        self.color_popover = popover;
        self
    }

    /// The stop bar's edit policy.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The editor's words; the installed ones by default.
    pub fn labels(mut self, labels: GradientMapLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The stop bar's own words ([`GradientEditor::labels`](super::GradientEditor::labels)).
    pub fn stop_labels(mut self, labels: GradientLabels) -> Self {
        self.stop_labels = Some(labels);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the editor and reports the map's edit.
    pub fn show(self, ui: &mut Ui) -> GradientMapOutput {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/gradient_map.rs:151-269 `gradient_map` (preset
        // combo with the gear and Import .GRD :158-205, the strip :206, 64 stops at most :120 and
        // :243, two at least :212, Reverse and Dither :259-266); locations rounded to 0.001 (:65).
        // lib: `GradientEditor::new(..).rows(StopRows::ColorOnly).stops(2, 64).midpoints(false)`;
        // the preset list is `GradientPalette` with `PaletteLayout::List` in a `Popover`.
        todo!()
    }
}
