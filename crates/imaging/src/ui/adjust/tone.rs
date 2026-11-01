//! The black, grey and white point pickers of Levels and Curves: the button row
//! ([`TonePickerRow`]) and the canvas tool that samples the image for the armed picker
//! ([`TonePickTool`]) (task AD-1).
//!
//! The library reports the sample; turning it into a Levels or Curves value needs the document's
//! colour management and mode, so the app does that (lead call L7), as with the Eyedropper.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::{AsIdSalt, IdSalt, Painter, Response, Ui};

use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolCursor, ToolPointer};

use super::AdjustLabels;

/// One of the three point pickers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TonePicker {
    /// Set Black Point.
    Black,
    /// Set Neutral Gray Point.
    Gray,
    /// Set White Point.
    White,
}

impl TonePicker {
    /// The three, in the row's order.
    pub const ALL: [TonePicker; 3] = [TonePicker::Black, TonePicker::Gray, TonePicker::White];

    /// The grey of the picker's tone dot: 0, 128, 255. An image tone, the same in every theme.
    // pc: crates/ui-egui/src/dialogs/adjust_dialog.rs:196-198.
    pub fn tone(self) -> u8 {
        match self {
            TonePicker::Black => 0,
            TonePicker::Gray => 128,
            TonePicker::White => 255,
        }
    }

    /// The picker's name ("Set Black Point"), its tooltip and AccessKit name.
    pub fn label(self, labels: &AdjustLabels) -> &str {
        match self {
            TonePicker::Black => &labels.set_black_point,
            TonePicker::Gray => &labels.set_gray_point,
            TonePicker::White => &labels.set_white_point,
        }
    }

    /// Whether the picker works in a document of this kind (Photoshop, PhotoCraft
    /// `picker_supported`): all three in RGB, Black and White in Grayscale, none in other modes.
    // pc: crates/ui-egui/src/dialogs/adjust_dialog.rs:31-34 `picker_supported`.
    pub fn supported(self, rgb: bool, gray: bool) -> bool {
        rgb || (gray && self != TonePicker::Gray)
    }
}

/// What [`TonePickerRow::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct TonePickerResponse {
    /// The row.
    pub response: Response,
    /// The armed picker changed this pass: `Some(Some(p))` arms `p`, `Some(None)` disarms. The
    /// host keeps the armed picker (and puts a [`TonePickTool`] on the canvas while one is).
    pub change: Option<Option<TonePicker>>,
}

/// The "Eyedroppers:" row: three icon buttons (Set Black Point, Set Neutral Gray Point, Set White
/// Point), each a pipette with its tone dot, and the "Unavailable in this color mode" note.
///
/// - A button is `icon_button` Md (28) showing `Icon::Eyedropper`, selected while its picker is
///   armed, with a tone dot ([`TonePicker::tone`]; radius `adjust.picker_dot` 3.5, centred 5.5 in
///   from the bottom-trailing corner, 1 pt ring in `field.border`).
/// - Click a disarmed button: arms it (`Some(Some(p))`); click the armed one again: disarms
///   (`Some(None)`). Esc while a picker button has focus and a picker is armed disarms it and
///   keeps the press (the dialog stays open); otherwise Esc is the dialog's.
/// - A picker the host disabled ([`enabled`](Self::enabled)) shows disabled and takes no click.
///   [`unavailable`](Self::unavailable) disables all three and shows the note after them
///   (`type.caption`, `text.faint`).
/// - Keyboard: each button is a Tab stop; Space or Enter acts as a click.
/// - AccessKit: each button is a `Button` with a toggled state, named by [`TonePicker::label`].
#[must_use = "show the row with `show`"]
pub struct TonePickerRow {
    id_salt: IdSalt,
    armed: Option<TonePicker>,
    enabled: [bool; 3],
    unavailable: bool,
    labels: Option<AdjustLabels>,
}

impl TonePickerRow {
    /// The row with `armed` shown selected.
    pub fn new(id_salt: impl AsIdSalt, armed: Option<TonePicker>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), armed, enabled: [true; 3], unavailable: false, labels: None }
    }

    /// Enables or disables one picker (the host passes [`TonePicker::supported`] for its mode).
    pub fn enabled(mut self, picker: TonePicker, enabled: bool) -> Self {
        self.enabled[picker as usize] = enabled;
        self
    }

    /// The document's mode has no point pickers (Lab, CMYK, Bitmap): every button is disabled
    /// and the note shows.
    pub fn unavailable(mut self, unavailable: bool) -> Self {
        self.unavailable = unavailable;
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: AdjustLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the row and reports an arm or disarm.
    pub fn show(self, ui: &mut Ui) -> TonePickerResponse {
        // pc: crates/ui-egui/src/dialogs/adjust_dialog.rs:177-183 `picker_button` (28 pt pipette,
        // dot 3.5 at 5.5, ring `field_border`, `WidgetInfo::selected`), :186-216 `picker_controls`
        // ("Eyedroppers:" caption, the note at 11 pt `text_faint`).
        // new: a second click and Esc disarm (PhotoCraft keeps the picker armed); the look from
        // `AdjustStyle::of`.
        todo!()
    }
}

/// A sample the armed picker took on the canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TonePick {
    pub picker: TonePicker,
    /// The document point sampled, in document units.
    pub at: [f64; 2],
    /// The composite colour there, as the host's `sample` function read it (the committed merged
    /// document, PhotoCraft `sample_at`).
    pub color: Srgba,
}

/// The canvas tool of an armed point picker: a press samples the host's `sample` function at the
/// document point and reports a [`TonePick`] for the picker (taken with [`take`](Self::take)
/// after the tool host's pass). Off the image `sample` returns `None` and nothing is reported. A
/// drag does nothing more; the release ends the gesture. The cursor is the pipette
/// (`Icon::Eyedropper`, hotspot at its tip, as craft-color's `EyedropperTool`, whose shape this
/// tool follows).
///
/// The host applies the pick with its engine (Photoshop maps the sample to black, neutral grey
/// or white in each channel) and passes the new value back to the panel, which takes it as an
/// outside change; the picker stays armed until the row disarms it.
pub struct TonePickTool<'a> {
    picker: TonePicker,
    sample: Box<dyn FnMut([f64; 2]) -> Option<Srgba> + 'a>,
    picked: Option<TonePick>,
}

impl<'a> TonePickTool<'a> {
    /// The tool for `picker`, sampling the composite with `sample` (document units in, `None` off
    /// the image).
    pub fn new(picker: TonePicker, sample: impl FnMut([f64; 2]) -> Option<Srgba> + 'a) -> Self {
        Self { picker, sample: Box::new(sample), picked: None }
    }

    /// The picker the tool samples for.
    pub fn picker(&self) -> TonePicker {
        self.picker
    }

    /// The pick of the last press, once.
    pub fn take(&mut self) -> Option<TonePick> {
        self.picked.take()
    }
}

impl CanvasTool for TonePickTool<'_> {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // craft-color: crates/color/src/ui/eyedropper.rs:327-330 (the pipette, hotspot (2, 22)).
        todo!()
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/adjust_dialog.rs:52-80 `sample_at` (the armed picker samples
        // the composite at the click).
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // new: nothing; the press took the sample.
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // new: nothing to commit.
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // new: drops a pick not yet taken.
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // new: no feedback beyond the cursor.
        todo!()
    }

    fn autoscrolls(&self) -> bool {
        false
    }
}
