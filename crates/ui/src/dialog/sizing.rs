//! [`ImageSizeDialog`], [`CanvasSizeDialog`], [`LinkedSizeFields`] and [`AnchorGrid`]: Image Size
//! and Canvas Size with units, file size and the 3×3 anchor. In `sizing`, not `size`: `size.rs`
//! holds the frame's own size rules ([`Width`](super::Width), [`Body`](super::Body)).

use craft_theme::ControlSize;
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{Align, Color32, Context, Label, Layout, Pos2, Rect, Response, Shape, Stroke, Ui, UiBuilder, Vec2, vec2};
use serde::{Deserialize, Serialize};

use super::dialog::{Dialog, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::footer::{DialogLabels, Footer, Reset};
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::behavior::RadioSelection;
use crate::composites::{Form, FormGrid, RowLabel, SectionHeader};
use crate::controls::{Checkbox, NumberField, Segmented, Select};
use crate::edit::{EditEvent, EditResponse};
use crate::localized::{Labels as _, LocaleNumbers, SharedLabels};
use crate::primitives::{Content, Interactive, StateColors, SurfaceKind};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};

crate::words! {
    /// The words of the size dialogs, English by default.
    #[non_exhaustive]
    pub struct SizingLabels {
        image_size_title: "sizing.imageSizeTitle" = "Image Size";
        canvas_size_title: "sizing.canvasSizeTitle" = "Canvas Size";
        image_size: "sizing.imageSize" = "Image Size:";
        /// `{size}` is the new file size, `{old}` the one the dialog opened with.
        size_was: "sizing.sizeWas" = "{size} (was {old})";
        dimensions: "sizing.dimensions" = "Dimensions:";
        /// `{width}` and `{height}` are whole pixels.
        dimensions_value: "sizing.dimensionsValue" = "{width} px \u{d7} {height} px";
        /// `{n}` is a whole pixel count.
        pixels_value: "sizing.pixelsValue" = "{n} px";
        current_size: "sizing.currentSize" = "Current Size:";
        new_size: "sizing.newSize" = "New Size:";
        width: "sizing.width" = "Width:";
        height: "sizing.height" = "Height:";
        /// The readouts of the size the canvas has, apart from the Width and Height fields of the new size.
        current_width: "sizing.currentWidth" = "Current width:";
        current_height: "sizing.currentHeight" = "Current height:";
        constrain: "sizing.constrain" = "Constrain proportions";
        width_unit: "sizing.widthUnit" = "Width unit";
        height_unit: "sizing.heightUnit" = "Height unit";
        resolution: "sizing.resolution" = "Resolution:";
        resolution_unit: "sizing.resolutionUnit" = "Resolution unit";
        pixels_per_inch: "sizing.pixelsPerInch" = "Pixels/Inch";
        pixels_per_centimetre: "sizing.pixelsPerCentimetre" = "Pixels/Centimeter";
        resample: "sizing.resample" = "Resample";
        resample_method: "sizing.resampleMethod" = "Resample method";
        scale_styles: "sizing.scaleStyles" = "Scale Styles";
        relative: "sizing.relative" = "Relative";
        anchor: "sizing.anchor" = "Anchor:";
        anchor_name: "sizing.anchorName" = "Anchor";
        extension: "sizing.extension" = "Canvas extension color:";
        pixels: "sizing.unitPixels" = "Pixels";
        percent: "sizing.unitPercent" = "Percent";
        inches: "sizing.unitInches" = "Inches";
        centimetres: "sizing.unitCentimetres" = "Centimeters";
        millimetres: "sizing.unitMillimetres" = "Millimeters";
        points: "sizing.unitPoints" = "Points";
        picas: "sizing.unitPicas" = "Picas";
        automatic: "sizing.automatic" = "Automatic";
        preserve_details: "sizing.preserveDetails" = "Preserve Details";
        bicubic_smoother: "sizing.bicubicSmoother" = "Bicubic Smoother (enlargement)";
        bicubic_sharper: "sizing.bicubicSharper" = "Bicubic Sharper (reduction)";
        bicubic: "sizing.bicubic" = "Bicubic (smooth gradients)";
        nearest_neighbor: "sizing.nearestNeighbor" = "Nearest Neighbor (hard edges)";
        bilinear: "sizing.bilinear" = "Bilinear";
        lanczos: "sizing.lanczos" = "Lanczos (sharp)";
        foreground: "sizing.foreground" = "Foreground";
        background: "sizing.background" = "Background";
        white: "sizing.white" = "White";
        black: "sizing.black" = "Black";
        gray: "sizing.gray" = "Gray";
        other: "sizing.other" = "Other";
        transparent: "sizing.transparent" = "Transparent";
        top_left: "sizing.anchorTopLeft" = "Top left";
        top: "sizing.anchorTop" = "Top";
        top_right: "sizing.anchorTopRight" = "Top right";
        left: "sizing.anchorLeft" = "Left";
        center: "sizing.anchorCenter" = "Center";
        right: "sizing.anchorRight" = "Right";
        bottom_left: "sizing.anchorBottomLeft" = "Bottom left";
        bottom: "sizing.anchorBottom" = "Bottom";
        bottom_right: "sizing.anchorBottomRight" = "Bottom right";
    }
}

impl SizingLabels {
    fn unit(&self, unit: SizeUnit) -> &str {
        match unit {
            SizeUnit::Pixels => &self.pixels,
            SizeUnit::Percent => &self.percent,
            SizeUnit::Inches => &self.inches,
            SizeUnit::Centimetres => &self.centimetres,
            SizeUnit::Millimetres => &self.millimetres,
            SizeUnit::Points => &self.points,
            SizeUnit::Picas => &self.picas,
        }
    }

    fn method(&self, method: Resample) -> &str {
        match method {
            Resample::None | Resample::Automatic => &self.automatic,
            Resample::PreserveDetails => &self.preserve_details,
            Resample::BicubicSmoother => &self.bicubic_smoother,
            Resample::BicubicSharper => &self.bicubic_sharper,
            Resample::Bicubic => &self.bicubic,
            Resample::NearestNeighbor => &self.nearest_neighbor,
            Resample::Bilinear => &self.bilinear,
            Resample::Lanczos => &self.lanczos,
        }
    }

    fn anchor_cell(&self, anchor: Anchor) -> &str {
        match anchor {
            Anchor::TopLeft => &self.top_left,
            Anchor::Top => &self.top,
            Anchor::TopRight => &self.top_right,
            Anchor::Left => &self.left,
            Anchor::Center => &self.center,
            Anchor::Right => &self.right,
            Anchor::BottomLeft => &self.bottom_left,
            Anchor::Bottom => &self.bottom,
            Anchor::BottomRight => &self.bottom_right,
        }
    }
}

/// The decimals a typed size or resolution keeps; the fields show fewer.
const COMMIT_DECIMALS: usize = 8;
/// The engine's greatest document side, in pixels.
const MAX_PIXELS: f64 = 300_000.0;
/// The least and greatest resolution, in pixels per inch.
const PPI_RANGE: (f64, f64) = (1.0, 30_000.0);
use crate::localized::units::{CM_PER_INCH, POINTS_PER_INCH};

/// The length unit of a size field: [`LengthUnit`](crate::localized::units::LengthUnit), under the name Image Size and Canvas Size use.
pub use crate::localized::units::LengthUnit as SizeUnit;

/// Resolution units of Image Size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResolutionUnit {
    #[allow(missing_docs)]
    PixelsPerInch,
    #[allow(missing_docs)]
    PixelsPerCentimetre,
}

impl ResolutionUnit {
    /// Pixels per inch in one of this unit.
    fn per_inch(self) -> f64 {
        match self {
            ResolutionUnit::PixelsPerInch => 1.0,
            ResolutionUnit::PixelsPerCentimetre => CM_PER_INCH,
        }
    }
}

/// Photoshop's Resample list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Resample {
    /// Resample off: the pixel count is fixed, a physical size changes the resolution.
    None,
    /// Lets the app pick the filter for enlarging or reducing.
    Automatic,
    #[allow(missing_docs)]
    PreserveDetails,
    #[allow(missing_docs)]
    Bicubic,
    #[allow(missing_docs)]
    BicubicSmoother,
    #[allow(missing_docs)]
    BicubicSharper,
    #[allow(missing_docs)]
    Bilinear,
    #[allow(missing_docs)]
    NearestNeighbor,
    /// Not in Photoshop's list; PhotoCraft offers it last.
    Lanczos,
}

/// A cell of the 3×3 anchor: which edge or corner of the current canvas stays put.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Anchor {
    #[allow(missing_docs)]
    TopLeft,
    #[allow(missing_docs)]
    Top,
    #[allow(missing_docs)]
    TopRight,
    #[allow(missing_docs)]
    Left,
    #[default]
    #[allow(missing_docs)]
    Center,
    #[allow(missing_docs)]
    Right,
    #[allow(missing_docs)]
    BottomLeft,
    #[allow(missing_docs)]
    Bottom,
    #[allow(missing_docs)]
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
    /// The app's foreground colour.
    Foreground,
    /// The app's background colour.
    Background,
    #[allow(missing_docs)]
    White,
    #[allow(missing_docs)]
    Black,
    #[allow(missing_docs)]
    Gray,
    /// Picked in the colour picker, sRGB components 0..=1.
    Other([f32; 3]),
    /// Transparent, for a document without a Background layer (the list is disabled then).
    Transparent,
}

/// The sizes the host hands a size dialog and gets back.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PixelSize {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Resolution in pixels per inch.
    pub pixels_per_inch: f64,
}

/// What the user chose in Image Size.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageSizeResult {
    /// The new size.
    pub size: PixelSize,
    /// The unit of the Width and Height fields when OK was pressed.
    pub unit: SizeUnit,
    /// The unit of the Resolution field when OK was pressed.
    pub resolution_unit: ResolutionUnit,
    /// The resampling filter.
    pub resample: Resample,
    /// Scale Styles: layer effects scale with the image (enabled while proportions are
    /// constrained).
    pub scale_styles: bool,
}

/// What the user chose in Canvas Size.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasSizeResult {
    /// The new canvas size.
    pub size: PixelSize,
    /// The unit of the Width and Height fields when OK was pressed.
    pub unit: SizeUnit,
    /// Whether the fields showed the change in size (the Relative box) when OK was pressed.
    pub relative: bool,
    /// Which part of the current canvas stays put.
    pub anchor: Anchor,
    /// What fills the added area.
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
    /// The unit the width and height fields show.
    pub unit: SizeUnit,
    /// The unit the resolution field shows.
    pub resolution_unit: ResolutionUnit,
    /// Keeps the aspect ratio when one side changes.
    pub constrain: bool,
    /// Whether the pixel count may change (Image Size with Resample on; always in Canvas Size).
    pub resampling: bool,
    /// A relative size (Canvas Size's Relative box): the fields show the change, not the size.
    pub relative: bool,
    /// Whether the rows draw the chain bracket that links width and height. Off, the sides never
    /// scale together (Canvas Size).
    pub chain: bool,
    /// Points in an inch for the Points and Picas units: 72, or the Point/Pica Size preference
    /// (72.27 for traditional points).
    pub points_per_inch: f64,
    /// The last typed width or height at full precision, shown back while the pixel size it made
    /// stands (a pixel count is whole, so 72.27 pt at 72 ppi is 72 px).
    pub(super) typed: Option<Typed>,
}

/// A value typed in a width or height field, with the state it produced.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Typed {
    field: Field,
    value: f64,
    unit: SizeUnit,
    relative: bool,
    size: PixelSize,
}

/// Which of the fields an edit came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Field {
    Width,
    Height,
    Resolution,
}

impl LinkedSizeFields {
    /// Fields for a document of size `original`, in pixels, constrained and resampling.
    pub fn new(original: PixelSize) -> Self {
        Self {
            original,
            current: original,
            unit: SizeUnit::Pixels,
            resolution_unit: ResolutionUnit::PixelsPerInch,
            constrain: true,
            resampling: true,
            relative: false,
            chain: true,
            points_per_inch: POINTS_PER_INCH,
            typed: None,
        }
    }

    /// The points in an inch of the Points and Picas units (PhotoCraft's Point/Pica Size
    /// preference). A value that is not positive keeps 72.
    pub fn points_per_inch(mut self, points_per_inch: f64) -> Self {
        if points_per_inch.is_finite() && points_per_inch > 0.0 {
            self.points_per_inch = points_per_inch;
        }
        self
    }

    /// The pixels of the width or the height of a size.
    fn side(size: PixelSize, field: Field) -> f64 {
        f64::from(if field == Field::Width { size.width } else { size.height })
    }

    /// The field's value in the current unit (the change in size, in a relative size).
    pub(super) fn shown(&self, field: Field) -> f64 {
        if let Some(typed) = &self.typed
            && typed.field == field
            && typed.unit == self.unit
            && typed.relative == self.relative
            && typed.size == self.current
        {
            return typed.value;
        }
        let (current, original) = (Self::side(self.current, field), Self::side(self.original, field));
        let pixels = if self.relative { current - original } else { current };
        self.unit.from_pixels(pixels, original, self.current.pixels_per_inch, self.points_per_inch)
    }

    /// Applies a value typed or stepped in a field, in the current unit.
    ///
    /// With Resample off the pixel count is fixed: a size in pixels or percent cannot change, and a
    /// physical size changes the resolution instead.
    pub(super) fn edit(&mut self, field: Field, value: f64) {
        if !value.is_finite() {
            return;
        }
        if field == Field::Resolution {
            self.set_resolution(value * self.resolution_unit.per_inch());
            return;
        }
        let (current, original) = (Self::side(self.current, field), Self::side(self.original, field));
        let ppi = self.current.pixels_per_inch;
        let pixels = self.unit.to_pixels(value, original, ppi, self.points_per_inch);
        self.typed = None;
        if !self.resampling {
            if !self.unit.is_pixel_unit() && pixels > 0.0 {
                self.current.pixels_per_inch = (ppi * current / pixels).clamp(PPI_RANGE.0, PPI_RANGE.1);
                self.current.width = self.original.width;
                self.current.height = self.original.height;
                self.remember_typed(field, value);
            }
            return;
        }
        let pixels = if self.relative { original + pixels } else { pixels };
        if field == Field::Width {
            self.set_width(pixels);
        } else {
            self.set_height(pixels);
        }
        self.remember_typed(field, value);
    }

    fn remember_typed(&mut self, field: Field, value: f64) {
        self.typed = Some(Typed { field, value, unit: self.unit, relative: self.relative, size: self.current });
    }

    /// The width in the current unit (the change in width, in a relative size), as the field
    /// shows it before rounding for display.
    pub fn width(&self) -> f64 {
        self.shown(Field::Width)
    }

    /// The height in the current unit; see [`width`](LinkedSizeFields::width).
    pub fn height(&self) -> f64 {
        self.shown(Field::Height)
    }

    /// The resolution in the current resolution unit, unrounded.
    pub fn resolution_value(&self) -> f64 {
        self.current.pixels_per_inch / self.resolution_unit.per_inch()
    }

    /// Sets the width as typing `value` in its field does, in the current unit.
    pub fn set_width_value(&mut self, value: f64) {
        self.edit(Field::Width, value);
    }

    /// Sets the height as typing `value` in its field does, in the current unit.
    pub fn set_height_value(&mut self, value: f64) {
        self.edit(Field::Height, value);
    }

    /// Sets the resolution as typing `value` in its field does, in the current resolution unit.
    pub fn set_resolution_value(&mut self, value: f64) {
        self.edit(Field::Resolution, value);
    }

    /// Sets the resolution in pixels per inch. With Resample on and a physical unit the print
    /// size stays and the pixels follow the resolution; otherwise the pixel count stays.
    pub(super) fn set_resolution(&mut self, pixels_per_inch: f64) {
        if !pixels_per_inch.is_finite() {
            return;
        }
        let ppi = pixels_per_inch.clamp(PPI_RANGE.0, PPI_RANGE.1);
        let old = self.current.pixels_per_inch;
        if self.resampling && !self.unit.is_pixel_unit() && old > 0.0 {
            self.current.width = clamp_pixels(f64::from(self.current.width) * ppi / old, self.current.width);
            self.current.height = clamp_pixels(f64::from(self.current.height) * ppi / old, self.current.height);
        }
        self.current.pixels_per_inch = ppi;
    }

    /// Draws the width, height and (when `with_resolution`) resolution rows into `form`,
    /// returning the edit of the pixel size this pass.
    ///
    /// `labels` are the dialog's words (`SharedLabels::shared_ctx`), taken by the caller because a
    /// row's label is needed before its control is drawn.
    // pc: crates/ui-egui/src/dialogs/sizing.rs:177 `dim_field`, :203 `unit_dropdown`, and the
    // rows of :211 `image_size`.
    pub fn rows(&mut self, form: &mut Form<'_>, labels: &SizingLabels, with_resolution: bool) -> EditResponse<PixelSize> {
        let mut event = None;
        let mut responses = Vec::new();
        // Without resampling the sides scale together: Photoshop keeps the link on.
        let mut linked = !self.resampling || self.constrain;
        let editable = self.resampling || !self.unit.is_pixel_unit();
        let chain = self.chain;
        let mut size_rows = |form: &mut Form<'_>| {
            for (field, label, unit_name) in [(Field::Width, &labels.width, &labels.width_unit), (Field::Height, &labels.height, &labels.height_unit)]
            {
                let row = form.row(label, |ui, row_label| self.dimension(ui, row_label, labels, field, unit_name, editable));
                responses.push(row.response);
                event = row.inner.1.or(event.take());
            }
        };
        let pair = if chain {
            let toggle = form.linked(&labels.constrain, &mut linked, &mut size_rows).response;
            if self.resampling {
                self.constrain = linked;
            }
            toggle
        } else {
            size_rows(form);
            responses.remove(0)
        };
        if with_resolution {
            let row = form.row(&labels.resolution, |ui, row_label| self.resolution(ui, row_label, labels));
            responses.push(row.response);
            event = row.inner.1.or(event.take());
        }
        let changed = event.is_some();
        let mut response = responses.into_iter().fold(pair, |all, r| all.union(r));
        if changed {
            response.mark_changed();
        }
        EditResponse::new(response, event)
    }

    /// One of the width and height rows' control: the number field and its unit menu.
    fn dimension(
        &mut self,
        ui: &mut Ui,
        row_label: &RowLabel<'_>,
        labels: &SizingLabels,
        field: Field,
        unit_name: &str,
        editable: bool,
    ) -> (Response, Option<EditEvent<PixelSize>>) {
        let numbers = LocaleNumbers::of(ui);
        let (id, name) = if field == Field::Width { ("width", &labels.width) } else { ("height", &labels.height) };
        let (unit, original, ppi) = (self.unit, Self::side(self.original, field), self.current.pixels_per_inch);
        let points = self.points_per_inch;
        let limit = unit.from_pixels(MAX_PIXELS, original, ppi, points);
        let least = if self.relative { -limit } else { unit.from_pixels(1.0, original, ppi, points).min(self.shown(field)) };
        let number = NumberField::new(id, name, self.shown(field), &numbers)
            .range(least..=limit)
            .decimals(unit.decimals()..=unit.decimals())
            .commit_decimals(COMMIT_DECIMALS)
            .or_parse(move |text| unit.parse_typed(text, original, ppi, points))
            .enabled(editable);
        let number = match row_label.scrub() {
            Some(label) => number.scrub_by(label),
            None => number,
        };
        let edit = number.show(ui);
        let event = self.apply(ui, field, edit.event);
        let mut picked = self.unit;
        let mut select = Select::new((id, "unit"), unit_name, &mut picked);
        for unit in SizeUnit::ALL {
            select = select.option(unit, labels.unit(unit));
        }
        let _ = select.show(ui);
        self.unit = picked;
        (edit.response, event)
    }

    /// The resolution row's control: the number field and its unit menu.
    fn resolution(&mut self, ui: &mut Ui, _row_label: &RowLabel<'_>, labels: &SizingLabels) -> (Response, Option<EditEvent<PixelSize>>) {
        let numbers = LocaleNumbers::of(ui);
        let per = self.resolution_unit.per_inch();
        let places = if self.resolution_unit == ResolutionUnit::PixelsPerInch { 0 } else { 2 };
        let number = NumberField::new("resolution", &labels.resolution, self.current.pixels_per_inch / per, &numbers)
            .range(PPI_RANGE.0 / per..=PPI_RANGE.1 / per)
            .decimals(places..=places)
            .commit_decimals(COMMIT_DECIMALS);
        let edit = number.show(ui);
        let event = self.apply(ui, Field::Resolution, edit.event);
        let mut picked = self.resolution_unit;
        let _ = Select::new("resolution-unit", &labels.resolution_unit, &mut picked)
            .option(ResolutionUnit::PixelsPerInch, &labels.pixels_per_inch)
            .option(ResolutionUnit::PixelsPerCentimetre, &labels.pixels_per_centimetre)
            .show(ui);
        self.resolution_unit = picked;
        (edit.response, event)
    }

    /// Applies a field's edit event and reports it with the pixel size it left. A cancelled
    /// gesture puts back the size from before it began.
    fn apply(&mut self, ui: &Ui, field: Field, event: Option<EditEvent<f64>>) -> Option<EditEvent<PixelSize>> {
        let event = event?;
        let before = ui.make_persistent_id(("size-before", field as u8));
        match &event {
            EditEvent::Begin { .. } => {
                let current = self.current;
                ui.data_mut(|d| d.insert_temp(before, current));
            }
            EditEvent::Cancel { .. } => {
                if let Some(size) = ui.data(|d| d.get_temp::<PixelSize>(before)) {
                    self.current = size;
                }
            }
            _ => {}
        }
        if let Some(value) = event.value() {
            self.edit(field, *value);
        }
        let current = self.current;
        Some(event.map(|_| current))
    }

    /// Sets the width in pixels, scaling the height when constrained.
    // pc: crates/ui-egui/src/dialogs/sizing.rs:211 `image_size` (the `w_row` branch).
    pub fn set_width(&mut self, pixels: f64) {
        self.current.width = clamp_pixels(pixels, self.current.width);
        if self.constrain {
            let scaled = f64::from(self.current.width) * f64::from(self.original.height) / f64::from(self.original.width.max(1));
            self.current.height = clamp_pixels(scaled, self.current.height);
        }
    }

    /// Sets the height in pixels, scaling the width when constrained.
    pub fn set_height(&mut self, pixels: f64) {
        self.current.height = clamp_pixels(pixels, self.current.height);
        if self.constrain {
            let scaled = f64::from(self.current.height) * f64::from(self.original.width) / f64::from(self.original.height.max(1));
            self.current.width = clamp_pixels(scaled, self.current.width);
        }
    }
}

/// Whole pixels from `pixels`, at least one and at most the engine's limit; `fallback` for a
/// value that is not a number.
fn clamp_pixels(pixels: f64, fallback: u32) -> u32 {
    if pixels.is_finite() { pixels.round().clamp(1.0, MAX_PIXELS) as u32 } else { fallback }
}

/// The direction of the arrow `cell` shows when the anchor is `anchor`, in cell steps (each -1, 0
/// or 1): away from the anchor, towards where the canvas grows, and towards it where the canvas
/// shrinks. Only the cells next to the anchor have one (Photoshop).
pub(super) fn arrow_direction(anchor: Anchor, cell: Anchor, delta: (f64, f64)) -> Option<(i32, i32)> {
    let ((ax, ay), (x, y)) = (anchor.cell(), cell.cell());
    let (dx, dy) = (x as i32 - ax as i32, y as i32 - ay as i32);
    if (dx, dy) == (0, 0) || dx.abs() > 1 || dy.abs() > 1 {
        return None;
    }
    let flip = |change: f64| if change < 0.0 { -1 } else { 1 };
    Some((dx * flip(delta.0), dy * flip(delta.1)))
}

/// The 3×3 anchor of Canvas Size: nine cells, the chosen one marked, arrows in the neighbouring
/// cells pointing where the canvas grows (they flip when it shrinks). Click picks; with the
/// focus, the arrow keys move the mark and Space or Enter picks (gap 10).
pub struct AnchorGrid<'a> {
    anchor: &'a mut Anchor,
    /// The change in width and height in pixels; the arrows' direction.
    delta: (f64, f64),
    arrows: bool,
}

impl<'a> AnchorGrid<'a> {
    /// A grid editing `anchor`; `delta` is the width and height change, which decides the direction of the arrows drawn.
    pub fn new(anchor: &'a mut Anchor, delta: (f64, f64)) -> Self {
        Self { anchor, delta, arrows: true }
    }

    /// Whether the neighbouring cells show the growth arrows; on by default. A grid that only
    /// picks a position (the export watermark's) turns them off.
    pub fn arrows(mut self, arrows: bool) -> Self {
        self.arrows = arrows;
        self
    }

    /// Draws the grid. `changed()` is set on the pass the anchor changes.
    ///
    /// The cells are the radio buttons of one group: every cell is a Tab stop, the arrow keys move
    /// the focus between them in two dimensions and Space or Enter picks the focused cell.
    // pc: crates/ui-egui/src/dialogs/sizing.rs:416 `anchor_grid` (24 pt cells, the arrow
    // geometry), with the library's field fill and focus ring; the keyboard part is new.
    pub fn show(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let labels = SizingLabels::shared(ui);
        let mark_radius = crate::style::dialog::DialogStyle::new(&tokens).anchor_mark_radius;
        let side = tokens.size.control.sm;
        let scale = side / PC_ANCHOR_CELL;
        let (_, grid) = ui.allocate_space(vec2(side * 3.0, side * 3.0));
        let group = ui.new_child(
            UiBuilder::new()
                .id_salt("anchor-grid")
                .max_rect(grid)
                .accessibility_role(Role::RadioGroup)
                .accessibility_label(labels.anchor_name.as_str()),
        );
        let ids = (0..Anchor::ALL.len()).map(|i| (Interactive::id_in(&group, ("anchor", i)), true)).collect();
        let mut radios = RadioSelection::new(ids, Anchor::ALL.iter().position(|a| a == self.anchor));
        radios.report_size(ui, group.unique_id());
        let selected_fill = StateColors { fill: Segmented::style(&tokens, ControlSize::Md).selected_fill, border: None };
        let sensed: Vec<_> = Anchor::ALL
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let (column, row) = cell.cell();
                let rect = Rect::from_min_size(grid.min + vec2(column as f32 * side, row as f32 * side), vec2(side, side))
                    .shrink(tokens.stroke.thin);
                Interactive::new(("anchor", i), Role::RadioButton, labels.anchor_cell(*cell))
                    .position_in_set(i)
                    .surface(SurfaceKind::Sunken)
                    .on_colors(selected_fill)
                    .interact_at(&group, rect)
            })
            .collect();
        radios.sensed(&sensed);
        let selected = radios.selected();
        if let Some(i) = selected.filter(|_| radios.changed()) {
            *self.anchor = Anchor::ALL[i];
        }

        let (mark, arrow) = (tokens.color.text.normal.to_egui(), tokens.color.text.muted.to_egui());
        let mut responses = Vec::with_capacity(sensed.len());
        for (i, sensed) in sensed.into_iter().enumerate() {
            let response = sensed.checked(selected == Some(i)).finish(&group);
            if ui.is_rect_visible(response.rect) {
                let center = response.rect.center();
                if selected == Some(i) {
                    let half = 4.0 * scale;
                    ui.painter().rect_filled(Rect::from_center_size(center, vec2(2.0 * half, 2.0 * half)), mark_radius, mark);
                } else if self.arrows
                    && let Some((dx, dy)) = arrow_direction(*self.anchor, Anchor::ALL[i], self.delta)
                {
                    ui.painter().extend(arrow_shapes(center, vec2(dx as f32, dy as f32).normalized(), scale, arrow));
                }
            }
            responses.push(response);
        }
        let mut response = responses.into_iter().reduce(|all, r| all.union(r)).map_or_else(|| ui.response(), |all| all.with_new_rect(grid));
        if radios.changed() {
            response.mark_changed();
        }
        response
    }
}

/// PhotoCraft's anchor cell, in points: the arrow geometry below is drawn for it.
const PC_ANCHOR_CELL: f32 = 24.0;

/// A line with a head, `direction` unit length, centred on `center`.
fn arrow_shapes(center: Pos2, direction: Vec2, scale: f32, color: Color32) -> [Shape; 2] {
    let tip = center + direction * 6.0 * scale;
    let tail = center - direction * 5.0 * scale;
    let side = vec2(-direction.y, direction.x) * 3.5 * scale;
    let base = tip - direction * 3.5 * scale;
    [
        Shape::line_segment([tail, tip], Stroke::new(1.3, color)),
        Shape::convex_polygon(vec![tip + direction * 1.5 * scale, base + side, base - side], color, Stroke::NONE),
    ]
}

/// Byte counts as the dialogs show them (`1.2M`, `850K`).
// pc: crates/ui-egui/src/dialogs/sizing.rs:144 `human_bytes`.
pub fn human_bytes(bytes: f64) -> String {
    const KIB: f64 = 1024.0;
    if bytes >= KIB * KIB * KIB {
        format!("{:.2}G", bytes / (KIB * KIB * KIB))
    } else if bytes >= KIB * KIB {
        format!("{:.1}M", bytes / (KIB * KIB))
    } else {
        format!("{:.0}K", (bytes / KIB).max(1.0))
    }
}

/// The file size of a document of `size` at `bytes_per_pixel`, as the readouts show it.
/// A readout row whose text is set after the rows below it have taken this frame's edits.
struct Readout(Rect, bool);

impl Readout {
    /// The right end (the end of the reading direction) of `row`, a header's row.
    fn trailing(row: Rect) -> Self {
        Self(row, true)
    }

    /// Takes the row's place in the grid, with the label and no text.
    fn reserve(form: &mut Form<'_>, label: &str) -> Self {
        Self(form.value(label, "").rect, false)
    }

    /// Draws `text` in the reserved row. `ui` is the parent of the grid.
    fn put(&self, ui: &mut Ui, text: String) {
        let tokens = Tokens::of(ui);
        let format = role_format(&tokens, craft_theme::TypeRole::Body, Content::Normal.color(&tokens).to_egui());
        let job = LayoutJob::single_section(text, format);
        let (rect, layout) = if self.1 {
            (self.0, crate::direction::trailing(ui))
        } else {
            (Rect::from_min_max(self.0.min, Pos2::new(ui.max_rect().right().max(self.0.min.x), self.0.max.y)), Layout::left_to_right(Align::Center))
        };
        ui.scope_builder(UiBuilder::new().max_rect(rect).layout(layout), |ui| {
            ui.add(Label::new(job).selectable(false));
        });
    }
}

fn file_size(size: PixelSize, bytes_per_pixel: f64) -> String {
    human_bytes(f64::from(size.width) * f64::from(size.height) * bytes_per_pixel)
}

/// Image › Image Size: the image's file size readout and dimensions, the
/// [`LinkedSizeFields`], Resample with its list, Scale Styles; OK, Cancel, Reset on Alt
/// (spec question 4). Fit To presets and the preview pane of Photoshop CC are not part of it.
///
/// The user sees the readouts at the top ("Image Size: 2.3M (was 1.1M)", "Dimensions: 1200 px ×
/// 800 px"), the linked fields, the Resample check box and its method list, and the footer.
/// With Resample off the pixel fields are disabled and a physical size changes the resolution.
/// Result: [`ImageSizeResult`].
pub struct ImageSizeDialog {
    id: DialogId,
    fields: LinkedSizeFields,
    resample: Resample,
    /// The method Resample returns to when the box is checked again.
    method: Resample,
    scale_styles: bool,
    bytes_per_pixel: f64,
    methods: Vec<Resample>,
    labels: Option<SizingLabels>,
    state: DialogState<(LinkedSizeFields, Resample, bool)>,
}

impl ImageSizeDialog {
    /// An Image Size dialog for a document of size `original`.
    pub fn new(original: PixelSize) -> Self {
        Self {
            id: dialog_id("image-size"),
            fields: LinkedSizeFields::new(original),
            resample: Resample::Automatic,
            method: Resample::Automatic,
            scale_styles: true,
            bytes_per_pixel: 4.0,
            methods: RESAMPLE_METHODS.to_vec(),
            labels: None,
            state: DialogState::new(),
        }
    }

    /// The words of the dialog, in place of the installed ones (an app's catalog words).
    pub fn labels(mut self, labels: SizingLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The unit the Width and Height fields open in (an app's Units & Rulers preference);
    /// Pixels by default. The chosen unit comes back in [`ImageSizeResult::unit`].
    pub fn unit(mut self, unit: SizeUnit) -> Self {
        self.fields.unit = unit;
        self
    }

    /// The unit the Resolution field opens in; Pixels/Inch by default.
    pub fn resolution_unit(mut self, unit: ResolutionUnit) -> Self {
        self.fields.resolution_unit = unit;
        self
    }

    /// Limits the Resample list to `methods` (PhotoCraft's Indexed Color offers Automatic and
    /// Nearest Neighbor), listed in the dialog's usual order. [`Resample::None`] is not a list
    /// entry, and a list with no entry the dialog knows leaves the full list. A method the
    /// dialog opens with that is no longer listed becomes the first listed.
    pub fn resample_methods(mut self, methods: &[Resample]) -> Self {
        let kept: Vec<Resample> = RESAMPLE_METHODS.into_iter().filter(|m| methods.contains(m)).collect();
        if !kept.is_empty() {
            self.methods = kept;
        }
        self.keep_method_listed();
        self
    }

    fn keep_method_listed(&mut self) {
        if !self.methods.contains(&self.method) {
            self.method = self.methods[0];
            if self.resample != Resample::None {
                self.resample = self.method;
            }
        }
    }

    /// Sets the width as typing it in the field does, in the current unit.
    pub fn set_width(&mut self, value: f64) {
        self.fields.set_width_value(value);
    }

    /// Sets the height as typing it in the field does, in the current unit.
    pub fn set_height(&mut self, value: f64) {
        self.fields.set_height_value(value);
    }

    /// Sets the resolution as typing it in the field does, in the resolution unit.
    pub fn set_resolution(&mut self, value: f64) {
        self.fields.set_resolution_value(value);
    }

    /// Switches the unit of the Width and Height fields; the size stays.
    pub fn set_unit(&mut self, unit: SizeUnit) {
        self.fields.unit = unit;
    }

    /// Switches the unit of the Resolution field; the resolution stays.
    pub fn set_resolution_unit(&mut self, unit: ResolutionUnit) {
        self.fields.resolution_unit = unit;
    }

    /// Chooses the Resample method as the list does, or [`Resample::None`] to uncheck Resample
    /// (the size returns to the opening pixel count). A method the list does not offer
    /// ([`resample_methods`](ImageSizeDialog::resample_methods)) is ignored.
    pub fn set_resample(&mut self, resample: Resample) {
        if resample == Resample::None {
            if self.fields.resampling {
                self.method = self.resample;
                self.resample = Resample::None;
                self.fields.resampling = false;
                self.fields.current.width = self.fields.original.width;
                self.fields.current.height = self.fields.original.height;
            }
        } else if self.methods.contains(&resample) {
            self.method = resample;
            self.resample = resample;
            self.fields.resampling = true;
        }
    }

    /// Sets the Scale Styles box.
    pub fn set_scale_styles(&mut self, on: bool) {
        self.scale_styles = on;
    }

    /// The Resample method now chosen; [`Resample::None`] while Resample is unchecked.
    pub fn chosen_resample(&self) -> Resample {
        self.resample
    }

    /// Whether the Scale Styles box is checked.
    pub fn chosen_scale_styles(&self) -> bool {
        self.scale_styles
    }

    /// The answer the dialog gives now.
    fn answer(&self) -> ImageSizeResult {
        ImageSizeResult {
            size: self.fields.current,
            unit: self.fields.unit,
            resolution_unit: self.fields.resolution_unit,
            resample: self.resample,
            scale_styles: self.scale_styles,
        }
    }

    /// Bytes per pixel of the document, for the file size readout (4 for 8-bit RGBA).
    pub fn bytes_per_pixel(mut self, bytes: f64) -> Self {
        self.bytes_per_pixel = bytes;
        self
    }

    /// The method the dialog opens with; [`Resample::None`] opens it with Resample off.
    pub fn resample(mut self, resample: Resample) -> Self {
        self.resample = resample;
        if resample != Resample::None {
            self.method = resample;
        }
        self.fields.resampling = resample != Resample::None;
        self.keep_method_listed();
        self
    }

    /// The points in an inch of the Points and Picas units; see [`LinkedSizeFields::points_per_inch`].
    pub fn points_per_inch(mut self, points_per_inch: f64) -> Self {
        self.fields = self.fields.points_per_inch(points_per_inch);
        self
    }

    /// The linked fields.
    pub fn fields(&self) -> &LinkedSizeFields {
        &self.fields
    }

    /// The linked fields, for a host that sets the size in pixels
    /// ([`LinkedSizeFields::set_width`]) whatever unit the dialog shows.
    pub fn fields_mut(&mut self) -> &mut LinkedSizeFields {
        &mut self.fields
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
        let labels = self.labels.clone().map_or_else(|| SizingLabels::shared_ctx(ctx), std::sync::Arc::new);
        let gaps = crate::theme::current(ctx).space.gap;
        let reset = DialogLabels::of_ctx(ctx).reset;
        let mut fields = (self.fields.clone(), self.resample, self.scale_styles);
        let (bytes_per_pixel, method, methods) = (self.bytes_per_pixel, &mut self.method, &self.methods);
        let shown = Dialog::new("image-size", labels.image_size_title.as_str())
            .stack(stack)
            .footer(Footer::standard().reset(Reset::AltCancel(reset)))
            .show(ctx, &mut self.state, &mut fields, |ui, (fields, resample, scale_styles)| {
                let readouts = FormGrid::new("image-size").show(ui, |form| {
                    let size_row = Readout::reserve(form, &labels.image_size);
                    let dimensions_row = Readout::reserve(form, &labels.dimensions);
                    form.space(gaps.stack);
                    let _ = fields.rows(form, &labels, true);
                    form.space(gaps.row);
                    form.unlabelled(|ui| {
                        let mut on = fields.resampling;
                        let check = Checkbox::bool(&labels.resample, &mut on).show(ui);
                        if on != fields.resampling {
                            fields.resampling = on;
                            if on {
                                *resample = *method;
                            } else {
                                *method = *resample;
                                *resample = Resample::None;
                                fields.current.width = fields.original.width;
                                fields.current.height = fields.original.height;
                            }
                        }
                        if on {
                            let mut picked = *resample;
                            let mut select = Select::new("resample-method", &labels.resample_method, &mut picked);
                            for option in methods.iter().copied() {
                                select = select.option(option, labels.method(option));
                            }
                            let _ = select.show(ui);
                            *resample = picked;
                        }
                        check
                    });
                    form.unlabelled(|ui| Checkbox::bool(&labels.scale_styles, scale_styles).enabled(fields.constrain || !fields.resampling).show(ui));
                    (size_row, dimensions_row)
                });
                let (size_row, dimensions_row) = readouts.inner;
                let (original, current) = (fields.original, fields.current);
                let was = file_size(original, bytes_per_pixel);
                size_row.put(ui, craft_i18n::fmt(&labels.size_was, &[("size", &file_size(current, bytes_per_pixel)), ("old", &was)]));
                let (width, height) = (current.width.to_string(), current.height.to_string());
                dimensions_row.put(ui, craft_i18n::fmt(&labels.dimensions_value, &[("width", &width), ("height", &height)]));
            });
        let ImageSizeDialog { fields: kept, resample, scale_styles, .. } = self;
        match shown.outcome {
            Some(Outcome::Ok) => {
                (*kept, *resample, *scale_styles) = fields;
                return Some(DialogResult::Ok(self.answer()));
            }
            Some(Outcome::Cancel | Outcome::Close) => return Some(DialogResult::Cancel),
            Some(Outcome::Reset) => {
                self.state.reset(&mut fields);
            }
            _ => {}
        }
        (*kept, *resample, *scale_styles) = fields;
        None
    }

    fn confirm(&mut self, _ctx: &Context) -> Option<ImageSizeResult> {
        Some(self.answer())
    }
}

/// Photoshop's Resample list, without Off, then Lanczos (PhotoCraft's order).
const RESAMPLE_METHODS: [Resample; 8] = [
    Resample::Automatic,
    Resample::PreserveDetails,
    Resample::BicubicSmoother,
    Resample::BicubicSharper,
    Resample::Bicubic,
    Resample::NearestNeighbor,
    Resample::Bilinear,
    Resample::Lanczos,
];

/// Image › Canvas Size: Current Size readouts, New Size with the [`LinkedSizeFields`], the
/// Relative box, the [`AnchorGrid`] and the Canvas extension colour list; OK, Cancel, Reset on
/// Alt (spec question 4). Result: [`CanvasSizeResult`].
///
/// The list offers Foreground, Background, White, Black and Gray; Other appears while the dialog
/// holds an [`CanvasExtension::Other`] colour the host set with
/// [`extension`](CanvasSizeDialog::extension), because the colour picker belongs to
/// `craft-color`, which depends on this crate.
pub struct CanvasSizeDialog {
    id: DialogId,
    fields: LinkedSizeFields,
    anchor: Anchor,
    extension: CanvasExtension,
    /// Whether the extension list is offered (a document with a Background layer).
    has_background: bool,
    bytes_per_pixel: f64,
    labels: Option<SizingLabels>,
    state: DialogState<(LinkedSizeFields, Anchor, CanvasExtension)>,
}

impl CanvasSizeDialog {
    /// A Canvas Size dialog for a document of size `original`; it has no chain and no constrain.
    pub fn new(original: PixelSize) -> Self {
        // Photoshop's Canvas Size has no linked proportions and no chain.
        let fields = LinkedSizeFields { constrain: false, chain: false, ..LinkedSizeFields::new(original) };
        Self {
            id: dialog_id("canvas-size"),
            fields,
            anchor: Anchor::Center,
            extension: CanvasExtension::Background,
            has_background: true,
            bytes_per_pixel: 4.0,
            labels: None,
            state: DialogState::new(),
        }
    }

    /// The words of the dialog, in place of the installed ones (an app's catalog words).
    pub fn labels(mut self, labels: SizingLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The unit the Width and Height fields open in (an app's Units & Rulers preference);
    /// Pixels by default. The chosen unit comes back in [`CanvasSizeResult::unit`].
    pub fn unit(mut self, unit: SizeUnit) -> Self {
        self.fields.unit = unit;
        self
    }

    /// Sets the width as typing it in the field does, in the current unit.
    pub fn set_width(&mut self, value: f64) {
        self.fields.set_width_value(value);
    }

    /// Sets the height as typing it in the field does, in the current unit.
    pub fn set_height(&mut self, value: f64) {
        self.fields.set_height_value(value);
    }

    /// Switches the unit of the Width and Height fields; the size stays.
    pub fn set_unit(&mut self, unit: SizeUnit) {
        self.fields.unit = unit;
    }

    /// Sets the Relative box: the fields show the change in size instead of the size.
    pub fn set_relative(&mut self, relative: bool) {
        self.fields.relative = relative;
    }

    /// Chooses the anchor cell.
    pub fn set_anchor(&mut self, anchor: Anchor) {
        self.anchor = anchor;
    }

    /// Chooses the extension colour.
    pub fn set_extension(&mut self, extension: CanvasExtension) {
        self.extension = extension;
    }

    /// The anchor cell now chosen.
    pub fn chosen_anchor(&self) -> Anchor {
        self.anchor
    }

    /// The extension colour the answer carries: transparent without a Background layer.
    pub fn chosen_extension(&self) -> CanvasExtension {
        if self.has_background { self.extension.clone() } else { CanvasExtension::Transparent }
    }

    /// Whether the Relative box is checked.
    pub fn chosen_relative(&self) -> bool {
        self.fields.relative
    }

    /// The answer the dialog gives now.
    fn answer(&self) -> CanvasSizeResult {
        CanvasSizeResult {
            size: self.fields.current,
            unit: self.fields.unit,
            relative: self.fields.relative,
            anchor: self.anchor,
            extension: self.chosen_extension(),
        }
    }

    /// Sets the bytes per pixel the file size readout assumes; 4 by default.
    pub fn bytes_per_pixel(mut self, bytes: f64) -> Self {
        self.bytes_per_pixel = bytes;
        self
    }

    /// Without a Background layer the extension is transparent and the list is disabled.
    pub fn has_background(mut self, has_background: bool) -> Self {
        self.has_background = has_background;
        self
    }

    /// The extension the list opens with.
    pub fn extension(mut self, extension: CanvasExtension) -> Self {
        self.extension = extension;
        self
    }

    /// The points in an inch of the Points and Picas units; see [`LinkedSizeFields::points_per_inch`].
    pub fn points_per_inch(mut self, points_per_inch: f64) -> Self {
        self.fields = self.fields.points_per_inch(points_per_inch);
        self
    }

    /// The linked fields.
    pub fn fields(&self) -> &LinkedSizeFields {
        &self.fields
    }

    /// The linked fields, for a host that sets the size in pixels
    /// ([`LinkedSizeFields::set_width`]) whatever unit the dialog shows.
    pub fn fields_mut(&mut self) -> &mut LinkedSizeFields {
        &mut self.fields
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
        let labels = self.labels.clone().map_or_else(|| SizingLabels::shared_ctx(ctx), std::sync::Arc::new);
        let gaps = crate::theme::current(ctx).space.gap;
        let reset = DialogLabels::of_ctx(ctx).reset;
        let mut fields = (self.fields.clone(), self.anchor, self.extension.clone());
        let (bytes_per_pixel, has_background) = (self.bytes_per_pixel, self.has_background);
        let shown = Dialog::new("canvas-size", labels.canvas_size_title.as_str())
            .stack(stack)
            .footer(Footer::standard().reset(Reset::AltCancel(reset)))
            .show(ctx, &mut self.state, &mut fields, |ui, (fields, anchor, extension)| {
                let new_size_row = FormGrid::new("canvas-size").show(ui, |form| {
                    let original = fields.original;
                    form.value(&labels.current_size, file_size(original, bytes_per_pixel));
                    form.value(&labels.current_width, craft_i18n::fmt(&labels.pixels_value, &[("n", &original.width.to_string())]));
                    form.value(&labels.current_height, craft_i18n::fmt(&labels.pixels_value, &[("n", &original.height.to_string())]));
                    let header = form.spanning(|ui| {
                        SectionHeader::new("new-size", labels.new_size.trim_end_matches([':', '\u{ff1a}']))
                            .collapsible(false)
                            .show(ui, |_| {})
                            .response
                    });
                    let new_size_row = Readout::trailing(header.inner.rect);
                    let _ = fields.rows(form, &labels, false);
                    let current = fields.current;
                    form.unlabelled(|ui| Checkbox::bool(&labels.relative, &mut fields.relative).show(ui));
                    let delta = (f64::from(current.width) - f64::from(original.width), f64::from(current.height) - f64::from(original.height));
                    form.row(&labels.anchor, |ui, _| AnchorGrid::new(anchor, delta).show(ui));
                    form.space(gaps.row);
                    form.row(&labels.extension, |ui, _| {
                        let mut picked = if has_background { extension.clone() } else { CanvasExtension::Transparent };
                        let other = matches!(picked, CanvasExtension::Other(_)).then(|| picked.clone());
                        let mut select = Select::new("canvas-extension", &labels.extension, &mut picked).enabled(has_background);
                        let list = [
                            (CanvasExtension::Foreground, &labels.foreground),
                            (CanvasExtension::Background, &labels.background),
                            (CanvasExtension::White, &labels.white),
                            (CanvasExtension::Black, &labels.black),
                            (CanvasExtension::Gray, &labels.gray),
                        ];
                        for (value, text) in list {
                            select = select.option(value, text);
                        }
                        if let Some(other) = other {
                            select = select.option(other, &labels.other);
                        }
                        if !has_background {
                            select = select.option(CanvasExtension::Transparent, &labels.transparent);
                        }
                        let edit = select.show(ui);
                        if has_background {
                            *extension = picked;
                        }
                        edit
                    });
                    new_size_row
                });
                new_size_row.inner.put(ui, file_size(fields.current, bytes_per_pixel));
            });
        let CanvasSizeDialog { fields: kept, anchor, extension, .. } = self;
        match shown.outcome {
            Some(Outcome::Ok) => {
                (*kept, *anchor, *extension) = fields;
                return Some(DialogResult::Ok(self.answer()));
            }
            Some(Outcome::Cancel | Outcome::Close) => return Some(DialogResult::Cancel),
            Some(Outcome::Reset) => {
                self.state.reset(&mut fields);
            }
            _ => {}
        }
        (*kept, *anchor, *extension) = fields;
        None
    }

    fn confirm(&mut self, _ctx: &Context) -> Option<CanvasSizeResult> {
        Some(self.answer())
    }
}
