//! [`ColorPickerDialog`] (F04.03): Photoshop's Color Picker as one value an app opens on its
//! [`DialogHost`](craft_ui::dialog::DialogHost) and one result it reads back.
//!
//! ```no_run
//! use craft_color::ui::{ColorPickerDialog, ColorPickerResult};
//! use craft_ui::dialog::{DialogHost, DialogResult};
//! # fn demo(ctx: &egui::Context, dialogs: &mut DialogHost, foreground: &mut craft_theme::Srgba) {
//! dialogs.open(ColorPickerDialog::foreground(*foreground));
//! dialogs.show(ctx);
//! if let Some(DialogResult::Ok(ColorPickerResult { picked, .. })) = dialogs.take::<ColorPickerDialog>() {
//!     *foreground = picked.color;
//! }
//! # }
//! ```

use craft_cms::convert::CmykConverter;
use craft_host::Host;
use craft_settings::Remembered;
use craft_theme::{ColorModel, Srgba, builtin_models};
use craft_ui::dialog::{
    ButtonRole, ColorRequest, Dialog, DialogId, DialogLabels, DialogResult, DialogStack, DialogState, Footer, FooterButton, InitialFocus, Outcome,
    StackDialog, dialog_id,
};
use craft_ui::localized::{Labels, LocaleNumbers};
use egui::{Context, Id, IdSalt};

use super::{ColorPicker, ColorPickerLayout, ColorWarning, DropperState, PickedColor, ScreenColorPicker};
use crate::{AreaMode, CmykModel, not_web_safe, web_safe};

/// The picker's id salt inside the dialog.
const PICKER: &str = "picker";

craft_ui::words! {
    /// The Color Picker dialog's own words, English by default. The picker body's words are
    /// [`ColorPicker`](super::ColorPicker)'s; OK and Cancel are [`DialogLabels`](craft_ui::dialog::DialogLabels).
    #[non_exhaustive]
    pub struct ColorPickerLabels {
        /// The title when a component's request gives none.
        title: "colorPicker.title" = "Color Picker";
        foreground_title: "colorPicker.foregroundTitle" = "Color Picker (Foreground Color)";
        background_title: "colorPicker.backgroundTitle" = "Color Picker (Background Color)";
        /// The title for another colour; `{name}` is its name ("Color Picker (Vine Color)").
        named_title: "colorPicker.namedTitle" = "Color Picker ({name})";
        add_to_swatches: "colorPicker.addToSwatches" = "Add to Swatches";
        out_of_gamut: "colorPicker.outOfGamut" = "Out of gamut for printing: click for the nearest printable color";
        not_web_color: "colorPicker.notWebColor" = "Not a web color: click for the nearest web color";
        pick_screen_color: "colorPicker.pickScreenColor" = "Pick screen color";
    }
}

/// What the picker keeps across opens and runs ([`Remembered`](craft_settings::Remembered) under
/// the scope `colorPicker.memory`): the component the field shows and Only Web Colors,
/// as Photoshop. Stored when the dialog closes with OK.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ColorPickerMemory {
    /// The component the area and strip edit.
    pub mode: AreaMode,
    /// Only Web Colors: snap to the 216 web-safe colours.
    pub web_colors: bool,
}

/// The remembered scope of [`ColorPickerMemory`].
pub(super) const MEMORY_SCOPE: &str = "colorPicker.memory";

/// What a [`ColorPickerDialog`] answers with OK: the key it was opened with (which colour it was
/// for) and the colour.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorPickerResult {
    /// The dialog's key: `"foreground"`, `"background"`, or the app's own.
    pub key: String,
    /// The colour, with the model values the user typed while they still give it (CMYK inks,
    /// Lab values outside sRGB).
    pub picked: PickedColor,
}

/// The dialog's title: given by the app, or one of the two the library words.
#[derive(Clone, Debug, PartialEq)]
enum Title {
    Default,
    Foreground,
    Background,
    Given(String),
}

/// Photoshop's Color Picker dialog on the [`DialogHost`](craft_ui::dialog::DialogHost):
/// [`ColorPicker`](super::ColorPicker) in its Full layout with the compare swatch, the
/// out-of-gamut and web-colour warnings, Only Web Colors, the eyedropper, and OK, Cancel and Add
/// to Swatches in a column at its trailing side, in a [`Dialog`](craft_ui::dialog::Dialog) with
/// no footer.
///
/// - **Opening:** the hex field has the focus with its text selected, so typing replaces it. The
///   field shows the component it showed last time, and Only Web Colors is as it was left
///   ([`ColorPickerMemory`]); with Only Web Colors on, the opening colour is snapped to the
///   nearest web colour and the compare swatch's "current" half shows the snapped colour, so OK
///   without an edit changes nothing (storytold/photocraft#4995).
/// - **Compare swatch:** "new" over "current"; a click on "current" sets the colour back.
/// - **Warnings:** the out-of-gamut chip shows when the colour
///   leaves the CMYK gamut of the [`cmyk`](Self::cmyk) converter (ΔE over 3); with Only Web Colors off, the web-colour chip shows for a
///   colour that is not one. A click on a chip takes its nearest colour.
/// - **Eyedropper:** while the dialog is the top dialog, the canvas samples into the new colour
///   ([`DropperState`](super::DropperState), armed by the dialog; the app's canvas delivers), as
///   Photoshop. The screen eyedropper button samples anywhere on the screen
///   ([`ScreenColorPicker`](super::ScreenColorPicker)).
/// - **Buttons:** Enter is OK and Esc is Cancel (not while a popup is open or a gesture runs).
///   Alt turns Cancel into Reset: the colour from when the dialog opened, the dialog stays open.
///   Add to Swatches keeps the dialog open; the app takes the colour with
///   [`take_add_to_swatches`](Self::take_add_to_swatches).
/// - **Preview:** [`current`](Self::current) follows every edit, so an app that previews reads it each
///   pass through `DialogHost::get::<ColorPickerDialog>()` and shows it live (Photoshop's text
///   colour); Cancel is the app's cue to put the old colour back.
/// - **Keyboard and AccessKit:** as [`ColorPicker`](super::ColorPicker)'s Full layout; Tab visits
///   the number fields, the hex field and the buttons; the dialog is a `Dialog` named by its title.
///
/// Photoshop's window is 534 by 376 points with a Color Libraries button; the library has no
/// colour books, so there is no such button.
pub struct ColorPickerDialog {
    pub(super) id: DialogId,
    key: String,
    title: Title,
    opening: PickedColor,
    current: PickedColor,
    compare: bool,
    remember: bool,
    add_to_swatches: bool,
    cmyk: Option<CmykConverter>,
    eyedropper: bool,
    canvas_samples: bool,
    memory: ColorPickerMemory,
    pending_swatch: Option<PickedColor>,
    state: DialogState<()>,
    started: bool,
    seeded: bool,
    picker: Id,
    gesture_start: Option<PickedColor>,
    /// What the picker's fields are seeded with next pass, when that is not the opening colour
    /// ([`set_color`](ColorPickerDialog::set_color)).
    seed: Option<PickedColor>,
}

impl ColorPickerDialog {
    /// A picker for what a component's [`ColorRequest`] asked: its key, its title (the default
    /// title when it has none) and its colour. The app opens it on its `DialogHost` and answers
    /// the component with the result's `key` and `picked.color`.
    pub fn from_request(request: ColorRequest) -> ColorPickerDialog {
        let ColorRequest { key, title, color } = request;
        let title = title.map_or(Title::Default, Title::Given);
        ColorPickerDialog::with_title(key, title, color)
    }

    /// A picker for the colour `key` names, titled `title` (a catalog key, translated every pass), opening on
    /// `color`.
    pub fn new(key: impl Into<String>, title: impl Into<String>, color: Srgba) -> ColorPickerDialog {
        ColorPickerDialog::with_title(key.into(), Title::Given(title.into()), color)
    }

    /// The foreground colour's picker: key `"foreground"`, "Color Picker (Foreground Color)".
    pub fn foreground(color: Srgba) -> ColorPickerDialog {
        ColorPickerDialog::with_title("foreground".into(), Title::Foreground, color)
    }

    /// The background colour's picker: key `"background"`, "Color Picker (Background Color)".
    pub fn background(color: Srgba) -> ColorPickerDialog {
        ColorPickerDialog::with_title("background".into(), Title::Background, color)
    }

    fn with_title(key: String, title: Title, color: Srgba) -> ColorPickerDialog {
        let opening = PickedColor { color, model: None };
        ColorPickerDialog {
            id: dialog_id(("color-picker", key.as_str())),
            key,
            title,
            opening,
            current: opening,
            compare: true,
            remember: true,
            add_to_swatches: true,
            cmyk: None,
            eyedropper: true,
            canvas_samples: true,
            memory: ColorPickerMemory::default(),
            pending_swatch: None,
            state: DialogState::default(),
            started: false,
            seeded: false,
            picker: Id::NULL,
            gesture_start: None,
            seed: None,
        }
    }

    /// The model values the colour was picked in (a CMYK fill's inks): the fields show them as
    /// they were typed, not what the sRGB colour converts to (storytold/photocraft#4196).
    pub fn kept(mut self, values: craft_theme::ModelValues) -> ColorPickerDialog {
        self.opening.model = Some(values);
        self.current.model = Some(values);
        self
    }

    /// The compare swatch beside the field; on by default.
    pub fn compare(mut self, on: bool) -> ColorPickerDialog {
        self.compare = on;
        self
    }

    /// Whether the field's component and Only Web Colors are recalled on open and stored on OK
    /// ([`ColorPickerMemory`]); on by default. A picker opened by a feature that wants its own
    /// start turns it off.
    pub fn web_colors_remembered(mut self, on: bool) -> ColorPickerDialog {
        self.remember = on;
        self
    }

    /// The Add to Swatches button; on by default.
    pub fn add_to_swatches(mut self, on: bool) -> ColorPickerDialog {
        self.add_to_swatches = on;
        self
    }

    /// The document's CMYK converter: the C, M, Y, K fields convert through it
    /// ([`CmykModel`](crate::CmykModel)) and the out-of-gamut chip tests against it. Without it
    /// the dialog uses the built-in coated converter ([`CmykConverter::coated`]), as PhotoCraft
    /// does for a document without a CMYK profile.
    pub fn cmyk(mut self, converter: &CmykConverter) -> ColorPickerDialog {
        self.cmyk = Some(converter.clone());
        self
    }

    /// The screen eyedropper button; on by default, and shown only where the platform can capture
    /// the screen ([`ScreenColorPicker::available`]).
    pub fn eyedropper(mut self, on: bool) -> ColorPickerDialog {
        self.eyedropper = on;
        self
    }

    /// Whether the canvas samples into the dialog while it is the top dialog; on by default.
    pub fn canvas_samples(mut self, on: bool) -> ColorPickerDialog {
        self.canvas_samples = on;
        self
    }

    /// The key the dialog was opened with.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The colour as it stands this pass.
    pub fn current(&self) -> PickedColor {
        self.current
    }

    /// The colour the dialog opened with: as given, until the first pass; then, with Only Web
    /// Colors on, snapped to the nearest web colour, which is what the compare swatch's
    /// "current" half shows and what Reset returns to.
    pub fn opening(&self) -> PickedColor {
        self.opening
    }

    /// Sets the colour as an edit would, for automation and scripts (`ui.dialog.set color`): the
    /// fields show `picked`'s model values when it has them, and the colour is what OK answers.
    /// The compare swatch keeps the opening colour.
    pub fn set_color(&mut self, picked: PickedColor) {
        self.current = picked;
        self.seed = Some(picked);
        self.seeded = false;
    }

    /// The colour Add to Swatches was pressed for, once; `None` otherwise. The app reads it each
    /// pass through `DialogHost::get_mut::<ColorPickerDialog>()` and adds the swatch
    /// ([`SwatchLibrary::add_picked`](super::SwatchLibrary::add_picked)).
    pub fn take_add_to_swatches(&mut self) -> Option<PickedColor> {
        self.pending_swatch.take()
    }
}

impl StackDialog for ColorPickerDialog {
    type Result = ColorPickerResult;

    fn id(&self) -> DialogId {
        self.id
    }

    /// Answers OK as the OK button would, without drawing: the dialog's start (recalled memory,
    /// the Only Web Colors snap) has run, the memory is stored, and the colour is the current one.
    fn confirm(&mut self, ctx: &Context) -> Option<ColorPickerResult> {
        if !self.started {
            self.start(ctx);
        }
        Some(self.accept(ctx))
    }

    /// The host closed the dialog from outside: its dropper ends with it.
    fn cancelled(&mut self, ctx: &Context) {
        DropperState::disarm(ctx, self.id);
    }

    // pc: crates/ui-egui/src/dialogs/color_picker_ui.rs:141 `open` and :169 `fields_with_web`
    // (recall memory; snap the opening colour when web-only, `opening` follows), :421 `body`
    // (built as craft-libs `crates/color/src/ui/dialog_tests.rs:82-105` composes it: `ColorPicker::new(..)
    // .layout(Full).compare(opening).warning(gamut).warning(web).web_colors(&mut memory.web_colors)
    // .mode(&mut memory.mode).models(HSB, RGB, Lab, CmykModel).eyedropper(..).side_actions([OK,
    // Cancel or Reset while Alt is held, Add to Swatches])` in `Dialog::new(..).footer(Footer::none())`),
    // :274 `reset` (Alt+Cancel), :755 `take_add_swatch`, :762 `confirm` (the result);
    // crates/ui-egui/src/dialogs.rs:313 (web-only written back on close). Canvas samples:
    // `DropperState::arm` while `stack.is_top(self.id)`, `DropperState::take` into
    // `ColorPicker::sampled`. Screen samples: `ScreenColorPicker::take` likewise.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<ColorPickerResult>> {
        let labels = ColorPickerLabels::of_ctx(ctx);
        let words = DialogLabels::of_ctx(ctx);
        if !self.started {
            self.start(ctx);
        }
        let owner = self.id;
        let mut sample = None;
        if self.canvas_samples && stack.is_top(owner) {
            sample = DropperState::take(ctx, owner);
            DropperState::arm(ctx, owner);
        } else {
            DropperState::disarm(ctx, owner);
        }
        if self.eyedropper {
            sample = ScreenColorPicker::take(ctx, owner).or(sample);
        }
        if let Some(color) = sample
            && self.picker != Id::NULL
        {
            ColorPicker::sampled(ctx, self.picker, color);
        }

        let title = match &self.title {
            Title::Default => labels.title.clone(),
            Title::Foreground => labels.foreground_title.clone(),
            Title::Background => labels.background_title.clone(),
            Title::Given(title) => craft_ui::dialog::translated(ctx, title),
        };
        let numbers = LocaleNumbers::of_ctx(ctx);
        let alt = ctx.input(|i| i.modifiers.alt);
        let cmyk = CmykModel::new(self.cmyk.clone().unwrap_or_else(|| CmykConverter::coated().clone()));
        let screen = self.eyedropper && ScreenColorPicker::available(ctx);
        let mut actions = vec![
            FooterButton::new(ButtonRole::Default, words.ok.clone()),
            FooterButton::new(ButtonRole::Cancel, if alt { words.reset.clone() } else { words.cancel.clone() }),
        ];
        if self.add_to_swatches {
            actions.push(FooterButton::new(ButtonRole::Alternate, labels.add_to_swatches.clone()));
        }

        let mut clicked = None;
        let Self { current, opening, compare, memory, gesture_start, seeded, picker, state, key, seed, .. } = self;
        let web_only = memory.web_colors;
        let color = current.color;
        let shown = Dialog::new(("color-picker", key.as_str()), title.as_str())
            .stack(stack)
            .footer(Footer::none())
            .initial_focus(InitialFocus::None)
            .show(ctx, state, &mut (), |ui, ()| {
                let id = ui.make_persistent_id(IdSalt::new(PICKER));
                if !*seeded {
                    let from = seed.take().unwrap_or(*opening);
                    if let Some(values) = from.model {
                        ColorPicker::keep(ui.ctx(), id, from.color, values);
                    }
                    *seeded = true;
                }
                let mut models: Vec<&dyn ColorModel> = builtin_models().into_iter().filter(|m| m.default_on().full).collect();
                models.push(&cmyk);
                let mut body = ColorPicker::new(PICKER, title.as_str(), color, &numbers)
                    .layout(ColorPickerLayout::Full)
                    .models(models)
                    .mode(&mut memory.mode)
                    .web_colors(&mut memory.web_colors)
                    .side_actions(actions);
                if *compare {
                    body = body.compare(opening.color);
                }
                let rgb = [color.r, color.g, color.b];
                if let Some(nearest) = cmyk.converter().nearest_in_gamut(rgb, CmykConverter::PICKER_GAMUT_DE) {
                    body = body
                        .warning(ColorWarning::new(labels.out_of_gamut.clone(), Srgba { r: nearest[0], g: nearest[1], b: nearest[2], a: color.a }));
                }
                if !web_only && let Some(nearest) = not_web_safe(rgb) {
                    body = body.warning(ColorWarning::web_colors(
                        labels.not_web_color.clone(),
                        Srgba { r: nearest[0], g: nearest[1], b: nearest[2], a: color.a },
                    ));
                }
                if screen {
                    body = body.eyedropper(labels.pick_screen_color.clone()).eyedropper_busy(ScreenColorPicker::busy(ui.ctx()));
                }
                let shown = body.show(ui);
                *picker = shown.edit.response.id;
                match shown.edit.event {
                    Some(craft_ui::edit::EditEvent::Begin { value, .. }) => {
                        *gesture_start = Some(*current);
                        if let Some(value) = value {
                            *current = value;
                        }
                    }
                    Some(craft_ui::edit::EditEvent::Cancel { .. }) => {
                        if let Some(start) = gesture_start.take() {
                            *current = start;
                        }
                    }
                    Some(event) => {
                        if let Some(value) = event.value() {
                            *current = *value;
                        }
                    }
                    None => {}
                }
                if shown.eyedropper_requested {
                    ScreenColorPicker::open(ui.ctx(), owner);
                }
                if let Some(button) = &shown.eyedropper
                    && let Some(color) = ScreenColorPicker::hold_drag(ui, button, owner)
                {
                    ColorPicker::sampled(ui.ctx(), *picker, color);
                }
                clicked = shown.side_clicked;
            });

        let mut outcome = shown.outcome;
        if let Some(click) = clicked {
            match click.role {
                ButtonRole::Default => outcome = Some(Outcome::Ok),
                ButtonRole::Cancel if alt => outcome = Some(Outcome::Reset),
                ButtonRole::Cancel => outcome = Some(Outcome::Cancel),
                ButtonRole::Alternate => self.pending_swatch = Some(self.current),
                ButtonRole::Apply => {}
            }
            if matches!(outcome, Some(Outcome::Ok | Outcome::Cancel)) && self.state.is_open() {
                self.state.close();
            }
        }
        match outcome {
            Some(Outcome::Reset) => {
                self.current = self.opening;
                self.seeded = false;
                None
            }
            Some(Outcome::Ok) => Some(DialogResult::Ok(self.accept(ctx))),
            Some(Outcome::Cancel | Outcome::Close) => {
                DropperState::disarm(ctx, owner);
                Some(DialogResult::Cancel)
            }
            _ => None,
        }
    }
}

impl ColorPickerDialog {
    /// What OK does: ends the dropper, stores the remembered field component and Only Web Colors,
    /// and answers with the colour.
    fn accept(&mut self, ctx: &Context) -> ColorPickerResult {
        DropperState::disarm(ctx, self.id);
        if self.remember
            && let Some(host) = Host::try_of_ctx(ctx)
        {
            Remembered::<ColorPickerMemory>::new(MEMORY_SCOPE).store(&host, &self.memory);
        }
        ColorPickerResult { key: self.key.clone(), picked: self.current }
    }

    /// The first pass: recalls the remembered field component and Only Web Colors, and with Only
    /// Web Colors on snaps the opening colour, which the compare swatch's current half follows
    /// (storytold/photocraft#4995).
    fn start(&mut self, ctx: &Context) {
        self.started = true;
        if self.remember
            && let Some(host) = Host::try_of_ctx(ctx)
            && let Some(memory) = Remembered::<ColorPickerMemory>::new(MEMORY_SCOPE).recall(&host)
        {
            self.memory = memory;
        }
        if self.memory.web_colors {
            let [r, g, b] = web_safe([self.opening.color.r, self.opening.color.g, self.opening.color.b]);
            self.opening = PickedColor { color: Srgba { r, g, b, a: self.opening.color.a }, model: None };
            self.current = self.opening;
        }
    }
}
