//! [`CreateShapeDialog`]: Create Rectangle, Ellipse, Triangle, Polygon (and Illustrator's Rounded
//! Rectangle and Star), opened by a Shape tool click (task SH-T7).

use std::cell::Cell;
use std::ops::RangeInclusive;
use std::sync::Arc;

use egui::{Context, Ui};
use kurbo::{Point, Rect};

use super::{CreateShapeLabels, ShapeKind};
use crate::geometry::shape::{SIDES, ShapeGeometry};
use craft_host::Host;
use craft_settings::Remembered;
use craft_ui::canvas::unit_entries;
use craft_ui::composites::{FieldControl as _, FormGrid, Notice, Tone};
use craft_ui::controls::expr::ExprUnits;
use craft_ui::controls::{Checkbox, ControlSize, LinkMode, LinkedFields, LinkedFieldsLabels, LinkedSlot, NumberField};
use craft_ui::dialog::{
    Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, GuideUnits, InitialFocus, Outcome, StackDialog, Width, dialog_id,
};
use craft_ui::edit::EditEvent;
use craft_ui::localized::units::{LengthUnit, UnitContext};
use craft_ui::localized::{Labels as _, LocaleNumbers, SharedLabels as _, Unit};
use craft_ui::patterns::unit_number_field;

/// The farthest the click may lie from the origin on either axis, in pixels: the same bound as
/// a side (PhotoCraft's `MAX_PX`, `shape_dialog.rs:19`, checked at `:111`).
const MAX_POSITION: f64 = 300_000.0;

/// The largest radius a field takes, in pixels (PhotoCraft's `value_field` range,
/// `shape_dialog.rs:223`); the shape clamps each radius to half its shorter side.
const MAX_RADIUS: f64 = 10_000.0;

/// What Create Shape opens with: the kind, the clicked point and the document's units, and the
/// values the fields start from. The dialog replaces width, height, From Center and the star ratio
/// with the remembered ones for the kind (`shape.create.<kind>`) on its first pass unless
/// [`recalled`](Self::recalled) is set.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CreateShapeOptions {
    /// Which shape the dialog creates.
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
    /// A polygon's or star's inner over outer radius (1 for a plain polygon; the dialog shows
    /// 1–100 % for Polygon and Star).
    pub star_ratio: f64,
    /// "From Center": the click is the shape's centre.
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
    /// Which shape was created.
    pub kind: ShapeKind,
    /// The shape at the click (centred on it with From Center, which may leave its top left on a
    /// half pixel, as PhotoCraft's).
    pub geometry: ShapeGeometry,
    /// Whether From Center was on.
    pub from_center: bool,
}

/// Photoshop's Create Rectangle (Ellipse, Triangle, Polygon) dialog, on the app's `DialogHost`.
///
/// - Title: "Create Rectangle" and so on ([`CreateShapeLabels`]).
/// - Fields in a `FormGrid` (labels right-aligned): Width and Height (`unit_number_field` in the
///   ruler unit; a secondary click on a field picks that field's unit, which is not remembered,
///   PhotoCraft #7786). Rectangle and Rounded Rectangle: four radii Top Left, Top Right, Bottom
///   Right, Bottom Left, as PhotoCraft's 2×2 grid (Top Left, Top Right / Bottom Left, Bottom
///   Right) under one "Radius" label with the corner names as the fields' accessible names, and a
///   "Link radii" `LinkedFields` toggle between the top two fields (`LinkMode::Equal`, on by
///   default). Polygon and Star: Sides (3–100) and Star Ratio (1–100 %, default 100); the
///   Polygon drag keeps ratio 1. Then "From Center".
/// - A width or height outside [`CreateShapeOptions::SIZE`] shows
///   [`CreateShapeLabels::bad_size`] under the fields and disables OK; so does a click farther
///   than 300,000 px from the origin ([`CreateShapeLabels::bad_position`]).
/// - OK answers [`CreatedShape`] at the click (centred on it with From Center, unrounded) and remembers
///   width, height, From Center and the star ratio under `shape.create.<kind>` (keys `width`,
///   `height`, `fromCenter`, `starRatio`, the ratio in percent); Cancel answers nothing and
///   remembers nothing.
/// - Keyboard: the first field has the focus; Enter is OK, Esc Cancel; every field is a Tab stop
///   in reading order. Right to left the grid mirrors; the corner order stays.
pub struct CreateShapeDialog {
    id: DialogId,
    options: CreateShapeOptions,
    /// Each size field's unit for this opening (a secondary-click pick).
    units: [Option<LengthUnit>; 2],
    error: Option<String>,
    labels: Option<CreateShapeLabels>,
    state: DialogState,
}

/// The remembered values of one kind. The ratio is the field's percentage.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Memory {
    width: Option<f64>,
    height: Option<f64>,
    from_center: Option<bool>,
    star_ratio: Option<f64>,
}

/// The document a dialog without [`CreateShapeOptions::units`] measures in: pixels, with no
/// percent.
const PIXELS: GuideUnits = GuideUnits { ruler: LengthUnit::Pixels, width_px: 1.0, height_px: 1.0, pixels_per_inch: 72.0, points_per_inch: 72.0 };

/// `value` rounded to a thousandth of a pixel (PhotoCraft stores sizes so).
fn thousandths(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

/// `value` rounded to the places a field in `unit` shows.
fn shown_places(value: f64, unit: LengthUnit) -> f64 {
    let scale = 10f64.powi(i32::try_from(unit.decimals()).unwrap_or(0));
    (value * scale).round() / scale
}

/// The value a field's event set, if it set one.
fn set_by(event: Option<&EditEvent<f64>>) -> Option<f64> {
    match event? {
        EditEvent::Commit { changed: false, .. } => None,
        event => event.value().copied(),
    }
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
        let o = &self.options;
        let side = |value: f64, caption: &str| {
            if value.is_finite() && CreateShapeOptions::SIZE.contains(&value) {
                return Ok(value);
            }
            let max = numbers.format(*CreateShapeOptions::SIZE.end(), 0..=0);
            Err(craft_i18n::fmt(&labels.bad_size, &[("field", caption), ("max", &max)]))
        };
        let (w, h) = (side(o.size.0, &labels.width)?, side(o.size.1, &labels.height)?);
        let at = Point::new(o.at.x.round(), o.at.y.round());
        if !(at.x.abs() <= MAX_POSITION && at.y.abs() <= MAX_POSITION) {
            return Err(labels.bad_position.clone());
        }
        let (x0, y0) = if o.from_center { (at.x - w / 2.0, at.y - h / 2.0) } else { (at.x, at.y) };
        let rect = Rect::new(x0, y0, x0 + w, y0 + h);
        let sides = o.sides.clamp(*SIDES.start(), *SIDES.end());
        let geometry = match o.kind {
            ShapeKind::Rectangle | ShapeKind::RoundedRectangle => {
                let most = w.min(h) / 2.0;
                let radii = o.radii.map(|r| if r.is_finite() { r.clamp(0.0, most) } else { 0.0 });
                ShapeGeometry::Rect { rect, radii }
            }
            ShapeKind::Ellipse => ShapeGeometry::Ellipse { rect },
            ShapeKind::Triangle => ShapeGeometry::Polygon { rect, sides: 3, star_ratio: 1.0 },
            ShapeKind::Polygon | ShapeKind::Star => {
                let ratio = if o.star_ratio.is_finite() { o.star_ratio.clamp(0.01, 1.0) } else { 1.0 };
                ShapeGeometry::Polygon { rect, sides, star_ratio: ratio }
            }
            ShapeKind::Line | ShapeKind::Custom => return Err(String::new()),
        };
        Ok(CreatedShape { kind: o.kind, geometry, from_center: o.from_center })
    }

    /// The `Remembered` scope of the kind: `shape.create.<kind>`.
    fn scope(&self) -> String {
        let kind = match self.options.kind {
            ShapeKind::Rectangle => "rectangle",
            ShapeKind::RoundedRectangle => "roundedRectangle",
            ShapeKind::Ellipse => "ellipse",
            ShapeKind::Triangle => "triangle",
            ShapeKind::Polygon => "polygon",
            ShapeKind::Star => "star",
            ShapeKind::Line => "line",
            ShapeKind::Custom => "custom",
        };
        format!("shape.create.{kind}")
    }

    /// Reads the remembered values of the kind into the options (first pass, which has the Host).
    fn recall(&mut self, ctx: &Context) {
        // pc: crates/ui-egui/src/dialogs/shape_dialog.rs:21 `KEYS`, :38 `pref_key` (`shape.create.<kind>`),
        // :58-96 `open` (the remembered values over the defaults).
        // new: `craft_settings::Remembered`, as `NewGuideDialog` uses it (crates/ui/src/dialog/guides.rs).
        self.options.recalled = true;
        let Some(memory) = Host::try_of_ctx(ctx).and_then(|host| Remembered::<Memory>::new(self.scope()).recall(&host)) else { return };
        let size = |value: Option<f64>| value.filter(|v| v.is_finite() && CreateShapeOptions::SIZE.contains(v));
        if let Some(width) = size(memory.width) {
            self.options.size.0 = width;
        }
        if let Some(height) = size(memory.height) {
            self.options.size.1 = height;
        }
        if let Some(from_center) = memory.from_center {
            self.options.from_center = from_center;
        }
        // PhotoCraft applies a remembered value only for a kind that has the field.
        if matches!(self.options.kind, ShapeKind::Polygon | ShapeKind::Star)
            && let Some(percent) = memory.star_ratio.filter(|v| v.is_finite() && (1.0..=100.0).contains(v))
        {
            self.options.star_ratio = percent / 100.0;
        }
    }

    /// Stores the answer's remembered values.
    fn remember(&self, ctx: &Context, answer: &CreatedShape) {
        // pc: crates/ui-egui/src/dialogs/shape_dialog.rs:140-155 `confirm` (remembers on OK), :21 `KEYS`.
        // new: `craft_settings::Remembered`.
        let Some(host) = Host::try_of_ctx(ctx) else { return };
        let bounds = answer.geometry.bounds();
        let memory = Memory {
            width: Some(bounds.width()),
            height: Some(bounds.height()),
            from_center: Some(answer.from_center),
            star_ratio: Some((self.options.star_ratio * 100.0).clamp(1.0, 100.0)),
        };
        Remembered::<Memory>::new(self.scope()).store(&host, &memory);
    }

    /// The document the size fields convert through.
    fn document(&self) -> GuideUnits {
        self.options.units.unwrap_or(PIXELS)
    }

    /// The Width field's id, which the dialog names as its initial focus.
    fn width_id(&self) -> egui::Id {
        self.id.with("width")
    }

    /// One size field (`axis` 0 is Width, 1 Height) in its unit.
    fn size_field(&mut self, ui: &mut Ui, axis: usize, caption: &str, numbers: &LocaleNumbers) -> egui::Response {
        let document = self.document();
        let unit = self.units[axis].unwrap_or(document.ruler);
        let context = document.context(if axis == 0 { document.width_px } else { document.height_px });
        let pixels = if axis == 0 { self.options.size.0 } else { self.options.size.1 };
        let shown = context.from_pixels(pixels, unit).unwrap_or(0.0);
        let menu: Vec<LengthUnit> = LengthUnit::ALL.into_iter().filter(|u| self.options.units.is_some() || *u != LengthUnit::Percent).collect();
        let entries = unit_entries(ui, &menu, unit);
        let mut picked = None;
        let units = ExprUnits { unit: Some(unit), suffix: unit.suffix(), context };
        let mut field = unit_number_field(("create-shape-size", axis), caption, shown, numbers, &units).unit_menu(&entries, &mut picked);
        if axis == 0 {
            field = field.with_id(self.width_id());
        }
        let edit = field.show(ui);
        if let Some(i) = picked {
            self.units[axis] = menu.get(i).copied();
        }
        if let Some(value) = set_by(edit.event.as_ref())
            && shown_places(value, unit) != shown_places(shown, unit)
            && let Some(px) = context.to_pixels(value, unit)
        {
            let px = thousandths(px);
            if axis == 0 {
                self.options.size.0 = px;
            } else {
                self.options.size.1 = px;
            }
        }
        edit.response
    }

    /// The fields.
    fn body(&mut self, ui: &mut Ui, labels: &CreateShapeLabels, numbers: &LocaleNumbers) {
        // pc: crates/ui-egui/src/dialogs/shape_dialog.rs:163-265 (Width, Height, the radius grid
        // with "Link radii" at :226, Sides :239, Star Ratio :246, From Center :258).
        // new: `FormGrid`, `unit_number_field`, `LinkedFields::<4>` with `LinkMode::Equal`.
        let kind = self.options.kind;
        FormGrid::new("create-shape").show(ui, |form| {
            form.row(labels.width.as_str(), |ui, _| self.size_field(ui, 0, &labels.width, numbers));
            form.row(labels.height.as_str(), |ui, _| self.size_field(ui, 1, &labels.height, numbers));
            if matches!(kind, ShapeKind::Rectangle | ShapeKind::RoundedRectangle) {
                form.row(labels.radius.as_str(), |ui, _| {
                    let mut link_words = LinkedFieldsLabels::of(ui);
                    link_words.same_values = labels.link_radii.clone();
                    let units = ExprUnits { unit: Some(LengthUnit::Pixels), suffix: LengthUnit::Pixels.suffix(), context: UnitContext::default() };
                    // The radii are top-left, top-right, bottom-right, bottom-left; the grid reads
                    // top-left, top-right, bottom-left, bottom-right.
                    let [top_left, top_right, bottom_right, bottom_left] = self.options.radii;
                    let captions = [labels.top_left.as_str(), labels.top_right.as_str(), labels.bottom_left.as_str(), labels.bottom_right.as_str()];
                    let linked = &mut self.options.link_radii;
                    // Columns: first field, link, second field; the second row leaves the link's cell empty.
                    let edit = egui::Grid::new("create-shape-radii-grid")
                        .show(ui, |ui| {
                            LinkedFields::new(
                                "create-shape-radii",
                                captions,
                                [top_left, top_right, bottom_left, bottom_right],
                                linked,
                                LinkMode::Equal,
                                numbers,
                            )
                            .units(&units)
                            .range(0.0..=MAX_RADIUS)
                            .captions_as_names()
                            .size(ControlSize::Sm)
                            .labels(link_words)
                            .show_cells(ui, |ui, slot, add| match slot {
                                LinkedSlot::Field(1) => {
                                    add(ui);
                                    ui.end_row();
                                }
                                LinkedSlot::Field(2) => {
                                    add(ui);
                                    ui.allocate_space(egui::Vec2::ZERO);
                                }
                                LinkedSlot::Field(_) | LinkedSlot::Link => add(ui),
                            })
                        })
                        .inner;
                    if let Some(event) = &edit.event
                        && !matches!(event, EditEvent::Commit { changed: false, .. })
                        && let Some([top_left, top_right, bottom_left, bottom_right]) = event.value().copied()
                    {
                        self.options.radii = [top_left, top_right, bottom_right, bottom_left];
                    }
                    // The row's label must not replace the Top Left field's corner name.
                    edit.own_name()
                });
            }
            if matches!(kind, ShapeKind::Polygon | ShapeKind::Star) {
                form.row(labels.sides.as_str(), |ui, _| {
                    let field = NumberField::new("create-shape-sides", labels.sides.as_str(), f64::from(self.options.sides), numbers)
                        .range(f64::from(*SIDES.start())..=f64::from(*SIDES.end()))
                        .decimals(0..=0)
                        .size(ControlSize::Sm);
                    let edit = field.show(ui);
                    if let Some(value) = set_by(edit.event.as_ref()) {
                        let rounded = value.round().clamp(f64::from(*SIDES.start()), f64::from(*SIDES.end()));
                        self.options.sides = rounded as u32;
                    }
                    // The row's label must not replace the Top Left field's corner name.
                    edit.own_name()
                });
            }
            if matches!(kind, ShapeKind::Polygon | ShapeKind::Star) {
                form.row(labels.star_ratio.as_str(), |ui, _| {
                    let percent = (self.options.star_ratio * 100.0).clamp(1.0, 100.0);
                    let field = NumberField::new("create-shape-ratio", labels.star_ratio.as_str(), percent, numbers)
                        .range(1.0..=100.0)
                        .decimals(0..=1)
                        .unit(Unit::new("{n}%"))
                        .size(ControlSize::Sm);
                    let edit = field.show(ui);
                    if let Some(value) = set_by(edit.event.as_ref())
                        && (value - percent).abs() > f64::EPSILON
                    {
                        self.options.star_ratio = (value / 100.0).clamp(0.01, 1.0);
                    }
                    edit
                });
            }
            form.unlabelled(|ui| Checkbox::bool(labels.from_center.as_str(), &mut self.options.from_center).show(ui));
        });
        self.error = self.answer(labels, numbers).err().filter(|message| !message.is_empty());
        if let Some(message) = &self.error {
            Notice::new(message.as_str()).tone(Tone::Bad).id_salt("create-shape-error").show(ui);
        }
    }

    /// The words: the given ones, else the installed ones.
    fn words(&self, ctx: &Context) -> Arc<CreateShapeLabels> {
        self.labels.clone().map_or_else(|| CreateShapeLabels::shared_ctx(ctx), Arc::new)
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
        let labels = self.words(ctx);
        if !self.options.recalled {
            self.recall(ctx);
        }
        let numbers = LocaleNumbers::of_ctx(ctx);
        let title = match self.options.kind {
            ShapeKind::RoundedRectangle => &labels.rounded_rectangle,
            ShapeKind::Ellipse => &labels.ellipse,
            ShapeKind::Triangle => &labels.triangle,
            ShapeKind::Polygon => &labels.polygon,
            ShapeKind::Star => &labels.star,
            ShapeKind::Rectangle | ShapeKind::Line | ShapeKind::Custom => &labels.rectangle,
        };
        let valid = Cell::new(self.answer(&labels, &numbers).is_ok());
        let width_id = self.width_id();
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::with_id(self.id, title.as_str())
            .stack(stack)
            .width(Width::Compact)
            .initial_focus(InitialFocus::Field(width_id))
            .footer(Footer::standard().ok_enabled_with(|| valid.get()))
            .show(ctx, &mut state, &mut (), |ui, ()| {
                self.body(ui, &labels, &numbers);
                valid.set(self.error.is_none() && self.answer(&labels, &numbers).is_ok());
            });
        self.state = state;
        match shown.outcome {
            Some(Outcome::Ok) => {
                let answer = self.answer(&labels, &numbers).ok()?;
                self.remember(ctx, &answer);
                Some(DialogResult::Ok(answer))
            }
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }

    fn confirm(&mut self, ctx: &Context) -> Option<CreatedShape> {
        // new: `answer`, then `remember`; `None` while a field is out of range.
        let labels = self.words(ctx);
        let answer = self.answer(&labels, &LocaleNumbers::of_ctx(ctx)).ok()?;
        self.remember(ctx, &answer);
        Some(answer)
    }
}
