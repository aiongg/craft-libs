//! [`CropBar`]: the Crop tool's options in the body and trailing slots of
//! [`OptionsBar`](craft_ui::patterns::OptionsBar).

use std::cell::RefCell;
use std::rc::Rc;

use egui::{AsIdSalt, IdSalt, Ui, UiBuilder};
use kurbo::Size;

use super::menus::{CropOptionsMenu, CropOverlayMenu};
use super::options::{
    CROP_RATIOS, CROP_SIZES, CropFill, CropPresetChoice, CropRatio, CropSettings, CropSizeFields, CropUserPreset, FIELD_MAX, RATIO_DECIMALS,
    RATIO_MAX, ResolutionUnit,
};
use super::session::{CropDocKey, CropDocument};
use craft_ui::controls::{Button, Checkbox, IconButton, NumberField, Select, TextField};
use craft_ui::dialog::ColorRequest;
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::patterns::{CellWidth, OptionsBar, OptionsBarSlots};
use craft_ui::theme::{Tokens, TokensOf as _};

craft_ui::words! {
    /// The words of the Crop tool's options bar and its overlay and gear menus, English by
    /// default (Photoshop's names and tooltips).
    #[non_exhaustive]
    pub struct CropBarLabels {
        /// The preset list's accessible name.
        presets: "cropBar.presets" = "Crop ratio";
        #[allow(missing_docs)]
        ratio: "cropBar.ratio" = "Ratio";
        #[allow(missing_docs)]
        size_mode: "cropBar.sizeMode" = "W x H x Resolution";
        /// The names of [`CROP_RATIOS`](super::CROP_RATIOS), in its order.
        ratio_original: "cropBar.ratioOriginal" = "Original Ratio";
        #[allow(missing_docs)]
        ratio_square: "cropBar.ratioSquare" = "1 : 1 (Square)";
        #[allow(missing_docs)]
        ratio_4_5: "cropBar.ratio4x5" = "4 : 5 (8 : 10)";
        #[allow(missing_docs)]
        ratio_5_7: "cropBar.ratio5x7" = "5 : 7";
        #[allow(missing_docs)]
        ratio_2_3: "cropBar.ratio2x3" = "2 : 3 (4 : 6)";
        #[allow(missing_docs)]
        ratio_16_9: "cropBar.ratio16x9" = "16 : 9";
        #[allow(missing_docs)]
        front_image: "cropBar.frontImage" = "Front Image";
        #[allow(missing_docs)]
        new_preset: "cropBar.newPreset" = "New Crop Preset…";
        #[allow(missing_docs)]
        delete_preset: "cropBar.deletePreset" = "Delete Crop Preset…";
        /// The unit word of a px/in resolution in a preset name ("4 x 5 in 300 ppi").
        ppi: "cropBar.ppi" = "ppi";
        #[allow(missing_docs)]
        pixels_per_inch: "cropBar.pixelsPerInch" = "px/in";
        #[allow(missing_docs)]
        pixels_per_centimetre: "cropBar.pixelsPerCentimeter" = "px/cm";
        #[allow(missing_docs)]
        width: "cropBar.width" = "Width";
        #[allow(missing_docs)]
        height: "cropBar.height" = "Height";
        #[allow(missing_docs)]
        resolution: "cropBar.resolution" = "Resolution";
        #[allow(missing_docs)]
        resolution_unit: "cropBar.resolutionUnit" = "Resolution unit";
        #[allow(missing_docs)]
        swap: "cropBar.swap" = "Swaps height and width";
        #[allow(missing_docs)]
        clear: "cropBar.clear" = "Clear";
        #[allow(missing_docs)]
        straighten: "cropBar.straighten" = "Straighten the image by drawing a line on it";
        #[allow(missing_docs)]
        overlay: "cropBar.overlay" = "Set the overlay options for the Crop tool";
        /// The overlays in [`CropGuide::ALL`](super::CropGuide::ALL) order.
        thirds: "cropBar.thirds" = "Rule of Thirds";
        #[allow(missing_docs)]
        grid: "cropBar.grid" = "Grid";
        #[allow(missing_docs)]
        diagonal: "cropBar.diagonal" = "Diagonal";
        #[allow(missing_docs)]
        triangle: "cropBar.triangle" = "Triangle";
        #[allow(missing_docs)]
        golden_ratio: "cropBar.goldenRatio" = "Golden Ratio";
        #[allow(missing_docs)]
        golden_spiral: "cropBar.goldenSpiral" = "Golden Spiral";
        /// The show rules in [`GuideShow::ALL`](super::GuideShow::ALL) order.
        show_auto: "cropBar.showAuto" = "Auto Show Overlay";
        #[allow(missing_docs)]
        show_always: "cropBar.showAlways" = "Always Show Overlay";
        #[allow(missing_docs)]
        show_never: "cropBar.showNever" = "Never Show Overlay";
        #[allow(missing_docs)]
        cycle_overlay: "cropBar.cycleOverlay" = "Cycle Overlay";
        #[allow(missing_docs)]
        cycle_orientation: "cropBar.cycleOrientation" = "Cycle Orientation";
        #[allow(missing_docs)]
        options: "cropBar.options" = "Set additional Crop options";
        #[allow(missing_docs)]
        classic_mode: "cropBar.classicMode" = "Use Classic Mode";
        #[allow(missing_docs)]
        show_cropped_area: "cropBar.showCroppedArea" = "Show Cropped Area";
        #[allow(missing_docs)]
        auto_center: "cropBar.autoCenter" = "Auto Center Preview";
        #[allow(missing_docs)]
        shield: "cropBar.shield" = "Enable Crop Shield";
        #[allow(missing_docs)]
        color: "cropBar.color" = "Color:";
        /// The shield colour radio group's accessible name.
        color_name: "cropBar.colorName" = "Color";
        #[allow(missing_docs)]
        match_canvas: "cropBar.matchCanvas" = "Match Canvas";
        #[allow(missing_docs)]
        custom: "cropBar.custom" = "Custom…";
        #[allow(missing_docs)]
        shield_color: "cropBar.shieldColor" = "Crop Shield color";
        #[allow(missing_docs)]
        opacity: "cropBar.opacity" = "Opacity:";
        /// The shield opacity row's accessible name.
        opacity_name: "cropBar.opacityName" = "Opacity";
        #[allow(missing_docs)]
        auto_adjust: "cropBar.autoAdjust" = "Auto Adjust Opacity";
        #[allow(missing_docs)]
        delete_pixels: "cropBar.deletePixels" = "Delete Cropped Pixels";
        #[allow(missing_docs)]
        fill: "cropBar.fill" = "Fill:";
        /// The Fill select's accessible name.
        fill_name: "cropBar.fillName" = "Fill";
        #[allow(missing_docs)]
        fill_background: "cropBar.fillBackground" = "Background (Default)";
        #[allow(missing_docs)]
        fill_content_aware: "cropBar.fillContentAware" = "Content-Aware Fill";
        #[allow(missing_docs)]
        commit: "cropBar.commit" = "Commit current crop operation";
        #[allow(missing_docs)]
        cancel: "cropBar.cancel" = "Cancel current crop operation";
    }
}

/// The ratio, a ratio field, a preset or W × H × Resolution changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CropRatioChange {
    /// True while a ratio field still has the focus or is scrubbed, so the frame refits from the
    /// frame the edit began with (#7752). A typed edit reports once, when the typing run ends
    /// (every `NumberField` reports typed text so), so typing never refits live.
    pub editing: bool,
}

/// What the bar reported this pass. Edits of the options go straight into the settings the bar was
/// given ([`settings_changed`](Self::settings_changed) says they changed); these are the rest.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app refits the frame, ends the session or opens a dialog for each field that is set"]
pub struct CropBarOutput {
    /// The ratio changed: the app calls [`CropTool::refit`](super::CropTool::refit) with
    /// [`CropRatioChange::editing`].
    pub ratio: Option<CropRatioChange>,
    /// The Straighten button: [`CropTool::toggle_straighten`](super::CropTool::toggle_straighten).
    pub straighten: bool,
    /// ✓: [`CropTool::apply`](super::CropTool::apply).
    pub apply: bool,
    /// ⊘: [`CropTool::discard`](super::CropTool::discard).
    pub discard: bool,
    /// New Crop Preset…: the app opens its dialog with [`CropSettings::preset_name`].
    pub new_preset: bool,
    /// Delete Crop Preset…: the app opens its dialog.
    pub delete_preset: bool,
    /// The gear menu's Custom… or the shield swatch: the app opens its colour picker with this
    /// request and sets [`CropShield::custom_color`](super::CropShield::custom_color) from the
    /// answer.
    pub pick_color: Option<ColorRequest>,
    /// Any option of the settings changed this pass: the app persists them.
    pub settings_changed: bool,
}

/// The Crop tool's options bar body and trailing slot.
///
/// ```no_run
/// use craft_draw::ui::crop::{CropBar, CropBarOutput, CropSettings};
/// use craft_ui::patterns::OptionsBar;
/// # fn demo(ui: &mut egui::Ui, settings: &mut CropSettings, armed: bool) {
/// let mut out = CropBarOutput::default();
/// OptionsBar::new("options").show(ui, |bar| {
///     CropBar::new("crop", settings).straighten(armed).output(&mut out).add_to(bar);
/// });
/// if out.apply {
///     // CropTool::apply
/// }
/// # }
/// ```
///
/// - Order (PhotoCraft's): the preset list (Ratio, W x H x Resolution, the ratio presets, the size
///   presets, the user's presets, Front Image, a separator, New Crop Preset…, Delete Crop
///   Preset…; `crop.preset_width` wide, 140); the W and H
///   ratio fields (`CellWidth::Sm`, 0 to 99,999 with 3 decimals) with ⇄ between them, or in
///   W × H × Resolution mode the W, H and Resolution text fields with the px/in or px/cm select;
///   a divider; Clear; the Straighten toggle (`Icon::Ruler`, pressed while armed); the overlay menu
///   ([`CropOverlayMenu`](super::CropOverlayMenu)); the gear menu
///   ([`CropOptionsMenu`](super::CropOptionsMenu), open flag through
///   [`options_open`](Self::options_open)); the Delete Cropped Pixels check box; Fill and
///   its select (`CellWidth::Content`). The trailing slot holds ⊘ and ✓
///   ([`OptionsBar::commit_cancel`](craft_ui::patterns::OptionsBar::commit_cancel)).
/// - Every control is at [`OptionsBar::CONTROL_SIZE`].
/// - Length fields keep what is typed while they have the focus and normalise it when the typing
///   run ends ([`CropSizeFields::normalize_length`](super::CropSizeFields::normalize_length)).
/// - Typed ratio edits refit when the typing run ends; steps and scrubs refit live
///   ([`CropRatioChange::editing`]).
/// - Delete Crop Preset… is disabled when no user preset is given; New Crop Preset… when
///   [`CropSettings::preset_name`] is `None`.
/// - **Keyboard and AccessKit:** cells in reading order; the list is a `Select` named "Crop ratio"
///   with the chosen entry as its value; the fields are named Width, Height and Resolution; the
///   Straighten toggle has its pressed state; the menus open with Enter or Space.
pub struct CropBar<'a> {
    id_salt: IdSalt,
    settings: &'a mut CropSettings,
    labels: Option<CropBarLabels>,
    document: Option<&'a CropDocument>,
    user_presets: &'a [CropUserPreset],
    straighten: bool,
    options_open: Option<&'a mut bool>,
    output: Option<&'a mut CropBarOutput>,
}

impl<'a> CropBar<'a> {
    /// The bar editing `settings`.
    pub fn new(id_salt: impl AsIdSalt, settings: &'a mut CropSettings) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            settings,
            labels: None,
            document: None,
            user_presets: &[],
            straighten: false,
            options_open: None,
            output: None,
        }
    }

    /// The words; default: [`CropBarLabels`] of the installed words.
    pub fn labels(mut self, labels: CropBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The active document: Original Ratio's numbers and Front Image. Without one both are
    /// disabled.
    pub fn document(mut self, document: &'a CropDocument) -> Self {
        self.document = Some(document);
        self
    }

    /// The user's saved crop presets, oldest first.
    pub fn user_presets(mut self, presets: &'a [CropUserPreset]) -> Self {
        self.user_presets = presets;
        self
    }

    /// The Straighten toggle's pressed state ([`CropTool::straighten_armed`](super::CropTool::straighten_armed)).
    pub fn straighten(mut self, armed: bool) -> Self {
        self.straighten = armed;
        self
    }

    /// The gear popover's open flag. Without it the bar keeps the flag itself, per viewer.
    pub fn options_open(mut self, open: &'a mut bool) -> Self {
        self.options_open = Some(open);
        self
    }

    /// Where the bar reports what happened.
    pub fn output(mut self, output: &'a mut CropBarOutput) -> Self {
        self.output = Some(output);
        self
    }

    /// Fills the bar's body and trailing slots.
    pub fn add_to(self, slots: &mut OptionsBarSlots<'a>) {
        // pc: crates/ui-egui/src/panels/options_bar.rs:465-555 (the Crop branch); crop_presets.rs:137
        // `dropdown`; crop_size.rs:316 `fields`, :287 `text_field`; crop_straighten.rs:166
        // `options_button`. Both slots report into the one output.
        let Self { id_salt, settings, labels, document, user_presets, straighten, options_open, output } = self;
        let output = Rc::new(RefCell::new(output));
        let trailing_output = Rc::clone(&output);
        let trailing_labels = labels.clone();
        slots.body(move |ui| {
            let labels = labels.unwrap_or_else(|| CropBarLabels::of(ui));
            let bar = Body { settings, labels: &labels, document, user_presets, straighten, options_open };
            let builder = UiBuilder::new().id_salt(id_salt).layout(*ui.layout());
            ui.scope_builder(builder, |ui| match output.borrow_mut().as_deref_mut() {
                Some(out) => bar.show(ui, out),
                None => bar.show(ui, &mut CropBarOutput::default()),
            });
        });
        slots.trailing(move |ui| {
            let labels = trailing_labels.unwrap_or_else(|| CropBarLabels::of(ui));
            let pressed = OptionsBar::commit_cancel(ui, &labels.commit, &labels.cancel);
            if let Some(out) = trailing_output.borrow_mut().as_deref_mut() {
                out.apply |= pressed.commit;
                out.discard |= pressed.cancel;
            }
        });
    }
}

/// What the ratio fields did this pass.
#[derive(Default)]
struct RatioPass {
    changed: bool,
    /// A field has the focus or is scrubbed.
    active: bool,
}

/// The body slot's cells.
struct Body<'a, 'l> {
    settings: &'a mut CropSettings,
    labels: &'l CropBarLabels,
    document: Option<&'a CropDocument>,
    user_presets: &'a [CropUserPreset],
    straighten: bool,
    options_open: Option<&'a mut bool>,
}

impl Body<'_, '_> {
    fn show(self, ui: &mut Ui, out: &mut CropBarOutput) {
        let Self { settings, labels, document, user_presets, straighten, options_open } = self;
        let sm = OptionsBar::CONTROL_SIZE;
        let numbers = LocaleNumbers::of(ui);
        let tokens = Tokens::of(ui);
        let field_width = OptionsBar::style(&tokens, false).fields[0];
        let preset_width = CropBar::style(&tokens).preset_width;
        let doc = document.copied().unwrap_or_else(|| CropDocument::new(CropDocKey(0), Size::ZERO));
        let has_document = document.is_some();
        let before = settings.clone();

        preset_list(ui, settings, labels, &doc, has_document, user_presets, preset_width, out);
        if settings.ratio == CropRatio::Size {
            size_fields(ui, settings, labels, &numbers, field_width, out);
        } else {
            ratio_fields(ui, settings, labels, &doc, has_document, &numbers, field_width, out);
        }
        OptionsBar::divider(ui);
        if ui.add(Button::new(labels.clear.as_str()).size(sm)).clicked() {
            settings.clear();
            out.ratio = Some(CropRatioChange { editing: false });
        }
        if ui.add(IconButton::new(Icon::Ruler, labels.straighten.as_str()).size(sm).selected(straighten).id_salt("straighten")).clicked() {
            out.straighten = true;
        }
        CropOverlayMenu::new("overlay", settings).labels(labels.clone()).show(ui);
        let kept_id = ui.make_persistent_id("options-open");
        let mut kept = ui.data(|d| d.get_temp::<bool>(kept_id)).unwrap_or(false);
        let own = options_open.is_none();
        let open = options_open.unwrap_or(&mut kept);
        let menu = CropOptionsMenu::new("options", &mut settings.shield).labels(labels.clone()).show(ui, open);
        if own {
            ui.data_mut(|d| d.insert_temp(kept_id, kept));
        }
        if menu.pick_color.is_some() {
            out.pick_color = menu.pick_color;
        }
        let _ = Checkbox::bool(labels.delete_pixels.as_str(), &mut settings.delete_pixels).size(sm).show(ui).response;
        OptionsBar::cell(ui, &labels.fill, CellWidth::Content, |ui, _| {
            let mut fill = settings.fill;
            let pick = Select::new("fill", labels.fill_name.as_str(), &mut fill)
                .option(CropFill::Background, labels.fill_background.as_str())
                .option(CropFill::ContentAware, labels.fill_content_aware.as_str())
                .size(sm)
                .show(ui);
            if let Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) = pick.event {
                settings.fill = value;
            }
        });
        out.settings_changed |= *settings != before;
    }
}

/// The name of ratio preset `i` of [`CROP_RATIOS`].
fn ratio_name(labels: &CropBarLabels, i: usize) -> &str {
    match i {
        0 => &labels.ratio_original,
        1 => &labels.ratio_square,
        2 => &labels.ratio_4_5,
        3 => &labels.ratio_5_7,
        4 => &labels.ratio_2_3,
        _ => &labels.ratio_16_9,
    }
}

/// The name of size preset `i` of [`CROP_SIZES`]: its values as New Crop Preset… would name them.
fn size_name(labels: &CropBarLabels, i: usize) -> String {
    let Some(preset) = CROP_SIZES.get(i) else { return String::new() };
    let size = CropSizeFields {
        width: preset.width.to_owned(),
        height: preset.height.to_owned(),
        resolution: preset.resolution.to_owned(),
        resolution_unit: ResolutionUnit::PixelsPerInch,
    };
    CropSettings { ratio: CropRatio::Size, size, ..CropSettings::default() }.preset_name(labels).unwrap_or_default()
}

/// The preset list: Ratio, W x H x Resolution, the ratio presets, the size presets, the user's
/// presets, Front Image, then New Crop Preset… and Delete Crop Preset….
#[expect(clippy::too_many_arguments, reason = "the bar's inputs")]
fn preset_list(
    ui: &mut Ui,
    settings: &mut CropSettings,
    labels: &CropBarLabels,
    doc: &CropDocument,
    has_document: bool,
    user_presets: &[CropUserPreset],
    width: f32,
    out: &mut CropBarOutput,
) {
    let current = settings.choice(user_presets);
    let mut shown = current;
    let mut select = Select::new("presets", labels.presets.as_str(), &mut shown)
        .option(CropPresetChoice::Free, labels.ratio.as_str())
        .option(CropPresetChoice::Size, labels.size_mode.as_str());
    for i in 0..CROP_RATIOS.len() {
        select = select.option(CropPresetChoice::Ratio(i), ratio_name(labels, i));
        if CROP_RATIOS.get(i) == Some(&CropRatio::Original) {
            select = select.option_enabled(has_document);
        }
    }
    for i in 0..CROP_SIZES.len() {
        select = select.option(CropPresetChoice::SizePreset(i), size_name(labels, i));
    }
    for (i, preset) in user_presets.iter().enumerate() {
        select = select.option(CropPresetChoice::User(i), preset.name.as_str());
    }
    select = select
        .option(CropPresetChoice::FrontImage, labels.front_image.as_str())
        .option_enabled(has_document)
        .separator()
        .option(CropPresetChoice::NewPreset, labels.new_preset.as_str())
        .option_enabled(settings.preset_name(labels).is_some())
        .option(CropPresetChoice::DeletePreset, labels.delete_preset.as_str())
        .option_enabled(!user_presets.is_empty())
        .size(OptionsBar::CONTROL_SIZE)
        .width(width);
    let pick = select.show(ui);
    let Some(EditEvent::Commit { value: choice, .. } | EditEvent::Set { value: choice, .. }) = pick.event else { return };
    match choice {
        CropPresetChoice::NewPreset => out.new_preset = true,
        CropPresetChoice::DeletePreset => out.delete_preset = true,
        choice => {
            if settings.choose(choice, doc, user_presets) {
                out.ratio = Some(CropRatioChange { editing: false });
            }
        }
    }
}

/// The two ratio fields with ⇄ between them.
#[expect(clippy::too_many_arguments, reason = "the bar's inputs")]
fn ratio_fields(
    ui: &mut Ui,
    settings: &mut CropSettings,
    labels: &CropBarLabels,
    doc: &CropDocument,
    has_document: bool,
    numbers: &LocaleNumbers,
    width: f32,
    out: &mut CropBarOutput,
) {
    let shown = settings.shown_ratio_fields(doc);
    let mut pass = RatioPass::default();
    ratio_field(ui, 0, &labels.width, shown, settings, numbers, width, &mut pass);
    let original = settings.ratio == CropRatio::Original;
    let swap = ui
        .add(
            IconButton::new(Icon::SwapHorizontal, labels.swap.as_str())
                .size(OptionsBar::CONTROL_SIZE)
                .enabled(has_document || !original)
                .id_salt("swap"),
        )
        .clicked();
    ratio_field(ui, 1, &labels.height, shown, settings, numbers, width, &mut pass);
    if swap {
        if original {
            settings.swap(doc);
        } else {
            settings.swap_fields();
        }
        pass.changed = true;
    }
    if pass.changed {
        out.ratio = Some(CropRatioChange { editing: pass.active });
    }
}

/// One ratio field: `shown` is both fields' numbers as drawn; an empty field is 0.
#[expect(clippy::too_many_arguments, reason = "the bar's inputs")]
fn ratio_field(
    ui: &mut Ui,
    side: usize,
    name: &str,
    shown: [f64; 2],
    settings: &mut CropSettings,
    numbers: &LocaleNumbers,
    width: f32,
    pass: &mut RatioPass,
) {
    let current = shown.get(side).copied().unwrap_or(0.0);
    let edit = NumberField::optional(("ratio-field", side), name, (current > 0.0).then_some(current), numbers)
        .range(0.0..=RATIO_MAX)
        .decimals(0..=RATIO_DECIMALS)
        .size(OptionsBar::CONTROL_SIZE)
        .width(width)
        .show_optional(ui);
    pass.active |= edit.response.has_focus() || edit.response.dragged();
    let start = edit.response.id.with("ratio-start");
    let apply = |settings: &mut CropSettings, pass: &mut RatioPass, value: Option<f64>| {
        let new = value.unwrap_or(0.0);
        if (new - current).abs() < 0.5 * 10f64.powi(-(RATIO_DECIMALS as i32)) {
            return;
        }
        // The other side keeps what the bar shows, so an Original ratio becomes its sides.
        if let Some(other) = settings.ratio_fields.get_mut(1 - side) {
            *other = shown.get(1 - side).copied().unwrap_or(0.0);
        }
        settings.set_ratio_field(side, new);
        pass.changed = true;
    };
    match edit.event {
        Some(EditEvent::Begin { value, .. }) => {
            ui.data_mut(|d| d.insert_temp(start, (settings.ratio, settings.ratio_fields)));
            if let Some(value) = value {
                apply(settings, pass, value);
            }
        }
        Some(EditEvent::Cancel { .. }) => {
            if let Some((ratio, fields)) = ui.data_mut(|d| d.remove_temp::<(CropRatio, [f64; 2])>(start)) {
                settings.ratio = ratio;
                settings.ratio_fields = fields;
                pass.changed = true;
            }
        }
        Some(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => apply(settings, pass, value),
        Some(_) | None => {}
    }
}

/// W, ⇄, H, Resolution and its unit, in W × H × Resolution mode.
fn size_fields(ui: &mut Ui, settings: &mut CropSettings, labels: &CropBarLabels, numbers: &LocaleNumbers, width: f32, out: &mut CropBarOutput) {
    let sm = OptionsBar::CONTROL_SIZE;
    let mut changed = false;
    changed |= length_field(ui, "width", &labels.width, &mut settings.size.width, width);
    if ui.add(IconButton::new(Icon::SwapHorizontal, labels.swap.as_str()).size(sm).id_salt("swap")).clicked() {
        settings.swap_fields();
        changed = true;
    }
    changed |= length_field(ui, "height", &labels.height, &mut settings.size.height, width);

    let resolution = settings.size.resolution.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0);
    let edit = NumberField::optional("resolution", labels.resolution.as_str(), resolution, numbers)
        .range(0.0..=FIELD_MAX)
        .decimals(0..=2)
        .size(sm)
        .width(width)
        .show_optional(ui);
    if let Some(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) = edit.event {
        let text = match value.filter(|v| *v > 0.0) {
            Some(v) => CropSizeFields::normalize_resolution(&v.to_string(), &settings.size.resolution),
            None => String::new(),
        };
        if text != settings.size.resolution {
            settings.size.resolution = text;
            changed = true;
        }
    }

    let mut unit = settings.size.resolution_unit;
    let pick = Select::new("resolution-unit", labels.resolution_unit.as_str(), &mut unit)
        .option(ResolutionUnit::PixelsPerInch, labels.pixels_per_inch.as_str())
        .option(ResolutionUnit::PixelsPerCentimetre, labels.pixels_per_centimetre.as_str())
        .size(sm)
        .show(ui);
    if let Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) = pick.event
        && value != settings.size.resolution_unit
    {
        // The same resolution in the other unit, as in Photoshop.
        if let Some(v) = resolution {
            let converted = if value == ResolutionUnit::PixelsPerCentimetre { v / 2.54 } else { v * 2.54 };
            settings.size.resolution = CropSizeFields::normalize_resolution(&converted.to_string(), &settings.size.resolution);
        }
        settings.size.resolution_unit = value;
        changed = true;
    }
    if changed {
        out.ratio = Some(CropRatioChange { editing: false });
    }
}

/// A length field: what is typed stays while the field has the focus and becomes the normalised
/// length when the typing run ends. Returns whether `value` changed.
fn length_field(ui: &mut Ui, salt: &'static str, name: &str, value: &mut String, width: f32) -> bool {
    let typed_id = ui.scope_id().with((salt, "typed"));
    let typed = ui.data(|d| d.get_temp::<String>(typed_id));
    let edit = TextField::new(salt, name, typed.as_deref().unwrap_or(value))
        .placeholder(name)
        .size(OptionsBar::CONTROL_SIZE)
        .width(width)
        .char_limit(32)
        .show(ui);
    match edit.event {
        Some(EditEvent::Begin { value: Some(text), .. } | EditEvent::Update { value: text, .. }) => {
            ui.data_mut(|d| d.insert_temp(typed_id, text));
        }
        Some(EditEvent::Commit { value: text, .. } | EditEvent::Set { value: text, .. }) => {
            ui.data_mut(|d| d.remove_temp::<String>(typed_id));
            let next = CropSizeFields::normalize_length(&text, value);
            let changed = next != *value;
            *value = next;
            return changed;
        }
        Some(EditEvent::Cancel { .. }) => {
            ui.data_mut(|d| d.remove_temp::<String>(typed_id));
        }
        _ => {}
    }
    false
}
