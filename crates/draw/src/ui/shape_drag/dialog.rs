//! [`CreateShapeDialog`]: Create Rectangle, Ellipse, Triangle, Polygon (and Illustrator's Rounded
//! Rectangle and Star), opened by a Shape tool click (task SH-T7).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::RangeInclusive;

use egui::{Context, Ui};
use kurbo::Point;

use super::{CreateShapeLabels, ShapeKind};
use crate::geometry::shape::ShapeGeometry;
use craft_ui::dialog::{DialogId, DialogResult, DialogStack, DialogState, GuideUnits, StackDialog, dialog_id};
use craft_ui::localized::LocaleNumbers;

/// What Create Shape opens with: the kind, the clicked point and the document's units, and the
/// values the fields start from. The dialog replaces width, height, From Center and the star ratio
/// with the remembered ones for the kind (`shape.create.<kind>`) on its first pass unless
/// [`recalled`](Self::recalled) is set.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CreateShapeOptions {
    pub kind: ShapeKind,
    /// The click in document pixels, rounded: the shape's top left, or its centre with From Center.
    pub at: Point,
    /// The ruler unit the size fields show, and the document's size and resolution for percent
    /// and physical units. Pixels when `None`.
    pub units: Option<GuideUnits>,
    /// Width and height in document pixels (100 × 100 by default).
    pub size: (f64, f64),
    /// A rectangle's radii, top-left, top-right, bottom-right, bottom-left (from the tool's corner
    /// radius).
    pub radii: [f64; 4],
    /// "Link radii": one radius for all four corners; on by default.
    pub link_radii: bool,
    /// A polygon's sides or a star's points (from the tool).
    pub sides: u32,
    /// A star's inner over outer radius (1 for a plain polygon; the dialog shows 1–100 %).
    pub star_ratio: f64,
    pub from_center: bool,
    /// The remembered values are already in (a reopened dialog, a test).
    pub recalled: bool,
}

impl CreateShapeOptions {
    /// Width and height in pixels: 1 to 300,000 (PhotoCraft's `MAX_PX`, `shape_dialog.rs:19`).
    pub const SIZE: RangeInclusive<f64> = 1.0..=300_000.0;

    /// The dialog for a click of `kind` at `at`, with the defaults (100 × 100 px, square corners,
    /// radii linked, 5 sides, star ratio 1, not from the centre) in pixels.
    pub fn initial(kind: ShapeKind, at: Point) -> Self {
        CreateShapeOptions {
            kind,
            at,
            units: None,
            size: (100.0, 100.0),
            radii: [0.0; 4],
            link_radii: true,
            sides: 5,
            star_ratio: 1.0,
            from_center: false,
            recalled: false,
        }
    }

    /// The ruler unit and the document the size fields convert through.
    pub fn units(mut self, units: GuideUnits) -> Self {
        self.units = Some(units);
        self
    }

    /// The tool's corner radius (all four corners), sides and star ratio.
    pub fn tool(mut self, corner_radius: f64, sides: u32, star_ratio: f64) -> Self {
        self.radii = [corner_radius; 4];
        self.sides = sides;
        self.star_ratio = star_ratio;
        self
    }

    /// Opens with these values and skips the remembered ones.
    pub fn recalled(mut self) -> Self {
        self.recalled = true;
        self
    }
}

/// What Create Shape answers on OK.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CreatedShape {
    pub kind: ShapeKind,
    /// The shape at the click (centred on it with From Center), whole pixels.
    pub geometry: ShapeGeometry,
    pub from_center: bool,
}

/// Photoshop's Create Rectangle (Ellipse, Triangle, Polygon) dialog, on the app's `DialogHost`.
///
/// - Title: "Create Rectangle" and so on ([`CreateShapeLabels`]).
/// - Fields in a `FormGrid` (labels right-aligned): Width and Height (`unit_number_field` in the
///   ruler unit; a secondary click on a field picks that field's unit, which is not remembered,
///   PhotoCraft #7786). Rectangle and Rounded Rectangle: four radii Top Left, Top Right, Bottom
///   Right, Bottom Left with a "Link radii" `LinkedFields` toggle (`LinkMode::Equal`, on by
///   default). Polygon and Star: Sides (3–100) and Star Ratio (1–100 %, default 100). Then "From
///   Center".
/// - A width or height outside [`CreateShapeOptions::SIZE`] shows
///   [`CreateShapeLabels::bad_size`] under the fields and disables OK.
/// - OK answers [`CreatedShape`] at the click (centred on it with From Center) and remembers
///   width, height, From Center and the star ratio under `shape.create.<kind>` (keys `width`,
///   `height`, `fromCenter`, `starRatio`); Cancel answers nothing and remembers nothing.
/// - Keyboard: the first field has the focus; Enter is OK, Esc Cancel; every field is a Tab stop
///   in reading order. Right to left the grid mirrors; the corner order stays.
pub struct CreateShapeDialog {
    id: DialogId,
    options: CreateShapeOptions,
    /// Each size field's unit for this opening (a secondary-click pick).
    units: [Option<craft_ui::localized::units::LengthUnit>; 2],
    error: Option<String>,
    labels: Option<CreateShapeLabels>,
    state: DialogState,
}

impl CreateShapeDialog {
    /// Create Shape with `options`.
    pub fn new(options: CreateShapeOptions) -> Self {
        CreateShapeDialog { id: dialog_id("shape.create"), options, units: [None; 2], error: None, labels: None, state: DialogState::new() }
    }

    /// Whether `kind` has a Create dialog: every kind but Line and Custom (PhotoCraft has none for
    /// them; VectorCraft's Line Segment dialog is its own).
    pub fn supports(kind: ShapeKind) -> bool {
        !matches!(kind, ShapeKind::Line | ShapeKind::Custom)
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: CreateShapeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The values as the fields show them now.
    pub fn options(&self) -> &CreateShapeOptions {
        &self.options
    }

    /// The shape the fields describe at the click, or why not: the error text under the fields.
    fn answer(&self, labels: &CreateShapeLabels, numbers: &LocaleNumbers) -> Result<CreatedShape, String> {
        // pc: crates/ui-egui/src/dialogs/shape_dialog.rs:102-138 `geometry` (from the fields: at the
        // rounded `__at`, centred with From Center, radii, sides, star ratio), :42 `valid`, :19 `MAX_PX`.
        todo!()
    }

    /// Reads the remembered values of the kind into the options (first pass, which has the Host).
    fn recall(&mut self, ctx: &Context) {
        // pc: crates/ui-egui/src/dialogs/shape_dialog.rs:21 `KEYS`, :38 `pref_key` (`shape.create.<kind>`),
        // :58-96 `open` (the remembered values over the defaults).
        // new: `craft_settings::Remembered`, as `NewGuideDialog` uses it (crates/ui/src/dialog/guides.rs).
        todo!()
    }

    /// Stores the answer's remembered values.
    fn remember(&self, ctx: &Context, answer: &CreatedShape) {
        // pc: crates/ui-egui/src/dialogs/shape_dialog.rs:140-155 `confirm` (remembers on OK), :21 `KEYS`.
        // new: `craft_settings::Remembered`.
        todo!()
    }

    /// The fields.
    fn body(&mut self, ui: &mut Ui, labels: &CreateShapeLabels, numbers: &LocaleNumbers) {
        // pc: crates/ui-egui/src/dialogs/shape_dialog.rs:163-265 (Width, Height, the radius grid
        // with "Link radii" at :226, Sides :239, Star Ratio :246, From Center :258).
        // new: `FormGrid`, `unit_number_field`, `LinkedFields::<4>` with `LinkMode::Equal`.
        todo!()
    }
}

impl StackDialog for CreateShapeDialog {
    type Result = CreatedShape;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<CreatedShape>> {
        // pc: crates/ui-egui/src/dialogs/shape_dialog.rs:58-96 `open` (title from `kind_of` :28), :163 `body`.
        // new: `Dialog::with_id(..).stack(stack).width(Width::Compact).footer(Footer::standard())`
        // as `NewGuideDialog::show`.
        todo!()
    }

    fn confirm(&mut self, ctx: &Context) -> Option<CreatedShape> {
        // new: `answer`, then `remember`; `None` while a field is out of range.
        todo!()
    }
}
