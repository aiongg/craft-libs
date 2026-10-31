//! [`ImageSizeDialog`], [`CanvasSizeDialog`], [`LinkedSizeFields`] and [`AnchorGrid`]: Image Size
//! and Canvas Size with units, file size and the 3×3 anchor. In `sizing`, not `size`: `size.rs`
//! holds the frame's own size rules ([`Width`](super::Width), [`Body`](super::Body)).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Context, Response, Ui};

use super::dialog_host::{DialogResult, StackDialog};
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::edit::EditResponse;

/// A length unit of a size field. Pixels and percent are the pixel units; the rest are physical
/// and convert through the resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SizeUnit {
    Pixels,
    Percent,
    Inches,
    Centimetres,
    Millimetres,
    Points,
    Picas,
}

impl SizeUnit {
    /// Every unit, in menu order.
    pub const ALL: [SizeUnit; 7] =
        [SizeUnit::Pixels, SizeUnit::Percent, SizeUnit::Inches, SizeUnit::Centimetres, SizeUnit::Millimetres, SizeUnit::Points, SizeUnit::Picas];

    pub fn is_pixel_unit(self) -> bool {
        matches!(self, SizeUnit::Pixels | SizeUnit::Percent)
    }

    /// `pixels` in this unit, given the original pixel size (for percent) and the resolution.
    // pc: crates/ui-egui/src/dialogs/sizing.rs:120 `to_unit`.
    pub fn from_pixels(self, pixels: f64, original_pixels: f64, pixels_per_inch: f64) -> f64 {
        todo!()
    }

    /// A value in this unit as pixels.
    // pc: crates/ui-egui/src/dialogs/sizing.rs:132 `from_unit`.
    pub fn to_pixels(self, value: f64, original_pixels: f64, pixels_per_inch: f64) -> f64 {
        todo!()
    }
}

/// Resolution units of Image Size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResolutionUnit {
    PixelsPerInch,
    PixelsPerCentimetre,
}

/// Photoshop's Resample list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Resample {
    /// Resample off: the pixel count is fixed, a physical size changes the resolution.
    None,
    Automatic,
    PreserveDetails,
    Bicubic,
    BicubicSmoother,
    BicubicSharper,
    Bilinear,
    NearestNeighbor,
}

/// A cell of the 3×3 anchor: which edge or corner of the current canvas stays put.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    #[default]
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    /// Row-major.
    pub const ALL: [Anchor; 9] = [
        Anchor::TopLeft,
        Anchor::Top,
        Anchor::TopRight,
        Anchor::Left,
        Anchor::Center,
        Anchor::Right,
        Anchor::BottomLeft,
        Anchor::Bottom,
        Anchor::BottomRight,
    ];

    /// Column and row, 0..3.
    pub fn cell(self) -> (usize, usize) {
        let index = Anchor::ALL.iter().position(|a| *a == self).unwrap_or(4);
        (index % 3, index / 3)
    }
}

/// What fills the new area of a grown canvas.
#[derive(Clone, Debug, PartialEq)]
pub enum CanvasExtension {
    Foreground,
    Background,
    White,
    Black,
    Gray,
    /// Picked in the colour picker, linear sRGB.
    Other([f32; 3]),
    /// Transparent, for a document without a Background layer (the list is disabled then).
    Transparent,
}

/// The sizes the host hands a size dialog and gets back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelSize {
    pub width: u32,
    pub height: u32,
    pub pixels_per_inch: f64,
}

/// What the user chose in Image Size.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageSizeResult {
    pub size: PixelSize,
    pub resample: Resample,
    /// Scale Styles: layer effects scale with the image (enabled while proportions are
    /// constrained).
    pub scale_styles: bool,
}

/// What the user chose in Canvas Size.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasSizeResult {
    pub size: PixelSize,
    pub anchor: Anchor,
    pub extension: CanvasExtension,
}

/// Width and height fields with a unit menu each, linked by the chain toggle so a change in one
/// scales the other, plus the resolution field. Serves Image Size, Canvas Size, export and
/// Save for Web.
///
/// The user sees two [`NumberField`](crate::controls::NumberField)s with a unit
/// [`Select`](crate::controls::Select) after each, the chain toggle bracket on the left
/// ([`Form::linked`](crate::composites::Form::linked)), and a resolution row. Typing in a linked
/// field updates the other on each keystroke (Photoshop); the fields show the current pixel
/// values converted to the unit; percent is relative to the opening size. In pixel units with
/// Resample off the fields are disabled.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkedSizeFields {
    /// The size the dialog opened with (percent and the readouts refer to it).
    pub original: PixelSize,
    /// The size being edited.
    pub current: PixelSize,
    pub unit: SizeUnit,
    pub resolution_unit: ResolutionUnit,
    pub constrain: bool,
    /// Whether the pixel count may change (Image Size with Resample on; always in Canvas Size).
    pub resampling: bool,
    /// A relative size (Canvas Size's Relative box): the fields show the change, not the size.
    pub relative: bool,
}

impl LinkedSizeFields {
    pub fn new(original: PixelSize) -> Self {
        Self {
            original,
            current: original,
            unit: SizeUnit::Pixels,
            resolution_unit: ResolutionUnit::PixelsPerInch,
            constrain: true,
            resampling: true,
            relative: false,
        }
    }

    /// Draws the width, height and (when `with_resolution`) resolution rows into `form`,
    /// returning the edit of the pixel size this pass.
    // pc: crates/ui-egui/src/dialogs/sizing.rs:177 `dim_field`, :203 `unit_dropdown`, and the
    // rows of :211 `image_size`.
    pub fn rows(&mut self, form: &mut crate::composites::Form<'_>, with_resolution: bool) -> EditResponse<PixelSize> {
        todo!()
    }

    /// Sets the width in pixels, scaling the height when constrained.
    // pc: crates/ui-egui/src/dialogs/sizing.rs:211 `image_size` (the `w_row` branch).
    pub fn set_width(&mut self, pixels: f64) {
        todo!()
    }

    /// Sets the height in pixels, scaling the width when constrained.
    pub fn set_height(&mut self, pixels: f64) {
        todo!()
    }
}

/// The 3×3 anchor of Canvas Size: nine cells, the chosen one marked, arrows in the neighbouring
/// cells pointing where the canvas grows (they flip when it shrinks). Click picks; with the
/// focus, the arrow keys move the mark and Space or Enter picks (gap 10).
pub struct AnchorGrid<'a> {
    anchor: &'a mut Anchor,
    /// The change in width and height in pixels; the arrows' direction.
    delta: (f64, f64),
}

impl<'a> AnchorGrid<'a> {
    pub fn new(anchor: &'a mut Anchor, delta: (f64, f64)) -> Self {
        Self { anchor, delta }
    }

    /// Draws the grid. `changed()` is set on the pass the anchor changes.
    // pc: crates/ui-egui/src/dialogs/sizing.rs:416 `anchor_grid` (24 pt cells, the arrow
    // geometry), with the library's field fill and focus ring; the keyboard part is new.
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}

/// Byte counts as the dialogs show them (`1.2M`, `850.3K`).
// pc: crates/ui-egui/src/dialogs/sizing.rs:144 `human_bytes`.
pub fn human_bytes(bytes: f64) -> String {
    todo!()
}

/// Image › Image Size: the image's file size readout and dimensions, Fit To presets, the
/// [`LinkedSizeFields`], Resample with its list, Scale Styles; OK, Cancel, Reset on Alt
/// (spec question 4).
///
/// The user sees the readouts at the top ("Image Size: 2.3M (was 1.1M)", "Dimensions: 1200 px ×
/// 800 px"), the linked fields, the Resample check box and its method list, and the footer.
/// With Resample off the pixel fields are disabled and a physical size changes the resolution.
/// Result: [`ImageSizeResult`].
pub struct ImageSizeDialog {
    id: DialogId,
    fields: LinkedSizeFields,
    resample: Resample,
    scale_styles: bool,
    bytes_per_pixel: f64,
    state: DialogState<(LinkedSizeFields, Resample, bool)>,
}

impl ImageSizeDialog {
    pub fn new(original: PixelSize) -> Self {
        Self {
            id: dialog_id("image-size"),
            fields: LinkedSizeFields::new(original),
            resample: Resample::Automatic,
            scale_styles: true,
            bytes_per_pixel: 4.0,
            state: DialogState::new(),
        }
    }

    /// Bytes per pixel of the document, for the file size readout (4 for 8-bit RGBA).
    pub fn bytes_per_pixel(mut self, bytes: f64) -> Self {
        self.bytes_per_pixel = bytes;
        self
    }

    pub fn resample(mut self, resample: Resample) -> Self {
        self.resample = resample;
        self
    }

    pub fn fields(&self) -> &LinkedSizeFields {
        &self.fields
    }
}

impl StackDialog for ImageSizeDialog {
    type Result = ImageSizeResult;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/sizing.rs:211 `image_size`, on `Dialog` with
    // `Footer::standard().reset(..)`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<ImageSizeResult>> {
        todo!()
    }
}

/// Image › Canvas Size: Current Size readouts, New Size with the [`LinkedSizeFields`], the
/// Relative box, the [`AnchorGrid`] and the Canvas extension colour list; OK, Cancel, Reset on
/// Alt (spec question 4). Result: [`CanvasSizeResult`].
pub struct CanvasSizeDialog {
    id: DialogId,
    fields: LinkedSizeFields,
    anchor: Anchor,
    extension: CanvasExtension,
    /// Whether the extension list is offered (a document with a Background layer).
    has_background: bool,
    bytes_per_pixel: f64,
    state: DialogState<(LinkedSizeFields, Anchor, CanvasExtension)>,
}

impl CanvasSizeDialog {
    pub fn new(original: PixelSize) -> Self {
        Self {
            id: dialog_id("canvas-size"),
            fields: LinkedSizeFields::new(original),
            anchor: Anchor::Center,
            extension: CanvasExtension::Background,
            has_background: true,
            bytes_per_pixel: 4.0,
            state: DialogState::new(),
        }
    }

    pub fn bytes_per_pixel(mut self, bytes: f64) -> Self {
        self.bytes_per_pixel = bytes;
        self
    }

    /// Without a Background layer the extension is transparent and the list is disabled.
    pub fn has_background(mut self, has_background: bool) -> Self {
        self.has_background = has_background;
        self
    }

    pub fn fields(&self) -> &LinkedSizeFields {
        &self.fields
    }
}

impl StackDialog for CanvasSizeDialog {
    type Result = CanvasSizeResult;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/sizing.rs:352 `canvas_size`; the extension list is new
    // (Photoshop's Canvas extension color; PhotoCraft has none).
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<CanvasSizeResult>> {
        todo!()
    }
}
