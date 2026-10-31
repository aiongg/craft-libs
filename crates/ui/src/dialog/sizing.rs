//! [`ImageSizeDialog`], [`CanvasSizeDialog`], [`LinkedSizeFields`] and [`AnchorGrid`]: Image Size
//! and Canvas Size with units, file size and the 3×3 anchor. In `sizing`, not `size`: `size.rs`
//! holds the frame's own size rules ([`Width`](super::Width), [`Body`](super::Body)).

use egui::accesskit::Role;
use egui::{Color32, Context, Pos2, Rect, Response, Shape, Stroke, Ui, UiBuilder, Vec2, vec2};

use super::dialog::{Dialog, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::footer::{DialogLabels, Footer, Reset};
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::behavior::RadioSelection;
use crate::composites::{Form, FormGrid, RowLabel};
use crate::controls::{Checkbox, NumberField, Select};
use crate::edit::{EditEvent, EditResponse};
use crate::localized::{Labels as _, LocaleNumbers, SharedLabels};
use crate::primitives::{Divider, Interactive, StateColors, SurfaceKind};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};

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

/// The engine's greatest document side, in pixels.
const MAX_PIXELS: f64 = 300_000.0;
/// The least and greatest resolution, in pixels per inch.
const PPI_RANGE: (f64, f64) = (1.0, 30_000.0);
/// Points per inch: PostScript points, which a pica is 12 of.
const POINTS_PER_INCH: f64 = 72.0;
const CM_PER_INCH: f64 = 2.54;

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

    /// Decimal places a field in this unit shows.
    fn decimals(self) -> usize {
        match self {
            SizeUnit::Pixels => 0,
            SizeUnit::Points | SizeUnit::Percent => 1,
            SizeUnit::Millimetres | SizeUnit::Centimetres | SizeUnit::Picas => 2,
            SizeUnit::Inches => 3,
        }
    }

    /// One arrow-key step in this unit.
    fn step(self) -> f64 {
        match self {
            SizeUnit::Inches | SizeUnit::Centimetres | SizeUnit::Picas => 0.1,
            SizeUnit::Pixels | SizeUnit::Percent | SizeUnit::Millimetres | SizeUnit::Points => 1.0,
        }
    }

    /// The unit a typed suffix names (`px`, `in` or `"`, `cm`, `mm`, `pt`, `pc` or `pica`, `%` and
    /// their long forms), in any case.
    fn from_suffix(suffix: &str) -> Option<SizeUnit> {
        Some(match suffix.to_ascii_lowercase().as_str() {
            "px" | "pixel" | "pixels" => SizeUnit::Pixels,
            "in" | "inch" | "inches" | "\"" => SizeUnit::Inches,
            "cm" | "centimeter" | "centimeters" | "centimetre" | "centimetres" => SizeUnit::Centimetres,
            "mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => SizeUnit::Millimetres,
            "pt" | "point" | "points" => SizeUnit::Points,
            "pc" | "pica" | "picas" => SizeUnit::Picas,
            "%" | "percent" => SizeUnit::Percent,
            _ => return None,
        })
    }

    /// Text typed in a field of this unit, with an optional unit suffix of its own (`2in` in a
    /// pixel field), as the value in this unit.
    fn parse_typed(self, text: &str, original_pixels: f64, pixels_per_inch: f64) -> Option<f64> {
        let text = text.trim();
        let end = text.find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+'))).unwrap_or(text.len());
        let (number, suffix) = text.split_at(end);
        let value: f64 = number.trim().replace(',', ".").parse().ok().filter(|v: &f64| v.is_finite())?;
        let suffix = suffix.trim();
        if suffix.is_empty() {
            return Some(value);
        }
        let typed = SizeUnit::from_suffix(suffix)?;
        Some(self.from_pixels(typed.to_pixels(value, original_pixels, pixels_per_inch), original_pixels, pixels_per_inch))
    }

    /// `pixels` in this unit, given the original pixel size (for percent) and the resolution.
    // pc: crates/ui-egui/src/dialogs/sizing.rs:120 `to_unit`.
    pub fn from_pixels(self, pixels: f64, original_pixels: f64, pixels_per_inch: f64) -> f64 {
        let resolution = pixels_per_inch.max(1.0);
        match self {
            SizeUnit::Pixels => pixels,
            SizeUnit::Percent => pixels / original_pixels.max(1.0) * 100.0,
            SizeUnit::Inches => pixels / resolution,
            SizeUnit::Centimetres => pixels / resolution * CM_PER_INCH,
            SizeUnit::Millimetres => pixels / resolution * CM_PER_INCH * 10.0,
            SizeUnit::Points => pixels / resolution * POINTS_PER_INCH,
            SizeUnit::Picas => pixels / resolution * POINTS_PER_INCH / 12.0,
        }
    }

    /// A value in this unit as pixels.
    // pc: crates/ui-egui/src/dialogs/sizing.rs:132 `from_unit`.
    pub fn to_pixels(self, value: f64, original_pixels: f64, pixels_per_inch: f64) -> f64 {
        let resolution = pixels_per_inch.max(1.0);
        match self {
            SizeUnit::Pixels => value,
            SizeUnit::Percent => value * original_pixels / 100.0,
            SizeUnit::Inches => value * resolution,
            SizeUnit::Centimetres => value / CM_PER_INCH * resolution,
            SizeUnit::Millimetres => value / (CM_PER_INCH * 10.0) * resolution,
            SizeUnit::Points => value / POINTS_PER_INCH * resolution,
            SizeUnit::Picas => value / POINTS_PER_INCH * 12.0 * resolution,
        }
    }
}

/// Resolution units of Image Size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResolutionUnit {
    PixelsPerInch,
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

/// Which of the fields an edit came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Field {
    Width,
    Height,
    Resolution,
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

    /// The pixels of the width or the height of a size.
    fn side(size: PixelSize, field: Field) -> f64 {
        f64::from(if field == Field::Width { size.width } else { size.height })
    }

    /// The field's value in the current unit (the change in size, in a relative size).
    pub(super) fn shown(&self, field: Field) -> f64 {
        let (current, original) = (Self::side(self.current, field), Self::side(self.original, field));
        let pixels = if self.relative { current - original } else { current };
        self.unit.from_pixels(pixels, original, self.current.pixels_per_inch)
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
        let pixels = self.unit.to_pixels(value, original, ppi);
        if !self.resampling {
            if !self.unit.is_pixel_unit() && pixels > 0.0 {
                self.current.pixels_per_inch = (ppi * current / pixels).clamp(PPI_RANGE.0, PPI_RANGE.1);
                self.current.width = self.original.width;
                self.current.height = self.original.height;
            }
            return;
        }
        let pixels = if self.relative { original + pixels } else { pixels };
        if field == Field::Width {
            self.set_width(pixels);
        } else {
            self.set_height(pixels);
        }
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
        let pair = form.linked(&labels.constrain, &mut linked, |form| {
            for (field, label, unit_name) in [(Field::Width, &labels.width, &labels.width_unit), (Field::Height, &labels.height, &labels.height_unit)]
            {
                let row = form.row(label, |ui, row_label| self.dimension(ui, row_label, labels, field, unit_name, editable));
                responses.push(row.response);
                event = row.inner.1.or(event.take());
            }
        });
        if self.resampling {
            self.constrain = linked;
        }
        if with_resolution {
            let row = form.row(&labels.resolution, |ui, row_label| self.resolution(ui, row_label, labels));
            responses.push(row.response);
            event = row.inner.1.or(event.take());
        }
        let changed = event.is_some();
        let mut response = responses.into_iter().fold(pair.response, |all, r| all.union(r));
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
        let limit = unit.from_pixels(MAX_PIXELS, original, ppi);
        let least = if self.relative { -limit } else { unit.from_pixels(1.0, original, ppi).min(self.shown(field)) };
        let number = NumberField::new(id, name, self.shown(field), &numbers)
            .range(least..=limit)
            .step(unit.step())
            .decimals(unit.decimals()..=unit.decimals())
            .or_parse(move |text| unit.parse_typed(text, original, ppi))
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
            .decimals(places..=places);
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
}

impl<'a> AnchorGrid<'a> {
    pub fn new(anchor: &'a mut Anchor, delta: (f64, f64)) -> Self {
        Self { anchor, delta }
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
        let side = tokens.component.dialog.anchor_cell;
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
        let selected_fill = StateColors { fill: tokens.component.segmented.selected.fill.to_egui(), border: None };
        let sensed: Vec<_> = Anchor::ALL
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let (column, row) = cell.cell();
                let rect = Rect::from_min_size(grid.min + vec2(column as f32 * side, row as f32 * side), vec2(side, side)).shrink(1.0);
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
                    ui.painter().rect_filled(Rect::from_center_size(center, vec2(2.0 * half, 2.0 * half)), 1.0, mark);
                } else if let Some((dx, dy)) = arrow_direction(*self.anchor, Anchor::ALL[i], self.delta) {
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
    state: DialogState<(LinkedSizeFields, Resample, bool)>,
}

impl ImageSizeDialog {
    pub fn new(original: PixelSize) -> Self {
        Self {
            id: dialog_id("image-size"),
            fields: LinkedSizeFields::new(original),
            resample: Resample::Automatic,
            method: Resample::Automatic,
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

    /// The method the dialog opens with; [`Resample::None`] opens it with Resample off.
    pub fn resample(mut self, resample: Resample) -> Self {
        self.resample = resample;
        if resample != Resample::None {
            self.method = resample;
        }
        self.fields.resampling = resample != Resample::None;
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
        let labels = SizingLabels::shared_ctx(ctx);
        let reset = DialogLabels::of_ctx(ctx).reset;
        let mut fields = (self.fields.clone(), self.resample, self.scale_styles);
        let (bytes_per_pixel, method) = (self.bytes_per_pixel, &mut self.method);
        let shown = Dialog::new("image-size", labels.image_size_title.as_str())
            .stack(stack)
            .footer(Footer::standard().reset(Reset::AltCancel(reset)))
            .show(ctx, &mut self.state, &mut fields, |ui, (fields, resample, scale_styles)| {
                FormGrid::new("image-size").show(ui, |form| {
                    let (original, current) = (fields.original, fields.current);
                    let was = file_size(original, bytes_per_pixel);
                    form.value(
                        &labels.image_size,
                        craft_i18n::fmt(&labels.size_was, &[("size", &file_size(current, bytes_per_pixel)), ("old", &was)]),
                    );
                    let (width, height) = (current.width.to_string(), current.height.to_string());
                    form.value(&labels.dimensions, craft_i18n::fmt(&labels.dimensions_value, &[("width", &width), ("height", &height)]));
                    form.space(6.0);
                    let _ = fields.rows(form, &labels, true);
                    form.space(4.0);
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
                            for option in RESAMPLE_METHODS {
                                select = select.option(option, labels.method(option));
                            }
                            let _ = select.show(ui);
                            *resample = picked;
                        }
                        check
                    });
                    form.unlabelled(|ui| Checkbox::bool(&labels.scale_styles, scale_styles).enabled(fields.constrain || !fields.resampling).show(ui));
                });
            });
        let ImageSizeDialog { fields: kept, resample, scale_styles, .. } = self;
        match shown.outcome {
            Some(Outcome::Ok) => {
                (*kept, *resample, *scale_styles) = fields;
                let result = ImageSizeResult { size: kept.current, resample: *resample, scale_styles: *scale_styles };
                return Some(DialogResult::Ok(result));
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
}

/// Photoshop's Resample list, without Off.
const RESAMPLE_METHODS: [Resample; 7] = [
    Resample::Automatic,
    Resample::PreserveDetails,
    Resample::BicubicSmoother,
    Resample::BicubicSharper,
    Resample::Bicubic,
    Resample::NearestNeighbor,
    Resample::Bilinear,
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
    state: DialogState<(LinkedSizeFields, Anchor, CanvasExtension)>,
}

impl CanvasSizeDialog {
    pub fn new(original: PixelSize) -> Self {
        // Photoshop's Canvas Size has no linked proportions: the chain starts open.
        let fields = LinkedSizeFields { constrain: false, ..LinkedSizeFields::new(original) };
        Self {
            id: dialog_id("canvas-size"),
            fields,
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

    /// The extension the list opens with.
    pub fn extension(mut self, extension: CanvasExtension) -> Self {
        self.extension = extension;
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
        let labels = SizingLabels::shared_ctx(ctx);
        let reset = DialogLabels::of_ctx(ctx).reset;
        let mut fields = (self.fields.clone(), self.anchor, self.extension.clone());
        let (bytes_per_pixel, has_background) = (self.bytes_per_pixel, self.has_background);
        let shown = Dialog::new("canvas-size", labels.canvas_size_title.as_str())
            .stack(stack)
            .footer(Footer::standard().reset(Reset::AltCancel(reset)))
            .show(ctx, &mut self.state, &mut fields, |ui, (fields, anchor, extension)| {
                FormGrid::new("canvas-size").show(ui, |form| {
                    let original = fields.original;
                    form.value(&labels.current_size, file_size(original, bytes_per_pixel));
                    form.value(&labels.width, craft_i18n::fmt(&labels.pixels_value, &[("n", &original.width.to_string())]));
                    form.value(&labels.height, craft_i18n::fmt(&labels.pixels_value, &[("n", &original.height.to_string())]));
                    form.spanning(|ui| Divider::horizontal().show(ui));
                    let _ = fields.rows(form, &labels, false);
                    let current = fields.current;
                    form.value(&labels.new_size, file_size(current, bytes_per_pixel));
                    form.unlabelled(|ui| Checkbox::bool(&labels.relative, &mut fields.relative).show(ui));
                    let delta = (f64::from(current.width) - f64::from(original.width), f64::from(current.height) - f64::from(original.height));
                    form.row(&labels.anchor, |ui, _| AnchorGrid::new(anchor, delta).show(ui));
                    form.space(4.0);
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
                });
            });
        let CanvasSizeDialog { fields: kept, anchor, extension, has_background, .. } = self;
        match shown.outcome {
            Some(Outcome::Ok) => {
                (*kept, *anchor, *extension) = fields;
                let extension = if *has_background { extension.clone() } else { CanvasExtension::Transparent };
                return Some(DialogResult::Ok(CanvasSizeResult { size: kept.current, anchor: *anchor, extension }));
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
}
