//! Harmony controls (F04.16): [`HarmonyWheel`], the colours of a harmony as markers on a hue and
//! saturation wheel, and [`HarmonyPanel`], VectorCraft's Color Guide (rule, harmony strip,
//! variation grid). The math is [`crate::harmony`].

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::{AsIdSalt, IdSalt, Ui};

use crate::harmony::{GuideOptions, Harmony};

craft_ui::words! {
    /// The harmony controls' words, English by default. The rules' names are [`HARMONY_WORDS`].
    #[non_exhaustive]
    pub struct HarmonyLabels {
        rule: "harmony.rule" = "Rule:";
        custom: "harmony.custom" = "Custom";
        brightness: "harmony.brightness" = "Brightness";
        set_base: "harmony.setBase" = "Set base color to the current color";
        show_tints_shades: "harmony.showTintsShades" = "Show Tints/Shades";
        show_warm_cool: "harmony.showWarmCool" = "Show Warm/Cool";
        show_vivid_muted: "harmony.showVividMuted" = "Show Vivid/Muted";
        save_as_swatches: "harmony.saveAsSwatches" = "Save Colors as Swatches";
        options: "harmony.options" = "Color Guide Options…";
        shades: "harmony.shades" = "Shades";
        tints: "harmony.tints" = "Tints";
        cool: "harmony.cool" = "Cool";
        warm: "harmony.warm" = "Warm";
        muted: "harmony.muted" = "Muted";
        vivid: "harmony.vivid" = "Vivid";
    }
}

/// The rules' names as words, keyed `harmony.rule.<camelCase id>` with [`Harmony::label`]'s
/// English (21 entries, [`Harmony::ALL`]'s order).
// inv-vectorcraft: crates/color/src/harmony.rs:101 `label`, :128 `id`.
pub const HARMONY_WORDS: &[craft_ui::localized::Word] = &[
    craft_ui::localized::Word::keyed("harmony.rule.complementary", "Complementary"),
    craft_ui::localized::Word::keyed("harmony.rule.complementary2", "Complementary 2"),
    craft_ui::localized::Word::keyed("harmony.rule.splitComplementary", "Split Complementary"),
    craft_ui::localized::Word::keyed("harmony.rule.leftComplement", "Left Complement"),
    craft_ui::localized::Word::keyed("harmony.rule.rightComplement", "Right Complement"),
    craft_ui::localized::Word::keyed("harmony.rule.analogous", "Analogous"),
    craft_ui::localized::Word::keyed("harmony.rule.analogous2", "Analogous 2"),
    craft_ui::localized::Word::keyed("harmony.rule.monochromatic", "Monochromatic"),
    craft_ui::localized::Word::keyed("harmony.rule.shades", "Shades"),
    craft_ui::localized::Word::keyed("harmony.rule.triad", "Triad"),
    craft_ui::localized::Word::keyed("harmony.rule.triad2", "Triad 2"),
    craft_ui::localized::Word::keyed("harmony.rule.triad3", "Triad 3"),
    craft_ui::localized::Word::keyed("harmony.rule.tetrad", "Tetrad"),
    craft_ui::localized::Word::keyed("harmony.rule.tetrad2", "Tetrad 2"),
    craft_ui::localized::Word::keyed("harmony.rule.tetrad3", "Tetrad 3"),
    craft_ui::localized::Word::keyed("harmony.rule.compound", "Compound"),
    craft_ui::localized::Word::keyed("harmony.rule.compound2", "Compound 2"),
    craft_ui::localized::Word::keyed("harmony.rule.highContrast", "High Contrast"),
    craft_ui::localized::Word::keyed("harmony.rule.highContrast2", "High Contrast 2"),
    craft_ui::localized::Word::keyed("harmony.rule.highContrast3", "High Contrast 3"),
    craft_ui::localized::Word::keyed("harmony.rule.pentagram", "Pentagram"),
];

/// What [`HarmonyWheel::show`] returns.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct HarmonyWheelResponse {
    /// The colour whose marker was pressed; it becomes the selected one.
    pub pressed: Option<usize>,
    /// A colour moved: its marker dragged (new hue and saturation) or, for the selected colour,
    /// the Brightness slider (new brightness); `[h°, s, b]` with `None` where it kept its value.
    /// The host applies it with [`move_on_wheel`](crate::harmony::move_on_wheel), linked while a
    /// rule is chosen.
    pub moved: Option<(usize, [Option<f32>; 3])>,
}

/// A hue and saturation wheel at the selected colour's brightness (hue around, saturation
/// outwards), a marker per colour joined to the centre by a line (the selected one larger), and
/// a Brightness slider for the selected colour under it.
///
/// - Press a marker to select it; drag it to move that colour. Arrow keys move the selected
///   marker (←/→ hue by 1°, ↑/↓ saturation by 1 %), Tab moves between markers.
/// - AccessKit: a `Group` named by `label`; each marker a `Slider`-like node named by its hex.
pub struct HarmonyWheel<'a> {
    id_salt: IdSalt,
    label: String,
    colors: &'a [Srgba],
    selected: Option<usize>,
    size: f32,
}

impl<'a> HarmonyWheel<'a> {
    /// A wheel over `colors`; its side is the `color_picker.area` token unless set.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, colors: &'a [Srgba]) -> HarmonyWheel<'a> {
        HarmonyWheel { id_salt: IdSalt::new(id_salt), label: label.into(), colors, selected: None, size: 0.0 }
    }

    pub fn selected(mut self, selected: Option<usize>) -> HarmonyWheel<'a> {
        self.selected = selected;
        self
    }

    /// The wheel's side in points.
    pub fn size(mut self, size: f32) -> HarmonyWheel<'a> {
        self.size = size;
        self
    }

    // inv-vectorcraft: crates/ui-egui/src/widgets/color.rs:485 `harmony_wheel`; built on
    // `ColorWheel`'s ring geometry where it fits (`WheelGeometry`).
    pub fn show(self, ui: &mut Ui) -> HarmonyWheelResponse {
        todo!()
    }
}

/// The Color Guide panel's settings, kept across runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HarmonyPanelState {
    pub rule: Harmony,
    pub options: GuideOptions,
}

/// A Color Guide panel menu entry the app carries out.
#[derive(Clone, Debug, PartialEq)]
pub enum HarmonyMenuAction {
    /// Save Colors as Swatches: the selected cells' colours, in grid order.
    SaveAsSwatches(Vec<Srgba>),
    /// Color Guide Options…: the app opens its options dialog (steps 1–20 and the Less–More
    /// amount of [`GuideOptions`]).
    Options,
}

/// What [`HarmonyPanel::show`] returns.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct HarmonyPanelResponse {
    /// A colour of the strip or the grid was clicked: the app applies it to the selection or the
    /// current colour.
    pub chosen: Option<Srgba>,
    /// The base swatch was clicked: the app sets the base to its current colour.
    pub set_base: bool,
}

/// VectorCraft's Color Guide: the base swatch (26 points; a click sets the base to the current
/// colour) and the rule [`Select`](craft_ui::controls::Select); the harmony strip (20 points
/// high, one cell per colour, tooltip the hex); the variation grid, one row per harmony colour
/// from shades to tints (or cool to warm, muted to vivid), the harmony colour itself in the centre
/// column outlined; the grid's end labels. A click on any cell chooses its colour; Shift or Cmd
/// selects several for Save Colors as Swatches.
pub struct HarmonyPanel<'a> {
    id_salt: IdSalt,
    state: &'a mut HarmonyPanelState,
    base: Srgba,
    limit: Option<&'a [Srgba]>,
}

impl<'a> HarmonyPanel<'a> {
    /// The guide for `base`.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut HarmonyPanelState, base: Srgba) -> HarmonyPanel<'a> {
        HarmonyPanel { id_salt: IdSalt::new(id_salt), state, base, limit: None }
    }

    /// Limit to Library: every colour snaps to the nearest of `palette`
    /// ([`Guide::limited`](crate::harmony::Guide::limited)).
    pub fn limit(mut self, palette: &'a [Srgba]) -> HarmonyPanel<'a> {
        self.limit = Some(palette);
        self
    }

    // inv-vectorcraft: crates/ui-egui/src/panels/color_guide.rs:137 `show`, :66 `guide`.
    pub fn show(self, ui: &mut Ui) -> HarmonyPanelResponse {
        todo!()
    }

    /// The panel menu: Show Tints/Shades, Show Warm/Cool, Show Vivid/Muted (the current one
    /// checked), a separator, Save Colors as Swatches (enabled while the panel `id_salt` has cells
    /// selected) and Color Guide Options…. The variation is set here; the other two come back as
    /// a [`HarmonyMenuAction`].
    // inv-vectorcraft: crates/ui-egui/src/panels/color_guide.rs:257 `menu`, :129 `save_selected`.
    pub fn panel_menu(ui: &mut Ui, id_salt: impl AsIdSalt, state: &mut HarmonyPanelState) -> Option<HarmonyMenuAction> {
        todo!()
    }
}
