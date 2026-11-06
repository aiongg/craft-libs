//! [`ColorPanel`] (F04.10): Photoshop's Color panel. The foreground and background wells, one of
//! nine views of the edited colour (two cubes, a wheel, five slider sets), the out-of-gamut and
//! web-colour warnings, and the colour ramp under the sliders; the panel menu chooses the view and
//! the ramp and copies the colour.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use craft_cms::convert::CmykConverter;
use craft_theme::{ColorModel, ControlSize, HsbModel, LabModel, ModelValues, RgbModel, Srgba, TypeRole};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{Align, AsIdSalt, Color32, Id, IdSalt, Layout, Modifiers, Rect, Ui, UiBuilder, vec2};

use super::picker::{Inner, RowRule, row_line, row_number, texts as picker_texts};
use super::{
    AreaMode, ColorArea, ColorPickerLabels, ColorRamp, ColorStrip, ColorWarning, ColorWarningChip, ColorWheel, HexField, PairWell, PickedColor,
    RampKind, SwatchPair, Well,
};
use crate::{CmykModel, GrayModel, not_web_safe};
use craft_ui::controls::{IconButton, NumberField};
use craft_ui::dialog::ColorRequest;
use craft_ui::direction;
use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::edit::{EditEvent, EditPolicy};
use craft_ui::icons::Icon;
use craft_ui::localized::{LocaleNumbers, SharedLabels, Unit, Word, Words};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use craft_ui::primitives::{Content, Gap, Row, Text};
use craft_ui::theme::{Tokens, TokensOf};

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
        foreground: "colorPanel.foreground" = "Foreground color";
        background: "colorPanel.background" = "Background color";
        wells: "colorPanel.wells" = "Colors";
        add_to_swatches: "colorPanel.addToSwatches" = "Add to Swatches";
        create_swatch: "colorPanel.createSwatch" = "Create new swatch";
        /// The title of the Color Picker opened for the foreground well.
        picker_foreground: "colorPanel.pickerForeground" = "Color Picker (Foreground Color)";
        /// The title of the Color Picker opened for the background well.
        picker_background: "colorPanel.pickerBackground" = "Color Picker (Background Color)";
    }
}

/// What the panel shows: Photoshop's panel menu choices, in its order. Hue Cube is the default;
/// the theme never changes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorPanelMode {
    #[default]
    #[allow(missing_docs)]
    HueCube,
    #[allow(missing_docs)]
    BrightnessCube,
    #[allow(missing_docs)]
    ColorWheel,
    #[allow(missing_docs)]
    GrayscaleSlider,
    #[allow(missing_docs)]
    RgbSliders,
    #[allow(missing_docs)]
    HsbSliders,
    #[allow(missing_docs)]
    CmykSliders,
    #[allow(missing_docs)]
    LabSliders,
    #[allow(missing_docs)]
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

    /// Hue Cube and Brightness Cube.
    pub fn is_cube(self) -> bool {
        matches!(self, ColorPanelMode::HueCube | ColorPanelMode::BrightnessCube)
    }

    /// The slider modes, which show the ramp under their sliders.
    pub fn is_sliders(self) -> bool {
        !matches!(self, ColorPanelMode::HueCube | ColorPanelMode::BrightnessCube | ColorPanelMode::ColorWheel)
    }

    /// The menu item.
    // pc: crates/ui-egui/src/state/color_panel.rs:70 `label`.
    pub fn label(self, labels: &ColorPanelLabels) -> String {
        match self {
            ColorPanelMode::HueCube => &labels.hue_cube,
            ColorPanelMode::BrightnessCube => &labels.brightness_cube,
            ColorPanelMode::ColorWheel => &labels.color_wheel,
            ColorPanelMode::GrayscaleSlider => &labels.grayscale_slider,
            ColorPanelMode::RgbSliders => &labels.rgb_sliders,
            ColorPanelMode::HsbSliders => &labels.hsb_sliders,
            ColorPanelMode::CmykSliders => &labels.cmyk_sliders,
            ColorPanelMode::LabSliders => &labels.lab_sliders,
            ColorPanelMode::WebColorSliders => &labels.web_color_sliders,
        }
        .clone()
    }
}

/// The ramp under the sliders. Shift-click on the ramp cycles them in this order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorPanelRamp {
    #[default]
    #[allow(missing_docs)]
    RgbSpectrum,
    #[allow(missing_docs)]
    CmykSpectrum,
    #[allow(missing_docs)]
    GrayscaleRamp,
    #[allow(missing_docs)]
    CurrentColors,
}

impl ColorPanelRamp {
    /// All ramps, in menu order.
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
        match self {
            ColorPanelRamp::RgbSpectrum => &labels.rgb_spectrum,
            ColorPanelRamp::CmykSpectrum => &labels.cmyk_spectrum,
            ColorPanelRamp::GrayscaleRamp => &labels.grayscale_ramp,
            ColorPanelRamp::CurrentColors => &labels.current_colors,
        }
        .clone()
    }
}

/// The Color panel's settings, kept across runs (settings section `color_panel`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ColorPanelState {
    /// The panel edits the background colour (its well was clicked), not the foreground.
    pub background: bool,
    /// The picker layout.
    pub mode: ColorPanelMode,
    /// The colour ramp under the picker.
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
    /// The foreground colour.
    Foreground,
    /// The background colour.
    Background,
}

/// What [`ColorPanel::show`] returns.
#[derive(Debug, Default)]
#[non_exhaustive]
#[must_use = "the host applies the edit"]
pub struct ColorPanelResponse {
    /// The edits of the foreground or background colour this pass (from the view, the sliders,
    /// the ramp or a warning chip), in order, each with the colour it is for. Usually one; a
    /// field that commits as a slider begins makes two.
    pub edits: Vec<(PanelTarget, EditEvent<PickedColor>)>,
    /// A well was clicked while it was already the edited one, for this pass only: the app opens
    /// the [`ColorPickerDialog`](super::ColorPickerDialog) from the request (Photoshop). Its key
    /// is the well's [`PanelTarget::key`]; the colour is the well's.
    pub pick_color: Option<ColorRequest>,
    /// The Add to Swatches button under the Color Wheel was pressed: the app adds the colour of
    /// this well to the swatches.
    pub add_to_swatches: Option<PanelTarget>,
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
///   picker's (the cubes show the out-of-gamut chip only); a click takes the nearest colour.
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
    cmyk: Option<&'a CmykConverter>,
    enabled: bool,
    dynamic: bool,
}

impl<'a> ColorPanel<'a> {
    /// The panel on the app's `[foreground, background]`.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut ColorPanelState, colors: [Srgba; 2], numbers: &'a LocaleNumbers) -> ColorPanel<'a> {
        ColorPanel { id_salt: IdSalt::new(id_salt), state, colors, numbers, cmyk: None, enabled: true, dynamic: true }
    }

    /// The document's CMYK converter, for the CMYK sliders, the CMYK Spectrum ramp and the gamut
    /// warning; without it they use the built-in coated CMYK.
    pub fn cmyk(mut self, converter: &'a CmykConverter) -> ColorPanel<'a> {
        self.cmyk = Some(converter);
        self
    }

    /// Turns input on or off; on by default.
    pub fn enabled(mut self, enabled: bool) -> ColorPanel<'a> {
        self.enabled = enabled;
        self
    }

    /// Dynamic Color Sliders (PhotoCraft's interface preference, on by default): the slider tracks
    /// show the channel's colours with the other channels as they are. Off, the tracks keep the
    /// colours they had when a drag began.
    pub fn dynamic_tracks(mut self, dynamic: bool) -> ColorPanel<'a> {
        self.dynamic = dynamic;
        self
    }

    /// Shows the panel and returns this frame's edits and requests.
    pub fn show(self, ui: &mut Ui) -> ColorPanelResponse {
        let Self { id_salt, state, colors, numbers, cmyk, enabled, dynamic } = self;
        let id = ui.make_persistent_id(id_salt);
        let words = Words::of(ui);
        let builder = UiBuilder::new()
            .scope_id(id)
            .layout(Layout::top_down_justified(Align::Min))
            .accessibility_role(Role::Group)
            .accessibility_label(words.get(Word::plain("Color")).into_owned());
        let inner = ui.scope_builder(builder, |ui| {
            let target = if state.background { PanelTarget::Background } else { PanelTarget::Foreground };
            let kept_key = id.with(("kept", target.index()));
            let mut body = Body {
                id,
                enabled: enabled && ui.is_enabled(),
                target,
                colors,
                numbers,
                converter: cmyk.unwrap_or_else(|| CmykConverter::coated()),
                dynamic,
                labels: ColorPanelLabels::shared(ui),
                picker: ColorPickerLabels::shared(ui),
                words,
                kept: ui.data(|d| d.get_temp::<Vec<ModelValues>>(kept_key)).unwrap_or_default(),
                events: Vec::new(),
                pick_color: None,
                add_to_swatches: None,
            };
            body.show(ui, state);
            ui.data_mut(|d| d.insert_temp(kept_key, std::mem::take(&mut body.kept)));
            (body.events, body.pick_color, body.add_to_swatches)
        });
        let (edits, pick_color, add_to_swatches) = inner.inner;
        ColorPanelResponse { edits, pick_color, add_to_swatches }
    }

    /// The panel menu's entries, for the docking host's
    /// [`panel_menu_entries`](craft_ui::docking::DockContent::panel_menu_entries) hook: the nine
    /// modes with a radio mark on the current one, a separator, Copy Color as HTML and Copy
    /// Color's Hex Code, a separator, the four ramps with a radio mark, a separator, Make Ramp
    /// Web Safe (checked when on). The host runs the chosen row's command with
    /// [`run_command`](ColorPanel::run_command). `id_salt` is the panel's, as the other panels'
    /// menus take it; the Color panel's rows depend on `state` alone.
    pub fn panel_menu(ctx: &egui::Context, _id_salt: impl AsIdSalt, state: &ColorPanelState) -> Vec<MenuEntry<PanelCommand>> {
        let labels = ColorPanelLabels::shared_ctx(ctx);
        let mut entries: Vec<MenuEntry<PanelCommand>> =
            ColorPanelMode::ALL.iter().map(|m| MenuItem::new(m.label(&labels), mode_command(*m)).radio(state.mode == *m).into()).collect();
        entries.push(MenuEntry::Separator);
        entries.push(MenuItem::new(labels.copy_html.clone(), COPY_HTML.to_owned()).into());
        entries.push(MenuItem::new(labels.copy_hex.clone(), COPY_HEX.to_owned()).into());
        entries.push(MenuEntry::Separator);
        entries.extend(ramp_entries(&labels, state));
        entries
    }

    /// Runs the command of a chosen [`panel_menu`](ColorPanel::panel_menu) row (also the ramp's
    /// context menu's): sets the mode or ramp, toggles Make Ramp Web Safe, or copies `edited` to
    /// the clipboard as HTML (`color="#rrggbb"`) or as a hex code (`rrggbb`). Whether the command
    /// was one of the panel's.
    pub fn run_command(ctx: &egui::Context, _id_salt: impl AsIdSalt, state: &mut ColorPanelState, command: &PanelCommand, edited: Srgba) -> bool {
        let rgb = [edited.r, edited.g, edited.b];
        if let Some(mode) = ColorPanelMode::ALL.into_iter().find(|m| mode_command(*m) == *command) {
            state.mode = mode;
        } else if let Some(ramp) = ColorPanelRamp::ALL.into_iter().find(|r| ramp_command(*r) == *command) {
            state.ramp = ramp;
        } else if command == WEB_SAFE {
            state.web_safe_ramp = !state.web_safe_ramp;
        } else if command == COPY_HTML {
            ctx.copy_text(html_color(rgb));
        } else if command == COPY_HEX {
            ctx.copy_text(Srgba::from_rgb_array(rgb).to_hex().trim_start_matches('#').to_owned());
        } else {
            return false;
        }
        true
    }
}

const COPY_HTML: &str = "colorPanel.copyHtml";
const COPY_HEX: &str = "colorPanel.copyHex";
const WEB_SAFE: &str = "colorPanel.webSafeRamp";

fn mode_command(mode: ColorPanelMode) -> PanelCommand {
    format!("colorPanel.mode.{mode:?}")
}

fn ramp_command(ramp: ColorPanelRamp) -> PanelCommand {
    format!("colorPanel.ramp.{ramp:?}")
}

/// `color="#rrggbb"`, what Copy Color as HTML puts on the clipboard.
fn html_color(rgb: [f32; 3]) -> String {
    format!("color=\"{}\"", Srgba::from_rgb_array(rgb).to_hex())
}

impl PanelTarget {
    /// The key of the [`ColorRequest`] for this colour's picker.
    pub fn key(self) -> &'static str {
        match self {
            PanelTarget::Foreground => "colorPanel.foreground",
            PanelTarget::Background => "colorPanel.background",
        }
    }

    /// The target whose [`key`](PanelTarget::key) is `key`.
    pub fn from_key(key: &str) -> Option<PanelTarget> {
        [PanelTarget::Foreground, PanelTarget::Background].into_iter().find(|t| t.key() == key)
    }

    fn index(self) -> usize {
        match self {
            PanelTarget::Foreground => 0,
            PanelTarget::Background => 1,
        }
    }

    fn other(self) -> PanelTarget {
        match self {
            PanelTarget::Foreground => PanelTarget::Background,
            PanelTarget::Background => PanelTarget::Foreground,
        }
    }
}

/// The ramps and Make Ramp Web Safe: the panel menu's last sections and the ramp's context menu.
fn ramp_entries(labels: &ColorPanelLabels, state: &ColorPanelState) -> Vec<MenuEntry<PanelCommand>> {
    let mut entries: Vec<MenuEntry<PanelCommand>> =
        ColorPanelRamp::ALL.iter().map(|r| MenuItem::new(r.label(labels), ramp_command(*r)).radio(state.ramp == *r).into()).collect();
    entries.push(MenuEntry::Separator);
    entries.push(MenuItem::new(labels.web_safe_ramp.clone(), WEB_SAFE.to_owned()).check(state.web_safe_ramp).into());
    entries
}

/// What the panel draws and collects in one pass.
struct Body<'a> {
    id: Id,
    enabled: bool,
    /// The colour the panel edits.
    target: PanelTarget,
    colors: [Srgba; 2],
    numbers: &'a LocaleNumbers,
    converter: &'a CmykConverter,
    dynamic: bool,
    labels: Arc<ColorPanelLabels>,
    picker: Arc<ColorPickerLabels>,
    words: Arc<Words>,
    /// The values last edited, one per model, of the edited colour: shown while they still give
    /// its 8-bit sRGB, so a grey keeps its hue and CMYK or Lab values keep what was typed.
    kept: Vec<ModelValues>,
    events: Vec<(PanelTarget, EditEvent<PickedColor>)>,
    pick_color: Option<ColorRequest>,
    add_to_swatches: Option<PanelTarget>,
}

/// `values` with every non-finite channel set to 0, as PhotoCraft's model wrapper did: a NaN or
/// infinity from automation or a field must not reach the colour or the remembered values.
pub(super) fn finite_values(mut values: ModelValues) -> ModelValues {
    for v in &mut values.values {
        if !v.is_finite() {
            *v = 0.0;
        }
    }
    values
}

impl Body<'_> {
    fn color(&self) -> Srgba {
        self.colors[self.target.index()]
    }

    fn word(&self, text: &'static str) -> String {
        self.words.get(Word::plain(text)).into_owned()
    }

    fn push(&mut self, target: PanelTarget, event: EditEvent<PickedColor>) {
        self.events.push((target, event));
    }

    fn show(&mut self, ui: &mut Ui, state: &mut ColorPanelState) {
        let tokens = Tokens::of(ui);
        ui.spacing_mut().item_spacing.y = Gap::Stack.points(&tokens);
        let gap = Gap::Inline.points(&tokens);
        let mode = state.mode;
        let left = SwatchPair::style(&tokens);
        let left = left.well + left.step;
        let top = if direction::is_rtl(ui) { Layout::right_to_left(Align::Min) } else { Layout::left_to_right(Align::Min) };
        ui.allocate_ui_with_layout(vec2(ui.available_width(), 0.0), top, |ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.allocate_ui_with_layout(vec2(left, 0.0), direction::column(ui), |ui| {
                ui.spacing_mut().item_spacing.y = Gap::Stack.points(&tokens);
                self.wells(ui, state);
                // The cubes show the out-of-gamut chip only: the web-colour chip is for Web Color.
                if mode.is_sliders() || mode.is_cube() {
                    self.warnings(ui, mode == ColorPanelMode::WebColorSliders);
                }
            });
            ui.allocate_ui_with_layout(vec2(ui.available_width(), 0.0), Layout::top_down_justified(Align::Min), |ui| match mode {
                ColorPanelMode::HueCube => self.cube(ui, AreaMode::Hue),
                ColorPanelMode::BrightnessCube => self.cube(ui, AreaMode::Brightness),
                ColorPanelMode::ColorWheel => self.rows(ui, &HsbModel, false),
                ColorPanelMode::GrayscaleSlider => self.rows(ui, &GrayModel, false),
                ColorPanelMode::RgbSliders => self.rows(ui, &RgbModel, false),
                ColorPanelMode::HsbSliders => self.rows(ui, &HsbModel, false),
                ColorPanelMode::CmykSliders => self.rows(ui, &CmykModel::new(self.converter.clone()), false),
                ColorPanelMode::LabSliders => self.rows(ui, &LabModel, false),
                ColorPanelMode::WebColorSliders => self.rows(ui, &RgbModel, true),
            });
        });
        match mode {
            ColorPanelMode::HueCube | ColorPanelMode::BrightnessCube => self.readout(ui),
            ColorPanelMode::ColorWheel => self.wheel(ui),
            _ => self.ramp(ui, state),
        }
    }

    /// The foreground over the background: a click on the other well edits it, a click on the
    /// edited one asks for the picker.
    fn wells(&mut self, ui: &mut Ui, state: &mut ColorPanelState) {
        let active = if state.background { Well::Secondary } else { Well::Primary };
        let wells = [PairWell::new(self.labels.foreground.clone(), self.colors[0]), PairWell::new(self.labels.background.clone(), self.colors[1])];
        let pair =
            SwatchPair::new("wells", self.labels.wells.clone(), wells, self.numbers).active(active).popover(false).enabled(self.enabled).show(ui);
        state.background = pair.active == Well::Secondary;
        if pair.clicked == Some(active) {
            let title = match self.target {
                PanelTarget::Foreground => self.labels.picker_foreground.clone(),
                PanelTarget::Background => self.labels.picker_background.clone(),
            };
            self.pick_color = Some(ColorRequest::new(self.target.key(), self.color()).title(title));
        }
    }

    /// The out-of-gamut chip and, for Web Color, the web-colour chip under the wells.
    fn warnings(&mut self, ui: &mut Ui, web: bool) {
        let color = self.color();
        let rgb = [color.r, color.g, color.b];
        let nearest = |near: [f32; 3]| Srgba::rgb(near[0], near[1], near[2]);
        let mut warnings = Vec::new();
        if let Some(near) = self.converter.nearest_in_gamut(rgb, CmykConverter::PICKER_GAMUT_DE) {
            warnings.push(ColorWarning::new(self.picker.out_of_gamut.clone(), nearest(near)));
        }
        if web && let Some(near) = not_web_safe(rgb) {
            warnings.push(ColorWarning::web_colors(self.picker.not_web_color.clone(), nearest(near)));
        }
        direction::horizontal(ui, |ui| {
            for (i, warning) in warnings.into_iter().enumerate() {
                if ColorWarningChip::new(("warning", i), warning.clone()).enabled(self.enabled).show(ui).clicked() {
                    let picked = PickedColor { color: warning.nearest().with_alpha(color.a), model: None };
                    self.push(self.target, EditEvent::set(picked, Modifiers::NONE));
                }
            }
        });
    }

    /// The model's values for the edited colour, whole numbers as Photoshop's fields.
    fn values(&self, model: &dyn ColorModel) -> ModelValues {
        let color = self.color();
        let kept = self.kept.iter().find(|v| v.model == model.id() && model.to_srgb(&v.values).same_rgb8(color));
        let mut values = kept.copied().unwrap_or_else(|| self.read(model, color));
        for (slot, ch) in values.values.iter_mut().zip(model.channels()) {
            let scale = 10f32.powi(i32::from(ch.decimals));
            *slot = if slot.is_finite() { (*slot * scale).round() / scale } else { 0.0 };
        }
        values
    }

    /// `color` in `model`. HSB takes the hue of a grey, and the hue and saturation of black, from
    /// the last HSB values (PhotoCraft's remembered hue), which no longer give the colour.
    fn read(&self, model: &dyn ColorModel, color: Srgba) -> ModelValues {
        let mut values = model.from_srgb(color);
        if model.id() == HsbModel.id()
            && let Some(old) = self.kept.iter().find(|v| v.model == model.id())
        {
            if values.values[2] < 1.0 {
                values.values[0] = old.values[0];
                values.values[1] = old.values[1];
            } else if values.values[1] < 1.0 {
                values.values[0] = old.values[0];
            }
        }
        values
    }

    /// Keeps `values`, edited in `model`, and the hue and saturation of the colour they give.
    fn remember(&mut self, model: &dyn ColorModel, values: ModelValues) {
        self.kept.retain(|v| v.model != values.model);
        self.kept.push(values);
        if model.id() != HsbModel.id() {
            let hsb = HsbModel.from_srgb(model.to_srgb(&values.values));
            if hsb.values[1] >= 1.0 && hsb.values[2] >= 1.0 {
                self.kept.retain(|v| v.model != hsb.model);
                self.kept.push(hsb);
            }
        }
    }

    /// An edit of `model`'s values for the edited colour.
    fn model_event(&mut self, model: &dyn ColorModel, event: EditEvent<ModelValues>) {
        let event = event.map(finite_values);
        if let Some(values) = event.value() {
            self.remember(model, *values);
        }
        let alpha = self.color().a;
        let event = event.map(|values| PickedColor { color: model.to_srgb(&values.values).with_alpha(alpha), model: Some(values) });
        self.push(self.target, event);
    }

    /// An edit of channel `channel` of `current`.
    fn channel_event(&mut self, model: &dyn ColorModel, current: ModelValues, channel: usize, event: EditEvent<f64>) {
        self.model_event(
            model,
            event.map(|x| {
                let mut values = current;
                values.values[channel] = x as f32;
                values
            }),
        );
    }

    /// A channel's unit as a number field shows it.
    fn unit(&self, unit: &'static str) -> Unit {
        match unit {
            "" => Unit::default(),
            "%" => Unit::new(&self.word(picker_texts::PERCENT)),
            other => Unit::new(&format!("{{n}}{other}")),
        }
    }

    /// The width of every channel field: the widest value of any model's channels.
    fn field_width(&self, ui: &Ui) -> f32 {
        let models: [Box<dyn ColorModel>; 5] =
            [Box::new(HsbModel), Box::new(RgbModel), Box::new(LabModel), Box::new(GrayModel), Box::new(CmykModel::new(self.converter.clone()))];
        models
            .iter()
            .flat_map(|m| m.channels().iter())
            .map(|ch| {
                let range = f64::from(ch.min)..=f64::from(ch.max);
                NumberField::fit_width(ui, ControlSize::Md, self.numbers, range, usize::from(ch.decimals), &self.unit(ch.unit))
            })
            .fold(0.0, f32::max)
    }

    /// The channel rows of `model`: a word label that scrubs the number field at the row's end,
    /// and the slider under it over the channel's colours. Web Color has hex-pair fields and
    /// sliders that step by the web levels.
    fn rows(&mut self, ui: &mut Ui, model: &dyn ColorModel, web: bool) {
        let current = self.values(model);
        let width = self.field_width(ui);
        let (enabled, numbers) = (self.enabled, self.numbers);
        // Dynamic Color Sliders off: the tracks keep the colours they had when a drag began.
        let held_key = self.id.with(("tracks", model.id(), self.target.index()));
        let dragging = ui.input(|i| i.pointer.any_down());
        let basis = if self.dynamic || !dragging { current } else { ui.data(|d| d.get_temp::<ModelValues>(held_key)).unwrap_or(current) };
        if !dragging {
            ui.data_mut(|d| d.insert_temp(held_key, current));
        }
        for (i, ch) in model.channels().iter().enumerate() {
            let name = self.word(ch.name);
            let value = f64::from(current.values[i]);
            let mut rule = RowRule::of(ch);
            if web {
                rule.step = 51.0;
            }
            let mut track_values = basis;
            // The hue track is the whole spectrum, whatever the saturation and brightness.
            if model.id() == HsbModel.id() && i == 0 {
                track_values.values[1] = 100.0;
                track_values.values[2] = 100.0;
            }
            let track = model.track(i, &track_values);
            let unit = self.unit(ch.unit);
            let inner = Inner { policy: EditPolicy::default(), enabled, width, part: false };
            let (field, slider) = row_line(ui, (model.id(), i), &name, value, &rule, unit, numbers, Some(track), web, inner);
            for event in [field.event, slider.event].into_iter().flatten() {
                self.channel_event(model, current, i, event);
            }
        }
    }
    /// Hue Cube or Brightness Cube: the area and the strip of `mode`.
    fn cube(&mut self, ui: &mut Ui, mode: AreaMode) {
        let model = mode.model();
        let values = self.values(model);
        let tokens = Tokens::of(ui);
        let style = ColorArea::style(&tokens);
        let gap = Gap::Inline.points(&tokens);
        let width = (ui.available_width() - gap - ColorStrip::outer_width(&style, style.strip)).max(0.0);
        let [across, down] = mode.area();
        let channel = |i: usize| self.word(model.channels()[i].name);
        let (area_name, across_name, down_name, strip_name) = (self.word(mode.name()), channel(across), channel(down), channel(mode.strip()));
        let enabled = self.enabled;
        let (area, strip) = Row::new(Gap::Inline)
            .show(ui, |ui| {
                let area = ColorArea::new("area", area_name, mode, values)
                    .channel_names(across_name, down_name)
                    .size(vec2(width, style.area_height))
                    .enabled(enabled)
                    .show(ui);
                let strip =
                    ColorStrip::new("strip", strip_name, mode, values).size(vec2(style.strip, area.response.rect.height())).enabled(enabled).show(ui);
                (area, strip)
            })
            .inner;
        for event in [area.event, strip.event].into_iter().flatten() {
            self.model_event(model, event);
        }
    }

    /// The readout under the cubes: the hex field, then the R, G and B number fields (0 to 255,
    /// as the RGB sliders') with their short labels, in one row. Any of them sets the colour.
    // pc: crates/ui-egui/src/panels/color.rs:228 `color_readout`.
    fn readout(&mut self, ui: &mut Ui) {
        let color = self.color();
        let tokens = Tokens::of(ui);
        let gap = Gap::Inline.points(&tokens);
        let model = RgbModel;
        let current = self.values(&model);
        let rules = model.channels().iter().map(RowRule::of).collect::<Vec<_>>();
        let font = tokens.font(TypeRole::Mono);
        let label_width = |label: &str| {
            let job = LayoutJob::simple_singleline(label.to_owned(), font.clone(), Color32::PLACEHOLDER);
            ui.fonts_mut(|f| f.layout_job(job)).size().x
        };
        let labels: Vec<String> = model.channels().iter().map(|ch| self.word(ch.label)).collect();
        let field_width = NumberField::fit_width(ui, ControlSize::Md, self.numbers, 0.0..=255.0, 0, &Unit::default());
        let fields: f32 = labels.iter().map(|l| label_width(l) + field_width + 2.0 * gap).sum();
        let hex_width = (ui.available_width() - fields).max(0.0);
        let (enabled, numbers) = (self.enabled, self.numbers);
        let hex_name = self.word(picker_texts::HEX_COLOUR);
        let (hex, edits) = Row::new(Gap::Inline)
            .show(ui, |ui| {
                let hex = HexField::new("hex", hex_name, color.to_rgba()).width(hex_width).enabled(enabled).show(ui);
                let mut edits = Vec::new();
                for (i, ch) in model.channels().iter().enumerate() {
                    ui.add(Text::new(labels[i].as_str()).role(TypeRole::Mono).content(Content::Muted));
                    let name = self.word(ch.name);
                    let inner = Inner { policy: EditPolicy::default(), enabled, width: field_width, part: false };
                    let field = row_number((model.id(), i), &name, f64::from(current.values[i]), &rules[i], Unit::default(), numbers, inner);
                    edits.push(ui.allocate_ui_with_layout(vec2(field_width, 0.0), Layout::top_down_justified(Align::Min), |ui| field.show(ui)).inner);
                }
                (hex, edits)
            })
            .inner;
        if let Some(event) = hex.event {
            self.push(self.target, event.map(|rgba| PickedColor { color: Srgba::from(rgba).with_alpha(color.a), model: None }));
        }
        for (i, edit) in edits.into_iter().enumerate() {
            if let Some(event) = edit.event {
                self.channel_event(&model, current, i, event);
            }
        }
    }

    /// The Color Wheel under the HSB rows.
    fn wheel(&mut self, ui: &mut Ui) {
        let values = self.values(&HsbModel);
        let words = self.words.clone();
        let translate =
            move |text: &str| ColorWheel::TEXTS.iter().find(|t| **t == text).map_or(text.to_owned(), |t| words.get(Word::plain(t)).into_owned());
        let wheel = ColorWheel::new("wheel", self.labels.color_wheel.clone(), values).text(translate).enabled(self.enabled).show(ui);
        if let Some(event) = wheel.edit.event {
            self.model_event(&HsbModel, event);
        }
        let tokens = Tokens::of(ui);
        let side = IconButton::style(&tokens, ControlSize::Md).side;
        let area = wheel.area;
        let corner = if direction::is_rtl(ui) { area.left_bottom() + vec2(0.0, -side) } else { area.right_bottom() - vec2(side, side) };
        let add = ui
            .scope_builder(UiBuilder::new().max_rect(Rect::from_min_size(corner, vec2(side, side))), |ui| {
                ui.add(IconButton::new(Icon::Plus, self.labels.add_to_swatches.as_str()).enabled(self.enabled))
            })
            .inner;
        if add.on_hover_text(self.labels.create_swatch.as_str()).clicked() {
            self.add_to_swatches = Some(self.target);
        }
    }

    /// The ramp with the white and black boxes; Alt picks for the colour not edited, Shift-click
    /// cycles the kind, a right-click opens the ramp menu.
    fn ramp(&mut self, ui: &mut Ui, state: &mut ColorPanelState) {
        let converter = self.converter;
        let gamut = |c: Srgba| {
            let back = converter.to_srgb(converter.from_srgb([c.r, c.g, c.b]));
            Srgba::new(back[0], back[1], back[2], c.a)
        };
        let mut hasher = DefaultHasher::new();
        Arc::as_ptr(converter.profile()).hash(&mut hasher);
        let [white, black] = ColorRamp::TEXTS.map(|t| self.word(t));
        let ramp = ColorRamp::new("ramp", self.labels.color_ramp.clone(), state.ramp.kind())
            .kind_name(state.ramp.label(&self.labels))
            .ends(self.colors[0], self.colors[1])
            .gamut(&gamut)
            .gamut_revision(hasher.finish())
            .web_safe(state.web_safe_ramp)
            .texts(white, black)
            .enabled(self.enabled)
            .show(ui);
        // Alt is on every event of a gesture that has a value; the others go where it went.
        let other_key = self.id.with("ramp-other");
        if let Some(event) = ramp.edit.event {
            if let Some(pick) = event.value() {
                ui.data_mut(|d| d.insert_temp(other_key, pick.other));
            }
            let other = ui.data(|d| d.get_temp::<bool>(other_key)).unwrap_or(false);
            let target = if other { self.target.other() } else { self.target };
            let alpha = self.colors[target.index()].a;
            self.push(target, event.map(|pick| PickedColor { color: pick.color.with_alpha(alpha), model: None }));
        }
        if ramp.cycle {
            state.ramp = state.ramp.next();
        }
        let entries = ramp_entries(&self.labels, state);
        let menu = ContextMenu::new(&entries).own_gestures(false).open_at(ramp.menu_at.map(OpenAt::Pointer)).show(ui, &ramp.bar);
        if let Some(command) = menu.chosen {
            ColorPanel::run_command(ui.ctx(), self.id, state, &command, self.color());
        }
    }
}
