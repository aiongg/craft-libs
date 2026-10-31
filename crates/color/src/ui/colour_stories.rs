//! Stories of the colour dialogs, panels and tools, one per component (ids in [`IDS`]).
//!
//! Dialogs run on a [`DialogHost`] kept in egui memory and, in an Overview tile, start behind a
//! "Show dialog" button. The screen eyedropper is off in these stories: they never start a real
//! screen capture.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use craft_cms::convert::CmykConverter;
use craft_host::Host;
use craft_settings::Remembered;
use craft_theme::{ColorModel, Hsb, RgbModel, Srgba};
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasView, ToolHost};
use craft_ui::controls::Button;
use craft_ui::dialog::{DialogHost, DialogResult, StackDialog};
use craft_ui::docking::panel_menu::menu_under;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels, LocaleNumbers};
use craft_ui::menu::MenuEntry;
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Gap, Row, Stack, Text};
use craft_ui::story::{Level, NoArgs, Story, StoryArgs, StoryChoice, Surface, dialog_closed, dialog_shown};
use craft_ui::typed_story;
use egui::{Color32, Context, Id, Rect, Sense, TextureHandle, TextureOptions, Ui, pos2, vec2};

use super::picker_dialog::MEMORY_SCOPE;
use super::{
    ChosenColor, ColorChoice, ColorPanel, ColorPanelMode, ColorPanelRamp, ColorPanelState, ColorPickerDialog, ColorPickerMemory, ColorSamplerLabels,
    ColorSamplerTool, ColorSamplers, EyedropperOptions, EyedropperOptionsBar, GradientPreview, HarmonyPanel, HarmonyPanelState, HarmonyWheel,
    LutChoice, LutPicker, NamedColor, Paint, PaintChip, PanelTarget, ProfileMismatchDialog, ProfileMode, ProfileQuestion, SamplerReadout,
    SamplerReadouts, ScreenColorPicker, StillCapture, SwatchLibrary, SwatchesPanel, sampler_options,
};
use crate::harmony::{Harmony, move_on_wheel};
use crate::{AreaMode, CmykModel};

/// The story ids, one per component:
/// - `patterns/color/picker-dialog`: a "Show dialog" tile opening `ColorPickerDialog` on a
///   `DialogHost`, args for Only Web Colors, preview; the eyedropper answers from a fake capture; prints the result.
/// - `patterns/color/panel`: `ColorPanel` with a mode and a ramp arg, the panel menu beside it.
/// - `patterns/color/swatches`: `SwatchesPanel` on the default library, the panel menu beside it.
/// - `patterns/color/eyedropper-options`: `EyedropperOptionsBar` in an `OptionsBar`.
/// - `patterns/color/samplers`: four points on a gradient image in a `CanvasView`, readouts under it.
/// - `patterns/color/paint-chip`: `PaintChip` for each paint kind, with and without No Color.
/// - `patterns/color/choice`: `ColorChoice` with PhotoCraft's pasteboard rows.
/// - `patterns/color/harmony`: `HarmonyWheel` and `HarmonyPanel` on one base colour, a rule arg.
/// - `patterns/color/lut-picker`: `LutPicker` with no folder.
/// - `patterns/color/profile-mismatch`: a "Show dialog" tile for both questions.
#[cfg(test)]
pub(super) const IDS: [&str; 10] = [
    "patterns/color/picker-dialog",
    "patterns/color/panel",
    "patterns/color/swatches",
    "patterns/color/eyedropper-options",
    "patterns/color/samplers",
    "patterns/color/paint-chip",
    "patterns/color/choice",
    "patterns/color/harmony",
    "patterns/color/lut-picker",
    "patterns/color/profile-mismatch",
];

/// The stories of [`IDS`].
pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/color/picker-dialog",
            Level::Patterns,
            "ColorPickerDialog",
            "Dialog",
            PickerDialogArgs { open: true, web_colors: false, preview: true },
            show_picker_dialog
        ),
        typed_story!(
            "patterns/color/panel",
            Level::Patterns,
            "ColorPanel",
            "Panel",
            PanelArgs { mode: ColorPanelMode::HueCube, ramp: ColorPanelRamp::RgbSpectrum, web_safe_ramp: false, cmyk_gamut: true },
            show_panel
        ),
        typed_story!("patterns/color/swatches", Level::Patterns, "SwatchesPanel", "Panel", NoArgs {}, show_swatches),
        typed_story!("patterns/color/eyedropper-options", Level::Patterns, "EyedropperOptionsBar", "Options bar", NoArgs {}, show_eyedropper_options)
            .on(Surface::Chrome),
        typed_story!("patterns/color/samplers", Level::Patterns, "ColorSamplers", "Sample points", NoArgs {}, show_samplers).on(Surface::Canvas),
        typed_story!(
            "patterns/color/paint-chip",
            Level::Patterns,
            "PaintChip",
            "Paint kinds",
            PaintChipArgs { no_color: true, gradient: true, disabled: false },
            show_paint_chip
        ),
        typed_story!(
            "patterns/color/choice",
            Level::Patterns,
            "ColorChoice",
            "Pasteboard",
            ChoiceArgs { custom: true, disabled: false },
            show_choice
        ),
        typed_story!(
            "patterns/color/harmony",
            Level::Patterns,
            "HarmonyPanel",
            "Wheel and panel",
            HarmonyArgs { rule: Harmony::Triad, limit_to_swatches: false },
            show_harmony
        ),
        typed_story!("patterns/color/lut-picker", Level::Patterns, "LutPicker", "Select", LutArgs { disabled: false }, show_lut_picker),
        typed_story!(
            "patterns/color/profile-mismatch",
            Level::Patterns,
            "ProfileMismatchDialog",
            "Dialog",
            ProfileArgs { open: true, question: QuestionArg::Mismatch, mode: ModeArg::Rgb, rememberable: true },
            show_profile_dialog
        ),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PickerDialogArgs {
    open: bool,
    #[arg(name = "Only Web Colors")]
    web_colors: bool,
    #[arg(name = "live preview")]
    preview: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PanelArgs {
    mode: ColorPanelMode,
    ramp: ColorPanelRamp,
    #[arg(name = "web safe ramp")]
    web_safe_ramp: bool,
    #[arg(name = "CMYK gamut")]
    cmyk_gamut: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PaintChipArgs {
    #[arg(name = "No Color")]
    no_color: bool,
    gradient: bool,
    disabled: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ChoiceArgs {
    #[arg(name = "custom color")]
    custom: bool,
    disabled: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct HarmonyArgs {
    rule: Harmony,
    #[arg(name = "limit to swatches")]
    limit_to_swatches: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct LutArgs {
    disabled: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum QuestionArg {
    Mismatch,
    Missing,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    #[choice(name = "RGB")]
    Rgb,
    #[choice(name = "CMYK")]
    Cmyk,
    #[choice(name = "Gray")]
    Gray,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ProfileArgs {
    open: bool,
    question: QuestionArg,
    mode: ModeArg,
    #[arg(name = "remember check box")]
    rememberable: bool,
}

impl StoryChoice for ColorPanelMode {
    const OPTIONS: &'static [&'static str] = &[
        "Hue Cube",
        "Brightness Cube",
        "Color Wheel",
        "Grayscale Slider",
        "RGB Sliders",
        "HSB Sliders",
        "CMYK Sliders",
        "Lab Sliders",
        "Web Color Sliders",
    ];

    fn index(&self) -> usize {
        ColorPanelMode::ALL.iter().position(|m| m == self).unwrap_or(0)
    }

    fn from_index(index: usize) -> Self {
        ColorPanelMode::ALL[index.min(ColorPanelMode::ALL.len() - 1)]
    }
}

impl StoryChoice for ColorPanelRamp {
    const OPTIONS: &'static [&'static str] = &["RGB Spectrum", "CMYK Spectrum", "Grayscale Ramp", "Current Colors"];

    fn index(&self) -> usize {
        ColorPanelRamp::ALL.iter().position(|r| r == self).unwrap_or(0)
    }

    fn from_index(index: usize) -> Self {
        ColorPanelRamp::ALL[index.min(ColorPanelRamp::ALL.len() - 1)]
    }
}

impl StoryChoice for Harmony {
    const OPTIONS: &'static [&'static str] = &[
        "Complementary",
        "Complementary 2",
        "Split Complementary",
        "Left Complement",
        "Right Complement",
        "Analogous",
        "Analogous 2",
        "Monochromatic",
        "Shades",
        "Triad",
        "Triad 2",
        "Triad 3",
        "Tetrad",
        "Tetrad 2",
        "Tetrad 3",
        "Compound",
        "Compound 2",
        "High Contrast",
        "High Contrast 2",
        "High Contrast 3",
        "Pentagram",
    ];

    fn index(&self) -> usize {
        Harmony::ALL.iter().position(|h| h == self).unwrap_or(0)
    }

    fn from_index(index: usize) -> Self {
        Harmony::ALL[index.min(Harmony::ALL.len() - 1)]
    }
}

/// Per-instance state kept in egui memory under `name`.
fn shared<T: Send + 'static>(ui: &Ui, name: &str, init: impl FnOnce() -> T) -> Arc<Mutex<T>> {
    let key = ui.make_persistent_id(("craft_color::colour_stories", name));
    ui.data_mut(|d| d.get_temp_mut_or_insert_with(key, || Arc::new(Mutex::new(init()))).clone())
}

fn lock<T>(state: &Mutex<T>) -> MutexGuard<'_, T> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

fn hex(color: Srgba) -> String {
    color.to_hex().to_uppercase()
}

fn muted(ui: &mut Ui, text: impl Into<String>) {
    ui.add(Text::new(text).content(Content::Muted));
}

/// The width of a panel beside its menu button: 260 points, less in a narrow tile.
fn panel_width(ui: &Ui) -> f32 {
    (ui.available_width() - 40.0).clamp(160.0, 260.0)
}

/// A panel's `entries` under a "≡" button; the chosen command, if any.
fn panel_menu(ui: &mut Ui, entries: &[MenuEntry<String>]) -> Option<String> {
    let button = ui.add(Button::new("≡"));
    menu_under(ui, &button, entries).chosen
}

/// Whether this story instance may draw the dialog `name` in this pass: the dialogs have one
/// fixed id each, so two instances drawn in the same pass (a component tab and a variants view)
/// must not both show it.
fn claim(ctx: &Context, name: &str, instance: Id) -> bool {
    let key = Id::unique(("craft_color::colour_stories::claim", name));
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        let (claimed_pass, owner) = d.get_temp::<(u64, Id)>(key).unwrap_or((u64::MAX, instance));
        if claimed_pass == pass && owner != instance {
            return false;
        }
        d.insert_temp(key, (pass, instance));
        true
    })
}

/// Opens `D` while the story is shown and `open` is on, reopens it when `signature` changes (the
/// arguments that build the dialog) and prints the answer under the tile once it closes. `live`
/// reads the open dialog for a line printed under it.
fn drive<D: StackDialog, S: PartialEq + Clone + Send + Sync + 'static>(
    ui: &mut Ui,
    name: &str,
    open: &mut bool,
    signature: S,
    make: impl FnOnce(&Context) -> D,
    live: impl FnOnce(&D) -> Option<String>,
    describe: impl FnOnce(D::Result) -> String,
) {
    let host = shared(ui, name, DialogHost::new);
    let result_key = ui.make_persistent_id(("craft_color::colour_stories::result", name));
    let signature_key = result_key.with("signature");
    let ctx = ui.ctx().clone();
    let mut host = lock(&host);
    if dialog_shown(ui, *open) && claim(&ctx, name, result_key) {
        let previous: Option<S> = ui.data(|d| d.get_temp::<Option<S>>(signature_key)).flatten();
        if previous.is_some_and(|previous| previous != signature) {
            host.cancel::<D>();
            host.take::<D>();
        }
        ui.data_mut(|d| d.insert_temp(signature_key, Some(signature)));
        if !host.is_open::<D>() {
            host.open(make(&ctx));
            ui.data_mut(|d| d.remove_temp::<String>(result_key));
        }
        host.show(&ctx);
        if let Some(line) = host.get::<D>().and_then(live) {
            muted(ui, line);
        }
        if let Some(result) = host.take::<D>() {
            let text = match result {
                DialogResult::Ok(value) => describe(value),
                DialogResult::Cancel => "Cancelled".to_owned(),
            };
            ui.data_mut(|d| d.insert_temp(result_key, text));
            dialog_closed(ui, open);
        }
    } else {
        host.cancel::<D>();
        host.take::<D>();
        ui.data_mut(|d| d.insert_temp(signature_key, None::<S>));
    }
    if let Some(text) = ui.data(|d| d.get_temp::<String>(result_key)) {
        ui.add(Text::new(format!("Result: {text}")));
    }
}

fn show_picker_dialog(ui: &mut Ui, args: &mut PickerDialogArgs) {
    let (web_colors, preview) = (args.web_colors, args.preview);
    // A fake capture in place of the screen: the eyedropper answers at once with a fixed colour.
    ScreenColorPicker::tick_with(ui.ctx(), 0, &|| Box::new(StillCapture::color(Srgba::rgb(0.9, 0.55, 0.1))));
    drive(
        ui,
        "picker-dialog",
        &mut args.open,
        (web_colors, preview),
        |ctx| {
            if let Some(host) = Host::try_of_ctx(ctx) {
                Remembered::<ColorPickerMemory>::new(MEMORY_SCOPE).store(&host, &ColorPickerMemory { mode: AreaMode::default(), web_colors });
            }
            ColorPickerDialog::foreground(Srgba::rgb(0.24, 0.39, 0.87)).canvas_samples(false)
        },
        |dialog| preview.then(|| format!("Preview: {}", hex(dialog.current().color))),
        |result| format!("{} {}", result.key, hex(result.picked.color)),
    );
}

/// The Color panel's story state: the panel settings, `[foreground, background]` and a note.
type PanelStory = (ColorPanelState, [Srgba; 2], String);

fn show_panel(ui: &mut Ui, args: &mut PanelArgs) {
    let numbers = LocaleNumbers::of(ui);
    let state = shared::<PanelStory>(ui, "panel", || {
        (ColorPanelState::default(), [Srgba::rgb(0.86, 0.24, 0.16), Srgba::rgb(0.96, 0.96, 0.9)], String::new())
    });
    let ctx = ui.ctx().clone();
    let (mut panel_state, mut colors, mut note) = lock(&state).clone();
    // The args seed the settings when they change; the panel menu and the ramp's context menu
    // change the settings and the args follow.
    let seed = ui.make_persistent_id("panel-seed");
    let seeded: Option<(ColorPanelMode, ColorPanelRamp, bool)> = ui.data(|d| d.get_temp(seed));
    if seeded != Some((args.mode, args.ramp, args.web_safe_ramp)) {
        panel_state.mode = args.mode;
        panel_state.ramp = args.ramp;
        panel_state.web_safe_ramp = args.web_safe_ramp;
    }
    Row::new(Gap::Inline).show(ui, |ui| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            ui.set_width(panel_width(ui));
            let mut panel = ColorPanel::new("panel", &mut panel_state, colors, &numbers);
            if args.cmyk_gamut {
                panel = panel.gamut(CmykConverter::coated());
            }
            let shown = panel.show(ui);
            for (target, event) in &shown.edits {
                if let Some(picked) = event.value() {
                    colors[usize::from(*target == PanelTarget::Background)] = picked.color;
                }
            }
            if shown.open_picker.is_some() {
                note = "The well opens the Color Picker dialog.".to_owned();
            }
            if shown.add_to_swatches.is_some() {
                note = "Add to Swatches adds the color of the edited well.".to_owned();
            }
        });
        let entries = ColorPanel::panel_menu(ui, &panel_state);
        if let Some(command) = panel_menu(ui, &entries) {
            let edited = colors[usize::from(panel_state.background)];
            ColorPanel::run_command(&ctx, &mut panel_state, &command, edited);
        }
    });
    muted(ui, format!("Foreground {}, background {}", hex(colors[0]), hex(colors[1])));
    if !note.is_empty() {
        muted(ui, note.clone());
    }
    args.mode = panel_state.mode;
    args.ramp = panel_state.ramp;
    args.web_safe_ramp = panel_state.web_safe_ramp;
    ui.data_mut(|d| d.insert_temp(seed, (args.mode, args.ramp, args.web_safe_ramp)));
    *lock(&state) = (panel_state, colors, note);
}

fn show_swatches(ui: &mut Ui, _: &mut NoArgs) {
    let state = shared(ui, "swatches", || (SwatchLibrary::default(), Srgba::rgb(0.24, 0.39, 0.87), String::new()));
    let ctx = ui.ctx().clone();
    let mut guard = lock(&state);
    let (library, foreground, note) = &mut *guard;
    Row::new(Gap::Inline).show(ui, |ui| {
        let size = vec2(panel_width(ui), 320.0);
        ui.allocate_ui(size, |ui| {
            ui.set_min_height(size.y);
            let shown = SwatchesPanel::new("swatches", library, CmykConverter::coated(), *foreground).show(ui);
            if let Some((_, color, _)) = shown.used {
                *foreground = color;
            }
        });
        let entries = SwatchesPanel::panel_menu(&ctx, library);
        if let Some(command) = panel_menu(ui, &entries) {
            let response = SwatchesPanel::run_command(&ctx, &command, library, *foreground);
            if let Some(action) = response.action {
                *note = format!("{action:?}: the app opens a file dialog.");
            }
        }
    });
    muted(ui, format!("Foreground {}", hex(*foreground)));
    if !note.is_empty() {
        muted(ui, note.clone());
    }
}

fn show_eyedropper_options(ui: &mut Ui, _: &mut NoArgs) {
    let options = shared(ui, "eyedropper-options", EyedropperOptions::default);
    let mut options = lock(&options);
    OptionsBar::new("eyedropper-bar").show(ui, |slots| EyedropperOptionsBar::new(&mut options).add_to(slots));
}

/// The sampler story's image, in pixels: hue across, brightness down.
const IMAGE: [usize; 2] = [240, 150];

fn image_size() -> [u32; 2] {
    IMAGE.map(|v| u32::try_from(v).unwrap_or(u32::MAX))
}

fn image_pixel(x: usize, y: usize) -> Srgba {
    Srgba::from_hsb(Hsb { h: x as f32 / IMAGE[0] as f32 * 360.0, s: 1.0, b: 1.0 - y as f32 / IMAGE[1] as f32 * 0.85 })
}

fn image_texture(ctx: &Context) -> TextureHandle {
    let key = Id::unique("craft_color::colour_stories::sampler-image");
    if let Some(texture) = ctx.data(|d| d.get_temp::<TextureHandle>(key)) {
        return texture;
    }
    let pixels = (0..IMAGE[1]).flat_map(|y| (0..IMAGE[0]).map(move |x| Color32::from(image_pixel(x, y)))).collect();
    let texture = ctx.load_texture("craft_color::colour_stories::sampler-image", egui::ColorImage::new(IMAGE, pixels), TextureOptions::NEAREST);
    ctx.data_mut(|d| d.insert_temp(key, texture.clone()));
    texture
}

fn show_samplers(ui: &mut Ui, _: &mut NoArgs) {
    let samplers = shared(ui, "samplers", || {
        let mut samplers = ColorSamplers::new();
        for at in [[40.0, 30.0], [120.0, 75.0], [200.0, 50.0], [80.0, 120.0]] {
            let _ = samplers.add(at, image_size());
        }
        samplers
    });
    let mut samplers = lock(&samplers);
    let labels = ColorSamplerLabels::of(ui);
    let mut clear = false;
    let count = samplers.points().len();
    OptionsBar::new("sampler-bar").show(ui, |slots| slots.body(|ui| clear = sampler_options(ui, count)));
    if clear {
        samplers.clear();
    }

    let (rect, canvas) = ui.allocate_exact_size(vec2(IMAGE[0] as f32, IMAGE[1] as f32), Sense::click_and_drag());
    let view = CanvasView::new(rect, Point::new(IMAGE[0] as f64 / 2.0, IMAGE[1] as f64 / 2.0), 1.0);
    let painter = ui.painter_at(rect);
    painter.image(image_texture(ui.ctx()).id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    let mut tool = ColorSamplerTool::new(&mut samplers, image_size());
    let _ = ToolHost::new(craft_ui::canvas::CanvasId::of(&canvas)).show(ui, &view, &canvas, &mut tool);
    let full = tool.take_error(ui.ctx());
    samplers.paint(&painter, &view);

    if let Some(line) = full {
        muted(ui, line.text);
    }
    let readouts = SamplerReadouts::new(&samplers, |p| image_pixel(p[0] as usize, p[1] as usize)).document("sampler-story").get(ui.ctx());
    let cmyk = CmykModel::new(CmykConverter::coated().clone());
    let channels = |model: &dyn ColorModel, values: &[u8]| {
        model.channels().iter().zip(values).map(|(c, v)| format!("{} {v}", c.label)).collect::<Vec<_>>().join("  ")
    };
    for SamplerReadout { number, position, rgb, cmyk: inks } in readouts.iter().copied() {
        let heading = craft_i18n::fmt(&labels.point, &[("n", &number.to_string())]);
        ui.add(Text::new(format!("{heading}  {}, {}   {}   {}", position[0], position[1], channels(&RgbModel, &rgb), channels(&cmyk, &inks))));
    }
}

fn show_paint_chip(ui: &mut Ui, args: &mut PaintChipArgs) {
    let numbers = LocaleNumbers::of(ui);
    let fallback = Srgba::rgb(0.24, 0.39, 0.87);
    let paints = shared(ui, "paint-chip", || {
        let teal_to_violet = GradientPreview { stops: vec![(0.0, Srgba::rgb(0.07, 0.65, 0.58)), (1.0, Srgba::rgb(0.56, 0.31, 0.78))] };
        [("Fill", Paint::Solid(fallback)), ("Gradient fill", Paint::Gradient(teal_to_violet)), ("Stroke", Paint::None)]
    });
    let mut paints = lock(&paints);
    let default_gradient = GradientPreview { stops: vec![(0.0, fallback), (1.0, Srgba::rgb(1.0, 1.0, 1.0))] };
    let shown = if args.no_color { 3 } else { 2 };
    Stack::new(Gap::Stack).show(ui, |ui| {
        for (index, (label, paint)) in paints.iter_mut().take(shown).enumerate() {
            let mut chip =
                PaintChip::new(("paint-chip", index), *label, paint, fallback, &numbers).none_allowed(args.no_color).enabled(!args.disabled);
            if args.gradient {
                chip = chip.gradient_editor(default_gradient.clone(), |_| None);
            }
            if let Some(event) = chip.show(ui).event
                && let Some(value) = event.value()
            {
                *paint = value.clone();
            }
        }
    });
}

fn show_choice(ui: &mut Ui, args: &mut ChoiceArgs) {
    let state = shared(ui, "choice", || (ChosenColor::Named("default".to_owned()), Srgba::rgb(0.2, 0.45, 0.7)));
    let mut state = lock(&state);
    let gray = |v: f32| Some(Srgba::rgb(v, v, v));
    let named = [
        NamedColor::new("default", "Default", None),
        NamedColor::new("black", "Black", gray(0.0)),
        NamedColor::new("darkGray", "Dark Gray", gray(0.2)),
        NamedColor::new("mediumGray", "Medium Gray", gray(0.5)),
        NamedColor::new("lightGray", "Light Gray", gray(0.75)),
    ];
    let (value, custom) = &mut *state;
    let shown =
        ColorChoice::new("pasteboard", "Pasteboard", value).named(named).custom(args.custom.then_some(*custom)).enabled(!args.disabled).show(ui);
    if let Some(EditEvent::Set { value: chosen, .. }) = shown.edit.event {
        *value = chosen;
    }
    if shown.pick_custom {
        *value = ChosenColor::Custom;
    }
    muted(
        ui,
        match value {
            ChosenColor::Named(id) => id.clone(),
            ChosenColor::Custom => hex(*custom),
        },
    );
}

/// The Color Guide story's state: the base, the rule the colours were made with, the wheel's
/// colours and its selection, and a note.
type HarmonyStory = (Srgba, Harmony, Vec<Srgba>, Option<usize>, String);

fn show_harmony(ui: &mut Ui, args: &mut HarmonyArgs) {
    let state = shared::<HarmonyStory>(ui, "harmony", || (Srgba::rgb(0.86, 0.24, 0.16), Harmony::Triad, Vec::new(), Some(0), String::new()));
    let panel_state = shared(ui, "harmony-panel", HarmonyPanelState::default);
    let mut guard = lock(&state);
    let (base, rule, colors, selected, note) = &mut *guard;
    let mut panel_state = lock(&panel_state);
    if colors.is_empty() || *rule != args.rule {
        *rule = args.rule;
        *colors = rule.apply(*base);
    }
    panel_state.rule = args.rule;
    let palette: Vec<Srgba> = SwatchLibrary::default()
        .groups
        .iter()
        .flat_map(|group| group.entries.iter().map(|entry| entry.color.to_srgb(CmykConverter::coated())))
        .collect();
    Row::new(Gap::Inline).show(ui, |ui| {
        let wheel = HarmonyWheel::new("harmony-wheel", "Harmony", colors).selected(*selected).show(ui);
        if wheel.pressed.is_some() {
            *selected = wheel.pressed;
        }
        for (index, target) in wheel.moved {
            let current = colors[index].to_hsb();
            let to = [target[0].unwrap_or(current.h), target[1].unwrap_or(current.s), target[2].unwrap_or(current.b)];
            move_on_wheel(colors, index, to, true);
        }
        Stack::new(Gap::Stack).show(ui, |ui| {
            let mut panel = HarmonyPanel::new("harmony-panel", &mut panel_state, *base);
            if args.limit_to_swatches {
                panel = panel.limit(&palette);
            }
            let shown = panel.show(ui);
            if let Some(color) = shown.chosen {
                *note = format!("Chosen {}", hex(color));
            }
            if shown.set_base {
                *note = "The base swatch sets the base to the current color.".to_owned();
            }
        });
    });
    args.rule = panel_state.rule;
    if !note.is_empty() {
        muted(ui, note.clone());
    }
}

fn show_lut_picker(ui: &mut Ui, args: &mut LutArgs) {
    let value = shared(ui, "lut", || LutChoice::None);
    let mut value = lock(&value);
    let shown = LutPicker::new("lut", &value).folder(None).enabled(!args.disabled).show(ui);
    if let Some(event) = shown.edit.event
        && let Some(chosen) = event.value()
    {
        *value = chosen.clone();
    }
    muted(ui, if shown.load_requested { "Load 3D LUT… asks the app for a file.".to_owned() } else { format!("{:?}", *value) });
}

fn show_profile_dialog(ui: &mut Ui, args: &mut ProfileArgs) {
    let (question, mode, rememberable) = (args.question, args.mode, args.rememberable);
    drive(
        ui,
        "profile-mismatch",
        &mut args.open,
        (question, mode, rememberable),
        move |_| {
            let (profile_mode, embedded, working) = match mode {
                ModeArg::Rgb => (ProfileMode::Rgb, "Display P3", "sRGB IEC61966-2.1"),
                ModeArg::Cmyk => (ProfileMode::Cmyk, "Coated FOGRA39 (ISO 12647-2:2004)", "U.S. Web Coated (SWOP) v2"),
                ModeArg::Gray => (ProfileMode::Gray, "Gray Gamma 2.2", "Dot Gain 20%"),
            };
            let question = match question {
                QuestionArg::Mismatch => ProfileQuestion::Mismatch { embedded: embedded.to_owned(), working: working.to_owned() },
                QuestionArg::Missing => ProfileQuestion::Missing { working: working.to_owned() },
            };
            ProfileMismatchDialog::new("Sunset over the bay.jpg", profile_mode, question).rememberable(rememberable)
        },
        |_| None,
        |answer| format!("{:?}{}", answer.action, if answer.remember { ", remembered" } else { "" }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The choice names are the rules' own names, in the rules' order, and the panel's choices
    /// cover every mode and ramp.
    #[test]
    fn rule_and_panel_choices_name_their_items() {
        assert_eq!(Harmony::OPTIONS.len(), Harmony::ALL.len());
        for (rule, name) in Harmony::ALL.iter().zip(Harmony::OPTIONS) {
            assert_eq!(rule.label(), *name);
        }
        assert_eq!(<ColorPanelMode as StoryChoice>::OPTIONS.len(), ColorPanelMode::ALL.len());
        assert_eq!(<ColorPanelRamp as StoryChoice>::OPTIONS.len(), ColorPanelRamp::ALL.len());
        for mode in ColorPanelMode::ALL {
            assert_eq!(ColorPanelMode::from_index(mode.index()), mode);
        }
    }

    /// Every story draws with its defaults over several passes, as a component tab and as an
    /// Overview tile (dialogs behind their button); none starts a screen capture.
    #[test]
    fn every_story_shows_with_its_defaults() {
        use craft_ui::theme::{Base, install};
        for overview in [false, true] {
            let ctx = Context::default();
            install(&ctx, &Base::suite_default());
            craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).install(&ctx);
            for story in stories() {
                let mut args = story.defaults();
                for _ in 0..3 {
                    craft_ui::story::with_overview(&ctx, overview, || {
                        ctx.run_ui(egui::RawInput::default(), |ui| (story.show)(ui, &mut args)).drop_without_applying_deltas();
                    });
                }
            }
        }
    }

    #[test]
    fn the_stories_are_the_declared_ids() {
        let ids: Vec<&str> = stories().iter().map(|s| s.id).collect();
        assert_eq!(ids, IDS);
    }
}
