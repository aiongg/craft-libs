//! [`LayerStyleDialog`]: the Layer Style dialog's body (task AD-8).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land (task AD-8)")]

use craft_theme::Srgba;
use egui::{AsIdSalt, IdSalt, Ui};

use craft_ui::controls::BlendChoice;
use craft_ui::dialog::ColorRequest;
use craft_ui::edit::EditResponse;
use craft_ui::localized::LocaleNumbers;

use super::contour::ContourAction;
use super::labels::LayerStyleLabels;
use super::page::PatternChoice;
use super::styles::StylePresets;
use crate::layer_style::{Contour, EffectId, EffectKind, EffectSpec, LayerStyleDraft, StyleParams};

/// What [`LayerStyleDialog::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the edit and answers the requests"]
pub struct LayerStyleOutput {
    /// The draft after this pass's change: a slider drag is a gesture (`Begin`, `Update`…,
    /// `Commit`, Esc `Cancel`); a check, select, list action, page change or preset is a `Set`.
    /// The host keeps the value and previews it while its `preview` is on; everything stays
    /// inside the dialog's outer gesture, and OK makes the one "Layer Style" history step.
    pub edit: EditResponse<LayerStyleDraft>,
    /// A colour swatch was clicked: the host opens its colour picker and answers with
    /// [`LayerStyleDialog::set_color`].
    pub pick_color: Option<ColorRequest>,
    /// A style preset was picked on the Styles page (its key): the host looks it up and calls
    /// [`LayerStyleDraft::apply_preset`].
    pub apply_style: Option<String>,
    /// New Style… was pressed: the host asks for a name and saves the enabled instances and the
    /// blend mode and opacities as a preset.
    pub new_style: bool,
    /// Make Default on an effect page: the kind and the page's shown values, to keep as the
    /// user's default for new instances of the kind.
    pub make_default: Option<(EffectKind, StyleParams)>,
    /// Reset to Default on an effect page: the host writes the user's default (or the spec's
    /// defaults before the first Make Default) into the instance's parameters.
    pub reset_to_default: Option<EffectId>,
    /// A request for the host's contour library from a contour field.
    pub contour_action: Option<ContourAction>,
}

/// The Layer Style dialog's body: the effect list on the left, the selected page on the right
/// (`layer_style.page_width`, a vertical divider between), as Photoshop lays it out. The host owns
/// the frame (`Dialog` with OK, Cancel and Alt+Cancel Reset), the draft, the engine and the
/// preview.
///
/// - The list: Styles, Blending Options, the effects (one Tab stop, ↑ and ↓, Space toggles, Enter
///   shows the page; absent effects dimmed, `+` and `−` for kinds that allow several).
/// - The page title is the effect's name (`type.title`) with the Preview check box at its end
///   (it flips [`LayerStyleDraft::preview`]; the Styles page has none).
/// - The Styles page ([`StylePresets`]), the Blending Options page, or an effect page drawn from
///   the kind's [`EffectSpec`].
/// - Use Global Light: on an effect that uses it, the Angle edits the shared light angle of
///   every such effect ([`LayerStyleDraft::set_param`]).
/// - Tab order: the list, then the page's controls; in the dialog Tab visits fields only (the
///   dialog rule). Esc is the frame's (Cancel).
///
/// ```no_run
/// use craft_imaging::layer_style::LayerStyleDraft;
/// use craft_imaging::ui::layer_style::LayerStyleDialog;
/// # fn demo(ui: &mut egui::Ui, draft: &mut LayerStyleDraft, channels: &[String], numbers: &craft_ui::localized::LocaleNumbers) {
/// let out = LayerStyleDialog::new("layer-style", draft, channels, numbers).show(ui);
/// if let Some(value) = out.edit.event.and_then(|e| e.value().cloned()) {
///     *draft = value; // and re-render the preview when `draft.render_hash()` changed
/// }
/// # }
/// ```
#[must_use = "show the dialog body with `show`"]
pub struct LayerStyleDialog<'a> {
    id_salt: IdSalt,
    draft: &'a LayerStyleDraft,
    channel_names: &'a [String],
    numbers: &'a LocaleNumbers,
    specs: Option<&'a [EffectSpec]>,
    blend_modes: &'a [BlendChoice],
    patterns: &'a [PatternChoice],
    user_contours: &'a [Contour],
    style_presets: Option<StylePresets<'a>>,
    labels: Option<LayerStyleLabels>,
    enabled: bool,
}

impl<'a> LayerStyleDialog<'a> {
    /// The body over `draft`; `channel_names` are the document's colour channels ("R", "G", "B";
    /// "C", "M", "Y", "K"; "L", "a", "b"; "Gray") for the Channels checks and Blend If.
    pub fn new(id_salt: impl AsIdSalt, draft: &'a LayerStyleDraft, channel_names: &'a [String], numbers: &'a LocaleNumbers) -> Self {
        LayerStyleDialog {
            id_salt: IdSalt::new(id_salt),
            draft,
            channel_names,
            numbers,
            specs: None,
            blend_modes: &[],
            patterns: &[],
            user_contours: &[],
            style_presets: None,
            labels: None,
            enabled: true,
        }
    }

    /// The effect tables in list order; Photoshop's ten ([`EffectSpec::photoshop`] for each of
    /// [`EffectKind::ALL`]) by default. A kind without a spec is not listed.
    pub fn specs(mut self, specs: &'a [EffectSpec]) -> Self {
        self.specs = Some(specs);
        self
    }

    /// The blend modes the Blend Mode selects offer (the host's list, with its separators).
    pub fn blend_modes(mut self, modes: &'a [BlendChoice]) -> Self {
        self.blend_modes = modes;
        self
    }

    /// The patterns Pattern Overlay offers.
    pub fn patterns(mut self, patterns: &'a [PatternChoice]) -> Self {
        self.patterns = patterns;
        self
    }

    /// The host's contour library, after the built-in presets in every contour picker.
    pub fn user_contours(mut self, contours: &'a [Contour]) -> Self {
        self.user_contours = contours;
        self
    }

    /// The Styles page's presets; without them the page shows only New Style….
    pub fn style_presets(mut self, presets: StylePresets<'a>) -> Self {
        self.style_presets = Some(presets);
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: LayerStyleLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the list and the page and reports this pass's change. List actions, field edits and
    /// page changes are applied to a copy of the draft with the [`LayerStyleDraft`] rules and
    /// reported as that copy.
    pub fn show(self, ui: &mut Ui) -> LayerStyleOutput {
        // pc: crates/ui-egui/src/dialogs/layer_style.rs:831-1185 `body` (list, divider at 330, title
        // with Preview, Styles page, Blending Options page, field pages, Make Default and Reset to
        // Default, the contour editor).
        todo!()
    }

    /// Answers a [`ColorRequest`] the dialog made: sets the colour parameter its key names
    /// (`"fx2/color"`; the instance id, a slash, the parameter key) to `color`. Returns false,
    /// changing nothing, for a key that names no instance of `draft`.
    pub fn set_color(draft: &mut LayerStyleDraft, key: &str, color: Srgba) -> bool {
        // pc: crates/ui-egui/src/dialogs/layer_style/color_picker.rs:128-136 `confirm` (the answer
        // written through `set_param` into the effect and field the request named).
        todo!()
    }
}
