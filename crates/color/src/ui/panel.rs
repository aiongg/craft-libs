//! [`ColorPanel`] (F04.10): Photoshop's Color panel. The foreground and background wells, one of
//! nine views of the edited colour (two cubes, a wheel, five slider sets), the out-of-gamut and
//! web-colour warnings, and the colour ramp under the sliders; the panel menu chooses the view and
//! the ramp and copies the colour.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use craft_cms::convert::CmykConverter;
use craft_theme::{ColorModel, ControlSize, HsbModel, LabModel, ModelValues, RgbModel, Srgba};
use egui::accesskit::Role;
use egui::{Align, AsIdSalt, Id, IdSalt, Layout, Modifiers, Rect, Ui, UiBuilder, pos2, vec2};

use super::picker::texts as picker_texts;
use super::{
    AreaMode, ColorArea, ColorPickerLabels, ColorRamp, ColorStrip, ColorWarning, ColorWarningChip, ColorWheel, HexField, HexPairField, PairWell,
    PickedColor, RampKind, SwatchPair, Well,
};
use crate::{CmykModel, GrayModel, not_web_safe};
use craft_ui::composites::PropertyRow;
use craft_ui::controls::{NumberField, Slider};
use craft_ui::direction;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{LocaleNumbers, SharedLabels, Unit, Word, Words};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use craft_ui::primitives::{Gap, Row};
use craft_ui::theme::{Tokens, TokensOf};

/// The ΔE (CIE76) past which a colour is out of the CMYK gamut: PhotoCraft's `GAMUT_DE`.
const GAMUT_DE: f32 = 3.0;

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
        wells: "colorPanel.wells" = "Colours";
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
    dynamic: bool,
}

impl<'a> ColorPanel<'a> {
    /// The panel on the app's `[foreground, background]`.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut ColorPanelState, colors: [Srgba; 2], numbers: &'a LocaleNumbers) -> ColorPanel<'a> {
        ColorPanel { id_salt: IdSalt::new(id_salt), state, colors, numbers, gamut: None, enabled: true, dynamic: true }
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

    /// Dynamic Color Sliders (PhotoCraft's interface preference, on by default): the slider tracks
    /// show the channel's colours with the other channels as they are. Off, the tracks keep the
    /// colours they had when a drag began.
    pub fn dynamic_tracks(mut self, dynamic: bool) -> ColorPanel<'a> {
        self.dynamic = dynamic;
        self
    }

    /// Shows the panel and returns this frame's edit and picker request. The panel keeps the edit
    /// events of one pass in order: a pass that makes more than one reports the first and the
    /// rest in the passes after it.
    pub fn show(self, ui: &mut Ui) -> ColorPanelResponse {
        let Self { id_salt, state, colors, numbers, gamut, enabled, dynamic } = self;
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
                converter: gamut.unwrap_or_else(|| CmykConverter::coated()),
                dynamic,
                labels: ColorPanelLabels::shared(ui),
                picker: ColorPickerLabels::shared(ui),
                words,
                kept: ui.data(|d| d.get_temp::<Vec<ModelValues>>(kept_key)).unwrap_or_default(),
                events: Vec::new(),
                open_picker: None,
            };
            body.show(ui, state);
            ui.data_mut(|d| d.insert_temp(kept_key, std::mem::take(&mut body.kept)));
            (body.events, body.open_picker)
        });
        let (events, open_picker) = inner.inner;
        let queue_key = id.with("queue");
        let mut queue: Vec<(PanelTarget, EditEvent<PickedColor>)> = ui.data_mut(|d| d.remove_temp(queue_key)).unwrap_or_default();
        queue.extend(events);
        let edit = (!queue.is_empty()).then(|| queue.remove(0));
        if !queue.is_empty() {
            ui.data_mut(|d| d.insert_temp(queue_key, queue));
            ui.ctx().request_repaint();
        }
        ColorPanelResponse { edit, open_picker }
    }

    /// The panel menu's entries, for the docking host's `panel_menu` hook
    /// ([`DockContent::panel_menu`](craft_ui::docking::DockContent::panel_menu)): the nine modes
    /// with a check mark on the current one, a separator, Copy Color as HTML (`color="#rrggbb"`)
    /// and Copy Color's Hex Code (`rrggbb`) of the edited colour to the clipboard, a separator,
    /// the four ramps with a check mark, a separator, Make Ramp Web Safe (checked when on).
    pub fn panel_menu(ui: &mut Ui, state: &mut ColorPanelState, edited: Srgba) {
        let labels = ColorPanelLabels::shared(ui);
        for mode in ColorPanelMode::ALL {
            if check_item(ui, Role::MenuItemRadio, state.mode == mode, &mode.label(&labels)) {
                state.mode = mode;
                ui.close();
            }
        }
        ui.separator();
        let rgb = [edited.r, edited.g, edited.b];
        if ui.button(&labels.copy_html).clicked() {
            ui.ctx().copy_text(html_color(rgb));
            ui.close();
        }
        if ui.button(&labels.copy_hex).clicked() {
            ui.ctx().copy_text(hex_code(rgb));
            ui.close();
        }
        ui.separator();
        for ramp in ColorPanelRamp::ALL {
            if check_item(ui, Role::MenuItemRadio, state.ramp == ramp, &ramp.label(&labels)) {
                state.ramp = ramp;
                ui.close();
            }
        }
        ui.separator();
        if check_item(ui, Role::MenuItemCheckBox, state.web_safe_ramp, &labels.web_safe_ramp) {
            state.web_safe_ramp = !state.web_safe_ramp;
            ui.close();
        }
    }
}

/// `color="#rrggbb"`, what Copy Color as HTML puts on the clipboard.
fn html_color(rgb: [f32; 3]) -> String {
    format!("color=\"{}\"", crate::format_hex(rgb))
}

/// `rrggbb`, what Copy Color's Hex Code puts on the clipboard.
fn hex_code(rgb: [f32; 3]) -> String {
    crate::format_hex(rgb).trim_start_matches('#').to_owned()
}

/// A panel menu row with a check mark column, announced as checked or not.
fn check_item(ui: &mut Ui, role: Role, on: bool, label: &str) -> bool {
    let response = ui.button(format!("{}{label}", if on { "\u{2714} " } else { "   " }));
    response.widget_info(|| egui::WidgetInfo::selected(role, ui.is_enabled(), on, label));
    response.clicked()
}

impl PanelTarget {
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

/// A command of the ramp's context menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RampCommand {
    Ramp(ColorPanelRamp),
    WebSafe,
}

/// The ramp's context menu: the ramps and Make Ramp Web Safe, as the panel menu's last sections.
fn ramp_entries(labels: &ColorPanelLabels, state: &ColorPanelState) -> Vec<MenuEntry<RampCommand>> {
    let mut entries: Vec<MenuEntry<RampCommand>> =
        ColorPanelRamp::ALL.iter().map(|r| MenuItem::new(r.label(labels), RampCommand::Ramp(*r)).radio(state.ramp == *r).into()).collect();
    entries.push(MenuEntry::Separator);
    entries.push(MenuItem::new(labels.web_safe_ramp.clone(), RampCommand::WebSafe).check(state.web_safe_ramp).into());
    entries
}

/// `add` in a column `width` wide at the reading end of the `Ui`.
fn at_end<R>(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let full = ui.available_rect_before_wrap();
    let width = width.min(full.width());
    let x = if direction::is_rtl(ui) { full.left() } else { full.right() - width };
    let rect = Rect::from_min_size(pos2(x, full.top()), vec2(width, full.height()));
    ui.scope_builder(UiBuilder::new().max_rect(rect).layout(Layout::top_down_justified(Align::Min)), add).inner
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
    open_picker: Option<PanelTarget>,
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
                if mode.is_sliders() {
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
            ColorPanelMode::HueCube | ColorPanelMode::BrightnessCube => self.hex(ui),
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
            self.open_picker = Some(self.target);
        }
    }

    /// The out-of-gamut chip and, for Web Color, the web-colour chip under the wells.
    fn warnings(&mut self, ui: &mut Ui, web: bool) {
        let color = self.color();
        let rgb = [color.r, color.g, color.b];
        let nearest = |near: [f32; 3]| Srgba::rgb(near[0], near[1], near[2]);
        let mut warnings = Vec::new();
        if let Some(near) = self.converter.nearest_in_gamut(rgb, GAMUT_DE) {
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
            let range = f64::from(ch.min)..=f64::from(ch.max);
            let field = if web {
                let byte = value.clamp(0.0, 255.0) as u8;
                PropertyRow::word(&name)
                    .show(ui, |ui, _| {
                        at_end(ui, width, |ui| {
                            HexPairField::new((model.id(), "hex", i), name.as_str(), byte).enabled(enabled).show(ui).map(f64::from)
                        })
                    })
                    .inner
            } else {
                let unit = self.unit(ch.unit);
                PropertyRow::word(&name)
                    .scrubs()
                    .show(ui, |ui, row| {
                        at_end(ui, width, |ui| {
                            let field = NumberField::new((model.id(), "field", i), name.as_str(), value, numbers)
                                .range(range.clone())
                                .step(f64::from(ch.step))
                                .decimals(0..=usize::from(ch.decimals))
                                .unit(unit)
                                .enabled(enabled);
                            match row.scrub() {
                                Some(label) => field.scrub_by(label),
                                None => field,
                            }
                            .show(ui)
                        })
                    })
                    .inner
            };
            if let Some(event) = field.event {
                self.channel_event(model, current, i, event);
            }
            let mut track_values = basis;
            // The hue track is the whole spectrum, whatever the saturation and brightness.
            if model.id() == HsbModel.id() && i == 0 {
                track_values.values[1] = 100.0;
                track_values.values[2] = 100.0;
            }
            let step = if web { 51.0 } else { f64::from(ch.step) };
            let slider = Slider::new((model.id(), "slider", i), name.as_str(), value, range)
                .step(step)
                .track_colors(model.track(i, &track_values))
                .enabled(enabled)
                .show(ui);
            if let Some(event) = slider.event {
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

    /// The hex field under the cubes.
    fn hex(&mut self, ui: &mut Ui) {
        let color = self.color();
        let edit = HexField::new("hex", self.word(picker_texts::HEX_COLOUR), color.to_rgba()).enabled(self.enabled).show(ui);
        if let Some(event) = edit.event {
            self.push(self.target, event.map(|rgba| PickedColor { color: Srgba::from(rgba).with_alpha(color.a), model: None }));
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
        match menu.chosen {
            Some(RampCommand::Ramp(kind)) => state.ramp = kind,
            Some(RampCommand::WebSafe) => state.web_safe_ramp = !state.web_safe_ramp,
            None => {}
        }
    }
}
