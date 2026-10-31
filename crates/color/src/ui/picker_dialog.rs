//! [`ColorPickerDialog`] (F04.03): Photoshop's Color Picker as one value an app opens on its
//! [`DialogHost`](craft_ui::dialog::DialogHost) and one result it reads back.
//!
//! ```no_run
//! use craft_color::ui::{ColorPickerDialog, ColorPickerResult};
//! use craft_ui::dialog::{DialogHost, DialogResult};
//! # fn demo(ctx: &egui::Context, dialogs: &mut DialogHost, foreground: &mut craft_theme::Srgba) {
//! dialogs.open(ColorPickerDialog::foreground(*foreground));
//! dialogs.show(ctx);
//! if let Some(DialogResult::Ok(ColorPickerResult { color, .. })) = dialogs.take::<ColorPickerDialog>() {
//!     *foreground = color.color;
//! }
//! # }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_cms::convert::CmykConverter;
use craft_theme::Srgba;
use egui::Context;

use super::PickedColor;
use crate::AreaMode;
use craft_ui::dialog::{DialogId, DialogResult, DialogStack, DialogState, StackDialog, dialog_id};

craft_ui::words! {
    /// The Color Picker dialog's own words, English by default. The picker body's words are
    /// [`ColorPicker`](super::ColorPicker)'s; OK and Cancel are [`DialogLabels`](craft_ui::dialog::DialogLabels).
    #[non_exhaustive]
    pub struct ColorPickerLabels {
        foreground_title: "colorPicker.foregroundTitle" = "Color Picker (Foreground Color)";
        background_title: "colorPicker.backgroundTitle" = "Color Picker (Background Color)";
        /// The title for another colour; `{name}` is its name ("Color Picker (Vine Color)").
        named_title: "colorPicker.namedTitle" = "Color Picker ({name})";
        add_to_swatches: "colorPicker.addToSwatches" = "Add to Swatches";
        out_of_gamut: "colorPicker.outOfGamut" = "Out of gamut for printing: click for the nearest printable colour";
        not_web_color: "colorPicker.notWebColor" = "Not a web colour: click for the nearest web colour";
        pick_screen_color: "colorPicker.pickScreenColor" = "Pick screen color";
    }
}

/// What the picker keeps across opens and runs ([`Remembered`](craft_settings::Remembered) under
/// the scope [`ColorPickerMemory::SCOPE`]): the component the field shows and Only Web Colors,
/// as Photoshop. Stored when the dialog closes with OK.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ColorPickerMemory {
    pub mode: AreaMode,
    pub web_colors: bool,
}

impl ColorPickerMemory {
    /// The remembered scope.
    pub const SCOPE: &'static str = "colorPicker";
}

/// What a [`ColorPickerDialog`] answers with OK: the key it was opened with (which colour it was
/// for) and the colour.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorPickerResult {
    /// The dialog's key: `"foreground"`, `"background"`, or the app's own.
    pub key: String,
    /// The colour, with the model values the user typed while they still give it (CMYK inks,
    /// Lab values outside sRGB).
    pub color: PickedColor,
}

/// The dialog's title: given by the app, or one of the two the library words.
#[derive(Clone, Debug, PartialEq)]
enum Title {
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
///   without an edit changes nothing (PhotoCraft #4995).
/// - **Compare swatch:** "new" over "current"; a click on "current" sets the colour back.
/// - **Warnings:** with a [`gamut`](Self::gamut), the out-of-gamut chip shows when the colour
///   leaves the CMYK gamut (ΔE over 3); with Only Web Colors off, the web-colour chip shows for a
///   colour that is not one. A click on a chip takes its nearest colour.
/// - **Eyedropper:** while the dialog is the top dialog, the canvas samples into the new colour
///   ([`DropperState`](super::DropperState), armed by the dialog; the app's canvas delivers), as
///   Photoshop. The screen eyedropper button samples anywhere on the screen
///   ([`ScreenColorPicker`](super::ScreenColorPicker)).
/// - **Buttons:** Enter is OK and Esc is Cancel (not while a popup is open or a gesture runs).
///   Alt turns Cancel into Reset: the colour from when the dialog opened, the dialog stays open.
///   Add to Swatches keeps the dialog open; the app takes the colour with
///   [`take_add_to_swatches`](Self::take_add_to_swatches).
/// - **Preview:** with [`preview`](Self::preview) the app reads [`current`](Self::current) each
///   pass through `DialogHost::get::<ColorPickerDialog>()` and shows it live (Photoshop's text
///   colour); Cancel is the app's cue to put the old colour back.
/// - **Keyboard and AccessKit:** as [`ColorPicker`](super::ColorPicker)'s Full layout; Tab visits
///   the number fields, the hex field and the buttons; the dialog is a `Dialog` named by its title.
///
/// Photoshop's window is 534 by 376 points with a Color Libraries button; the library has no
/// colour books, so there is no such button.
pub struct ColorPickerDialog {
    id: DialogId,
    key: String,
    title: Title,
    opening: PickedColor,
    current: PickedColor,
    compare: bool,
    remember: bool,
    add_to_swatches: bool,
    gamut: Option<CmykConverter>,
    preview: bool,
    eyedropper: bool,
    canvas_samples: bool,
    memory: ColorPickerMemory,
    pending_swatch: Option<PickedColor>,
    state: DialogState<()>,
}

impl ColorPickerDialog {
    /// A picker for the colour `key` names, titled `title` (translated by the app), opening on
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
            gamut: None,
            preview: false,
            eyedropper: true,
            canvas_samples: true,
            memory: ColorPickerMemory::default(),
            pending_swatch: None,
            state: DialogState::default(),
        }
    }

    /// The model values the colour was picked in (a CMYK fill's inks): the fields show them as
    /// they were typed, not what the sRGB colour converts to (PhotoCraft #4196).
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
    /// ([`CmykModel`](crate::CmykModel)) and the out-of-gamut chip tests against it. Without it the
    /// dialog shows no CMYK fields and no gamut chip.
    pub fn gamut(mut self, converter: &CmykConverter) -> ColorPickerDialog {
        self.gamut = Some(converter.clone());
        self
    }

    /// Live preview: [`current`](Self::current) follows every edit for the app to show; off by
    /// default.
    pub fn preview(mut self, on: bool) -> ColorPickerDialog {
        self.preview = on;
        self
    }

    /// The screen eyedropper button; on by default where the platform can capture the screen.
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
        todo!()
    }
}
