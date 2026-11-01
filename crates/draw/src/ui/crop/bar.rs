//! [`CropOptionsBar`]: the Crop tool's options in the body and trailing slots of
//! [`OptionsBar`](craft_ui::patterns::OptionsBar).

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TO-C5)")]

use egui::{AsIdSalt, IdSalt};

use super::options::{CropSettings, CropUserPreset};
use super::session::CropDocument;
use craft_ui::patterns::OptionsBarSlots;

craft_ui::words! {
    /// The words of the Crop tool's options bar and its overlay and gear menus, English by
    /// default (Photoshop's names and tooltips).
    #[non_exhaustive]
    pub struct CropBarLabels {
        /// The preset list's accessible name.
        presets: "cropBar.presets" = "Crop ratio";
        ratio: "cropBar.ratio" = "Ratio";
        size_mode: "cropBar.sizeMode" = "W x H x Resolution";
        /// The names of [`CROP_RATIOS`](super::CROP_RATIOS), in its order.
        ratio_original: "cropBar.ratioOriginal" = "Original Ratio";
        ratio_square: "cropBar.ratioSquare" = "1 : 1 (Square)";
        ratio_4_5: "cropBar.ratio4x5" = "4 : 5 (8 : 10)";
        ratio_5_7: "cropBar.ratio5x7" = "5 : 7";
        ratio_2_3: "cropBar.ratio2x3" = "2 : 3 (4 : 6)";
        ratio_16_9: "cropBar.ratio16x9" = "16 : 9";
        front_image: "cropBar.frontImage" = "Front Image";
        new_preset: "cropBar.newPreset" = "New Crop Preset…";
        delete_preset: "cropBar.deletePreset" = "Delete Crop Preset…";
        /// The unit word of a px/in resolution in a preset name ("4 x 5 in 300 ppi").
        ppi: "cropBar.ppi" = "ppi";
        pixels_per_inch: "cropBar.pixelsPerInch" = "px/in";
        pixels_per_centimetre: "cropBar.pixelsPerCentimeter" = "px/cm";
        width: "cropBar.width" = "Width";
        height: "cropBar.height" = "Height";
        resolution: "cropBar.resolution" = "Resolution";
        resolution_unit: "cropBar.resolutionUnit" = "Resolution unit";
        swap: "cropBar.swap" = "Swaps height and width";
        clear: "cropBar.clear" = "Clear";
        straighten: "cropBar.straighten" = "Straighten the image by drawing a line on it";
        overlay: "cropBar.overlay" = "Set the overlay options for the Crop tool";
        /// The overlays in [`CropGuide::ALL`](super::CropGuide::ALL) order.
        thirds: "cropBar.thirds" = "Rule of Thirds";
        grid: "cropBar.grid" = "Grid";
        diagonal: "cropBar.diagonal" = "Diagonal";
        triangle: "cropBar.triangle" = "Triangle";
        golden_ratio: "cropBar.goldenRatio" = "Golden Ratio";
        golden_spiral: "cropBar.goldenSpiral" = "Golden Spiral";
        /// The show rules in [`GuideShow::ALL`](super::GuideShow::ALL) order.
        show_auto: "cropBar.showAuto" = "Auto Show Overlay";
        show_always: "cropBar.showAlways" = "Always Show Overlay";
        show_never: "cropBar.showNever" = "Never Show Overlay";
        cycle_overlay: "cropBar.cycleOverlay" = "Cycle Overlay";
        cycle_orientation: "cropBar.cycleOrientation" = "Cycle Orientation";
        options: "cropBar.options" = "Set additional Crop options";
        classic_mode: "cropBar.classicMode" = "Use Classic Mode";
        show_cropped_area: "cropBar.showCroppedArea" = "Show Cropped Area";
        auto_center: "cropBar.autoCenter" = "Auto Center Preview";
        shield: "cropBar.shield" = "Enable Crop Shield";
        color: "cropBar.color" = "Color:";
        match_canvas: "cropBar.matchCanvas" = "Match Canvas";
        custom: "cropBar.custom" = "Custom…";
        shield_color: "cropBar.shieldColor" = "Crop Shield color";
        opacity: "cropBar.opacity" = "Opacity:";
        auto_adjust: "cropBar.autoAdjust" = "Auto Adjust Opacity";
        delete_pixels: "cropBar.deletePixels" = "Delete Cropped Pixels";
        fill: "cropBar.fill" = "Fill:";
        fill_background: "cropBar.fillBackground" = "Background (Default)";
        fill_content_aware: "cropBar.fillContentAware" = "Content-Aware Fill";
        commit: "cropBar.commit" = "Commit current crop operation";
        cancel: "cropBar.cancel" = "Cancel current crop operation";
    }
}

/// What the bar asks of the app or the tool. Edits of the options go straight into the settings
/// the bar was given; these are the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CropBarEvent {
    /// The ratio, a ratio field, a preset or W × H × Resolution changed: the app calls
    /// [`CropTool::refit`](super::CropTool::refit). `editing` is true while a ratio field still has
    /// the focus or is scrubbed, so the frame refits from the frame the edit began with (#7752).
    Ratio { editing: bool },
    /// The Straighten button: [`CropTool::toggle_straighten`](super::CropTool::toggle_straighten).
    Straighten,
    /// ✓: [`CropTool::apply`](super::CropTool::apply).
    Commit,
    /// ⊘: [`CropTool::discard`](super::CropTool::discard).
    Cancel,
    /// New Crop Preset…: the app opens its dialog with [`CropSettings::preset_name`].
    NewPreset,
    /// Delete Crop Preset…: the app opens its dialog.
    DeletePreset,
    /// The gear menu's Custom… or the shield swatch: the app asks for a colour
    /// ([`ColorRequest`](craft_ui::dialog::ColorRequest)) and sets
    /// [`CropShield::custom_color`](super::CropShield::custom_color).
    PickShieldColor,
}

/// What the bar reported this pass, in order: the body's events, then ✓ or ⊘.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CropBarOutput {
    /// The body's events.
    pub events: Vec<CropBarEvent>,
    /// The trailing slot's [`CropBarEvent::Commit`] or [`CropBarEvent::Cancel`].
    pub end: Option<CropBarEvent>,
}

impl CropBarOutput {
    /// Every event of the pass in order, emptying the output.
    pub fn take(&mut self) -> Vec<CropBarEvent> {
        let mut events = std::mem::take(&mut self.events);
        events.extend(self.end.take());
        events
    }
}

/// The Crop tool's options bar body and trailing slot.
///
/// ```no_run
/// use craft_draw::ui::crop::{CropBarOutput, CropOptionsBar, CropSettings};
/// use craft_ui::patterns::OptionsBar;
/// # fn demo(ui: &mut egui::Ui, settings: &mut CropSettings, armed: bool) {
/// let mut out = CropBarOutput::default();
/// OptionsBar::new("options").show(ui, |bar| {
///     CropOptionsBar::new("crop", settings).straighten(armed).output(&mut out).add_to(bar);
/// });
/// for event in out.take() {
///     // CropBarEvent::{Ratio, Straighten, Commit, Cancel, NewPreset, DeletePreset, PickShieldColor}
///     # let _ = event;
/// }
/// # }
/// ```
///
/// - Order (PhotoCraft's): the preset list (Ratio, W x H x Resolution, the ratio presets, the size
///   presets, the user's presets, Front Image, a separator, New Crop Preset…, Delete Crop
///   Preset…; `CellWidth::Content`, PhotoCraft's 140 wide); the W and H
///   ratio fields (`CellWidth::Sm`, 0 to 99,999 with 3 decimals) with ⇄ between them, or in
///   W × H × Resolution mode the W, H and Resolution text fields with the px/in or px/cm select;
///   a divider; Clear; the Straighten toggle (`Icon::Ruler`, pressed while armed); the overlay menu
///   ([`CropOverlayMenu`](super::CropOverlayMenu)); the gear menu
///   ([`CropOptionsMenu`](super::CropOptionsMenu)); the Delete Cropped Pixels check box; Fill and
///   its select (`CellWidth::Content`). The trailing slot holds ⊘ and ✓
///   ([`OptionsBar::commit_cancel`](craft_ui::patterns::OptionsBar::commit_cancel)).
/// - Length fields keep what is typed while they have the focus and normalise it when the typing
///   run ends ([`CropSizeFields::normalize_length`](super::CropSizeFields::normalize_length)).
/// - Delete Crop Preset… is disabled when no user preset is given; New Crop Preset… when
///   [`CropSettings::preset_name`] is `None`.
/// - **Keyboard and AccessKit:** cells in reading order; the list is a `Select` named "Crop ratio"
///   with the chosen entry as its value; the fields are named Width, Height and Resolution; the
///   Straighten toggle has its pressed state; the menus open with Enter or Space.
pub struct CropOptionsBar<'a> {
    id_salt: IdSalt,
    settings: &'a mut CropSettings,
    labels: Option<CropBarLabels>,
    document: Option<&'a CropDocument>,
    user_presets: &'a [CropUserPreset],
    straighten: bool,
    output: Option<&'a mut CropBarOutput>,
}

impl<'a> CropOptionsBar<'a> {
    /// The bar editing `settings`.
    pub fn new(id_salt: impl AsIdSalt, settings: &'a mut CropSettings) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, labels: None, document: None, user_presets: &[], straighten: false, output: None }
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

    /// Where the bar reports its events.
    pub fn output(mut self, output: &'a mut CropBarOutput) -> Self {
        self.output = Some(output);
        self
    }

    /// Fills the bar's body and trailing slots.
    pub fn add_to(self, slots: &mut OptionsBarSlots<'a>) {
        // pc: crates/ui-egui/src/panels/options_bar.rs:465-555 (the Crop branch); crop_presets.rs:137
        // `dropdown`; crop_size.rs:316 `fields`, :287 `text_field`; crop_straighten.rs:166
        // `options_button`. The two slots get disjoint halves of the output (`events` to the body,
        // `end` to the trailing slot).
        todo!()
    }
}
