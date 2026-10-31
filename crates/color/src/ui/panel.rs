//! [`ColorPanel`] (F04.10): Photoshop's Color panel. The foreground and background wells, one of
//! nine views of the edited colour (two cubes, a wheel, five slider sets), the out-of-gamut and
//! web-colour warnings, and the colour ramp under the sliders; the panel menu chooses the view and
//! the ramp and copies the colour.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_cms::convert::CmykConverter;
use craft_theme::Srgba;
use egui::{AsIdSalt, IdSalt, Ui};

use super::{PickedColor, RampKind};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;

craft_ui::words! {
    /// The Color panel's words, English by default.
    #[non_exhaustive]
    pub struct ColorPanelLabels {
        hue_cube: "colorPanel.hueCube" = "Hue Cube";
        brightness_cube: "colorPanel.brightnessCube" = "Brightness Cube";
        color_wheel: "colorPanel.colorWheel" = "Color Wheel";
        grayscale_slider: "colorPanel.grayscaleSlider" = "Grayscale Slider";
        rgb_sliders: "colorPanel.rgbSliders" = "RGB Sliders";
        hsb_sliders: "colorPanel.hsbSliders" = "HSB Sliders";
        cmyk_sliders: "colorPanel.cmykSliders" = "CMYK Sliders";
        lab_sliders: "colorPanel.labSliders" = "Lab Sliders";
        web_color_sliders: "colorPanel.webColorSliders" = "Web Color Sliders";
        copy_html: "colorPanel.copyHtml" = "Copy Color as HTML";
        copy_hex: "colorPanel.copyHex" = "Copy Color's Hex Code";
        rgb_spectrum: "colorPanel.rgbSpectrum" = "RGB Spectrum";
        cmyk_spectrum: "colorPanel.cmykSpectrum" = "CMYK Spectrum";
        grayscale_ramp: "colorPanel.grayscaleRamp" = "Grayscale Ramp";
        current_colors: "colorPanel.currentColors" = "Current Colors";
        web_safe_ramp: "colorPanel.webSafeRamp" = "Make Ramp Web Safe";
        color_ramp: "colorPanel.colorRamp" = "Color Ramp";
        foreground: "colorPanel.foreground" = "Foreground colour";
        background: "colorPanel.background" = "Background colour";
    }
}

/// What the panel shows: Photoshop's panel menu choices, in its order. Hue Cube is the default;
/// the theme never changes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorPanelMode {
    #[default]
    HueCube,
    BrightnessCube,
    ColorWheel,
    GrayscaleSlider,
    RgbSliders,
    HsbSliders,
    CmykSliders,
    LabSliders,
    WebColorSliders,
}

impl ColorPanelMode {
    /// Every mode, in the panel menu's order (one section, no separator).
    pub const ALL: [ColorPanelMode; 9] = [
        ColorPanelMode::HueCube,
        ColorPanelMode::BrightnessCube,
        ColorPanelMode::ColorWheel,
        ColorPanelMode::GrayscaleSlider,
        ColorPanelMode::RgbSliders,
        ColorPanelMode::HsbSliders,
        ColorPanelMode::CmykSliders,
        ColorPanelMode::LabSliders,
        ColorPanelMode::WebColorSliders,
    ];

    /// The slider modes, which show the ramp under their sliders.
    pub fn is_sliders(self) -> bool {
        !matches!(self, ColorPanelMode::HueCube | ColorPanelMode::BrightnessCube | ColorPanelMode::ColorWheel)
    }

    /// The menu item.
    // pc: crates/ui-egui/src/state/color_panel.rs:70 `label`.
    pub fn label(self, labels: &ColorPanelLabels) -> String {
        todo!()
    }
}

/// The ramp under the sliders. Shift-click on the ramp cycles them in this order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorPanelRamp {
    #[default]
    RgbSpectrum,
    CmykSpectrum,
    GrayscaleRamp,
    CurrentColors,
}

impl ColorPanelRamp {
    pub const ALL: [ColorPanelRamp; 4] =
        [ColorPanelRamp::RgbSpectrum, ColorPanelRamp::CmykSpectrum, ColorPanelRamp::GrayscaleRamp, ColorPanelRamp::CurrentColors];

    /// The next ramp of the Shift-click cycle (Current Colors goes back to RGB Spectrum).
    pub fn next(self) -> ColorPanelRamp {
        let i = ColorPanelRamp::ALL.iter().position(|r| *r == self).unwrap_or(0);
        ColorPanelRamp::ALL.get(i + 1).copied().unwrap_or(ColorPanelRamp::RgbSpectrum)
    }

    /// The [`ColorRamp`](super::ColorRamp) kind that draws it: Spectrum, CmykSpectrum,
    /// Grayscale, or Between the foreground and background.
    pub fn kind(self) -> RampKind {
        match self {
            ColorPanelRamp::RgbSpectrum => RampKind::Spectrum,
            ColorPanelRamp::CmykSpectrum => RampKind::CmykSpectrum,
            ColorPanelRamp::GrayscaleRamp => RampKind::Grayscale,
            ColorPanelRamp::CurrentColors => RampKind::Between,
        }
    }

    /// The menu item.
    // pc: crates/ui-egui/src/state/color_panel.rs:106 `label`.
    pub fn label(self, labels: &ColorPanelLabels) -> String {
        todo!()
    }
}

/// The Color panel's settings, kept across runs (settings section `color_panel`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ColorPanelState {
    /// The panel edits the background colour (its well was clicked), not the foreground.
    pub background: bool,
    pub mode: ColorPanelMode,
    pub ramp: ColorPanelRamp,
    /// Make Ramp Web Safe: the ramp shows and picks web colours only.
    pub web_safe_ramp: bool,
}

impl craft_host::Section for ColorPanelState {
    const KEY: &'static str = "color_panel";
}

/// Which colour a [`ColorPanel`] edit is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelTarget {
    Foreground,
    Background,
}

/// What [`ColorPanel::show`] returns.
#[derive(Debug, Default)]
#[non_exhaustive]
#[must_use = "the host applies the edit"]
pub struct ColorPanelResponse {
    /// An edit of the foreground or background colour (from the view, the sliders, the ramp or a
    /// warning chip), with the colour it is for.
    pub edit: Option<(PanelTarget, EditEvent<PickedColor>)>,
    /// A well was clicked while it was already the edited one: the app opens the
    /// [`ColorPickerDialog`](super::ColorPickerDialog) for it (Photoshop).
    pub open_picker: Option<PanelTarget>,
}

/// The Color panel.
///
/// - Wells: the foreground over the background ([`SwatchPair`](super::SwatchPair)); a click on
///   the other well makes it the edited one ([`ColorPanelState::background`]); a click on the
///   edited one asks for the picker. The swap and default buttons are the pair's.
/// - Views: Hue Cube and Brightness Cube are [`ColorPicker`](super::ColorPicker)'s Compact area
///   and strip (hue or brightness in the strip); Color Wheel is [`ColorWheel`](super::ColorWheel);
///   the slider modes are the Compact picker's channel rows of that model (Grayscale is
///   [`GrayModel`](crate::GrayModel), CMYK is [`CmykModel`](crate::CmykModel) through the
///   converter, Web Color is RGB with hex-pair fields and sliders snapping to web colours).
///   Slider values show rounded to whole numbers, as Photoshop's fields.
/// - Warnings: in the slider modes, the out-of-gamut and web-colour chips beside the wells, as the
///   picker's; a click takes the nearest colour.
/// - Ramp: in the slider modes, a [`ColorRamp`](super::ColorRamp) of the chosen kind with the
///   white and black boxes at its end; a press picks for the edited well, Alt picks for the
///   other; Shift-click cycles the kind. A drag keeps the Current Colors ends it began with.
/// - Focus: the panel never takes the focus when shown (the picker's `focus_on_open(false)`).
/// - AccessKit: a `Group` named "Color"; the ramp a `Slider`-like image named "Color Ramp".
pub struct ColorPanel<'a> {
    id_salt: IdSalt,
    state: &'a mut ColorPanelState,
    colors: [Srgba; 2],
    numbers: &'a LocaleNumbers,
    gamut: Option<&'a CmykConverter>,
    enabled: bool,
}

impl<'a> ColorPanel<'a> {
    /// The panel on the app's `[foreground, background]`.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut ColorPanelState, colors: [Srgba; 2], numbers: &'a LocaleNumbers) -> ColorPanel<'a> {
        ColorPanel { id_salt: IdSalt::new(id_salt), state, colors, numbers, gamut: None, enabled: true }
    }

    /// The document's CMYK converter, for the CMYK sliders, the CMYK Spectrum ramp and the gamut
    /// warning; without it they use the built-in coated CMYK.
    pub fn gamut(mut self, converter: &'a CmykConverter) -> ColorPanel<'a> {
        self.gamut = Some(converter);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> ColorPanel<'a> {
        self.enabled = enabled;
        self
    }

    // pc: crates/ui-egui/src/panels/color_panel_ui.rs:36 `panel` (dispatch by mode), :320
    // `sliders`, :350 `alerts`, :471 `ramp` (white/black boxes, Alt target, drag-held ends), :538
    // `wheel`; crates/ui-egui/src/panels.rs `color_field` for the cubes. Built on `SwatchPair`,
    // `ColorPicker` Compact, `ColorWheel`, `ColorRamp`.
    pub fn show(self, ui: &mut Ui) -> ColorPanelResponse {
        todo!()
    }

    /// The panel menu's entries, for the docking host's `panel_menu` hook
    /// ([`DockContent::panel_menu`](craft_ui::docking::DockContent::panel_menu)): the nine modes
    /// with a check mark on the current one, a separator, Copy Color as HTML (`color="#rrggbb"`)
    /// and Copy Color's Hex Code (`rrggbb`) of the edited colour to the clipboard, a separator,
    /// the four ramps with a check mark, a separator, Make Ramp Web Safe (checked when on).
    // pc: crates/ui-egui/src/panels/color_panel_ui.rs:750 `panel_menu`, :735 `ramp_menu`, :719
    // `html_color`, :724 `hex_code`.
    pub fn panel_menu(ui: &mut Ui, state: &mut ColorPanelState, edited: Srgba) {
        todo!()
    }
}
