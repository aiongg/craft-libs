//! [`ShapePanel`]: a shape layer's Appearance and Shape sections in the Properties panel.

use std::ops::RangeInclusive;

use craft_color::ui::{Paint, PaintChip};
use craft_theme::Srgba;
use egui::accesskit::Role;
use egui::{AsIdSalt, IdSalt, Ui, UiBuilder};

use super::{LiveShape, ShapeAppearance, ShapeChange, ShapeLabels};
use crate::ui::stroke::StrokeButton;
use craft_ui::composites::{FieldControl as _, PropertyGroup, PropertyLabel, PropertyRow, RowLabel, SectionHeader};
use craft_ui::controls::{NumberField, TooltipExt as _};
use craft_ui::edit::{EditEvent, EditResponse};
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit, UnitKind, Units};

/// Which paint a chip edits, for [`ShapePanel::chips`] and [`ShapeBar::chips`](super::ShapeBar::chips).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShapePaint {
    /// The fill chip.
    Fill,
    /// The stroke chip.
    Stroke,
}

/// The app's decoration of a paint chip before it shows: the Gradient, Pattern and Picture tabs.
pub(super) type ChipDecor<'a> = dyn for<'b> FnMut(ShapePaint, PaintChip<'b>) -> PaintChip<'b> + 'a;

/// What [`ShapePanel::show`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app turns each event into a document command"]
pub struct ShapePanelOutput {
    /// Each changed field this pass, in drawing order: a scrub or drag is `Begin`, `Update`,
    /// `Commit` (one undo step per gesture, replacing PhotoCraft's `coalesce` keys); a chip tab, a
    /// typed value or an arrow step is `Set`.
    pub events: Vec<EditEvent<ShapeChange>>,
}

/// A shape layer's Properties sections, as PhotoCraft's `shape_properties`.
///
/// - Appearance (collapsible `SectionHeader`, id `appearance`): Fill chip, Stroke chip, stroke
///   width (`NumberField` 0–288 px) and the Stroke style button (S7-pen's `StrokeButton`, enabled
///   while the width is over 0, its popover open flag from [`stroke_open`](Self::stroke_open)).
///   Stroke width 0 is no stroke; an edit that turns a stroke on at 0 carries
///   [`ShapeChange::ENABLED_STROKE_WIDTH`] in the same change; an arrow step that clamps at 0
///   reports nothing.
/// - Shape (collapsible, id `liveShape`), by the [`LiveShape`]: W and H (1–300,000 px, whole
///   pixels) for a rect, ellipse or polygon; a rect's "Corner radius" (0–100,000, sets all four)
///   then Top Left, Top Right, Bottom Left, Bottom Right rows (PhotoCraft's row order, mapped to
///   the radii order TL, TR, BR, BL); a polygon's Corner radius, Sides (3–100) and Star ratio
///   (1–100 %, stored 0.01–1); a line's Weight (1–10,000).
/// - A locked layer ([`ShapeAppearance::locked`]) disables the whole panel.
/// - Keyboard: every field and chip is a Tab stop in reading order; sections fold with their
///   headers. Right to left the rows mirror; the corner names stay.
pub struct ShapePanel<'a> {
    id_salt: IdSalt,
    shape: &'a LiveShape,
    appearance: &'a ShapeAppearance,
    stroke_open: Option<&'a mut bool>,
    chips: Option<Box<ChipDecor<'a>>>,
    fallbacks: (Srgba, Srgba),
    sections: (bool, bool),
    labels: Option<ShapeLabels>,
}

impl<'a> ShapePanel<'a> {
    /// The panel for `shape` with `appearance`.
    pub fn new(id_salt: impl AsIdSalt, shape: &'a LiveShape, appearance: &'a ShapeAppearance) -> Self {
        ShapePanel {
            id_salt: IdSalt::new(id_salt),
            shape,
            appearance,
            stroke_open: None,
            chips: None,
            fallbacks: (Srgba::BLACK, Srgba::WHITE),
            sections: (true, true),
            labels: None,
        }
    }

    /// The Stroke style popover's open flag, kept by the app between passes (S7-pen's
    /// `StrokeButton::show(ui, &mut open)`). Without it the button is not shown.
    pub fn stroke_open(mut self, open: &'a mut bool) -> Self {
        self.stroke_open = Some(open);
        self
    }

    /// Decorates each paint chip before it shows (its Gradient, Pattern and Picture editors).
    pub fn chips(mut self, decorate: impl for<'b> FnMut(ShapePaint, PaintChip<'b>) -> PaintChip<'b> + 'a) -> Self {
        self.chips = Some(Box::new(decorate));
        self
    }

    /// The colours a switch to Solid starts from when a chip has none: the app's foreground (fill)
    /// and background (stroke). Black and white by default.
    pub fn fallbacks(mut self, fill: Srgba, stroke: Srgba) -> Self {
        self.fallbacks = (fill, stroke);
        self
    }

    /// Which sections show; both by default (an app may show one, or a custom path's layer only
    /// Appearance).
    pub fn sections(mut self, appearance: bool, shape: bool) -> Self {
        self.sections = (appearance, shape);
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: ShapeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Adds the sections to `ui` and reports the changed fields.
    pub fn show(self, ui: &mut Ui) -> ShapePanelOutput {
        // pc: crates/ui-egui/src/tools/vector_ui/options.rs:156-312 `shape_properties` (Appearance at
        // :178, the live shape at :233, fields :232-300, corner rows :265, the stroke width rule
        // :126-128, the gesture keys :142-153).
        // new: `SectionHeader` (collapsible), `PaintChip`, `NumberField`, `craft_draw::ui::stroke::StrokeButton`
        // (its `EditEvent<StrokeStyle>` mapped to `ShapeChange::StrokeStyle`).
        let Self { id_salt, shape, appearance, stroke_open, mut chips, fallbacks, sections, labels } = self;
        let labels = labels.unwrap_or_else(|| ShapeLabels::of(ui));
        let units = Units::of(ui);
        let numbers = units.numbers();
        let mut events = Vec::new();
        let builder = UiBuilder::new().id_salt(id_salt).accessibility_role(Role::Group).accessibility_label(labels.properties.as_str());
        ui.scope_builder(builder, |ui| {
            ui.add_enabled_ui(!appearance.locked, |ui| {
                if sections.0 {
                    SectionHeader::new("appearance", labels.appearance.as_str()).show(ui, |ui| {
                        PropertyGroup::new("appearance-rows").show(ui, |ui| {
                            let paints = [
                                (
                                    ShapePaint::Fill,
                                    "fill",
                                    labels.fill.as_str(),
                                    labels.fill_color.as_str(),
                                    labels.set_fill_type.as_str(),
                                    &appearance.fill,
                                    fallbacks.0,
                                ),
                                (
                                    ShapePaint::Stroke,
                                    "stroke",
                                    labels.stroke.as_str(),
                                    labels.stroke_color.as_str(),
                                    labels.set_stroke_type.as_str(),
                                    &appearance.stroke,
                                    fallbacks.1,
                                ),
                            ];
                            for (which, salt, caption, name, tip, paint, fallback) in paints {
                                let edit = PropertyRow::word(caption)
                                    .show(ui, |ui, _| {
                                        let mut edit = paint_chip(ui, chips.as_deref_mut(), which, salt, name, paint, fallback, numbers);
                                        edit.response = edit.response.tooltip(tip);
                                        edit.own_name()
                                    })
                                    .inner;
                                paint_events(&mut events, which, edit.0.event, appearance.stroke_width);
                            }
                            PropertyRow::word(labels.stroke_width.as_str()).scrubs().show(ui, |ui, label| {
                                let field = stroke_width_field("stroke-width", &labels, appearance.stroke_width, &units);
                                let edit = scrubbed(field, label).show(ui);
                                width_events(&mut events, edit.event.clone(), appearance.stroke_width);
                                edit
                            });
                        });
                        if let Some(open) = stroke_open {
                            let corners = !matches!(shape, LiveShape::Ellipse { .. });
                            style_button(ui, &mut events, "stroke-style", appearance, corners, numbers, open);
                        }
                    });
                }
                if sections.1 {
                    SectionHeader::new("liveShape", labels.shape.as_str()).show(ui, |ui| {
                        PropertyGroup::new("shape-rows").show(ui, |ui| shape_rows(ui, &mut events, shape, &labels, &units));
                    });
                }
            });
        });
        ShapePanelOutput { events }
    }
}

/// The Shape section's rows for `shape`.
fn shape_rows(ui: &mut Ui, events: &mut Vec<EditEvent<ShapeChange>>, shape: &LiveShape, labels: &ShapeLabels, units: &Units) {
    let numbers = units.numbers();
    match shape {
        LiveShape::Rect { width, height, radii } => {
            size_row(ui, events, (*width, *height), labels, units);
            let radii = *radii;
            number_row(ui, events, Field::pixels("corner-radius", &labels.corner_radius, radii[0], 0.0..=100_000.0, units), numbers, |r| {
                ShapeChange::Radii([r; 4])
            });
            // PhotoCraft's row order; the radii order is top-left, top-right, bottom-right, bottom-left.
            let corners = [
                ("corner-top-left", &labels.top_left, 0),
                ("corner-top-right", &labels.top_right, 1),
                ("corner-bottom-left", &labels.bottom_left, 3),
                ("corner-bottom-right", &labels.bottom_right, 2),
            ];
            for (salt, caption, index) in corners {
                number_row(ui, events, Field::pixels(salt, caption, radii[index], 0.0..=100_000.0, units), numbers, |r| {
                    let mut next = radii;
                    next[index] = r;
                    ShapeChange::Radii(next)
                });
            }
        }
        LiveShape::Ellipse { width, height } => size_row(ui, events, (*width, *height), labels, units),
        LiveShape::Polygon { width, height, corner_radius, sides, star_ratio } => {
            size_row(ui, events, (*width, *height), labels, units);
            number_row(
                ui,
                events,
                Field::pixels("corner-radius", &labels.corner_radius, *corner_radius, 0.0..=100_000.0, units),
                numbers,
                ShapeChange::CornerRadius,
            );
            let count =
                Field { salt: "sides", caption: &labels.sides, value: f64::from(*sides), range: 3.0..=100.0, places: 0..=0, unit: Unit::default() };
            number_row(ui, events, count, numbers, |n| ShapeChange::Sides(n.round().clamp(3.0, 100.0) as u32));
            let ratio = Field {
                salt: "star-ratio",
                caption: &labels.star_ratio,
                value: star_ratio * 100.0,
                range: 1.0..=100.0,
                places: 0..=1,
                unit: units.unit(UnitKind::Percent),
            };
            number_row(ui, events, ratio, numbers, |percent| ShapeChange::StarRatio(percent / 100.0));
        }
        LiveShape::Line { weight } => {
            number_row(ui, events, Field::pixels("weight", &labels.weight, *weight, 1.0..=10_000.0, units), numbers, ShapeChange::Weight);
        }
    }
}

/// The W and H row of a rect, ellipse or polygon: whole pixels, 1 to 300,000.
fn size_row(ui: &mut Ui, events: &mut Vec<EditEvent<ShapeChange>>, (width, height): (f64, f64), labels: &ShapeLabels, units: &Units) {
    let numbers = units.numbers();
    let pixels = units.unit(UnitKind::Pixels);
    let size = |salt, name: &str, value| NumberField::new(salt, name, value, numbers).range(1.0..=300_000.0).decimals(0..=0).unit(pixels.clone());
    let (w, h) = PropertyRow::pair(PropertyLabel::glyph(labels.width.as_str()).scrubs(), PropertyLabel::glyph(labels.height.as_str()).scrubs())
        .show(
            ui,
            |ui, label| scrubbed(size("width", &labels.width_name, width), label).show(ui),
            |ui, label| scrubbed(size("height", &labels.height_name, height), label).show(ui),
        )
        .inner;
    events.extend(w.event.map(|e| e.map(|w| ShapeChange::Size { width: w, height })));
    events.extend(h.event.map(|e| e.map(|h| ShapeChange::Size { width, height: h })));
}

/// One number field of a row: its id, caption, value, range, decimals and unit.
pub(super) struct Field<'a> {
    pub salt: &'static str,
    pub caption: &'a str,
    pub value: f64,
    pub range: RangeInclusive<f64>,
    pub places: RangeInclusive<usize>,
    pub unit: Unit,
}

impl<'a> Field<'a> {
    /// A length in pixels with up to two decimals.
    pub fn pixels(salt: &'static str, caption: &'a str, value: f64, range: RangeInclusive<f64>, units: &Units) -> Self {
        Field { salt, caption, value, range, places: 0..=2, unit: units.unit(UnitKind::Pixels) }
    }

    pub fn number<'n>(&self, numbers: &'n LocaleNumbers) -> NumberField<'n> {
        NumberField::new(self.salt, self.caption, self.value, numbers).range(self.range.clone()).decimals(self.places.clone()).unit(self.unit.clone())
    }
}

/// A labelled row with `field`; its edit goes out through `change`.
fn number_row(ui: &mut Ui, events: &mut Vec<EditEvent<ShapeChange>>, field: Field<'_>, numbers: &LocaleNumbers, change: impl Fn(f64) -> ShapeChange) {
    let edit = PropertyRow::word(field.caption).scrubs().show(ui, |ui, label| scrubbed(field.number(numbers), label).show(ui)).inner;
    events.extend(edit.event.map(|e| e.map(&change)));
}

/// `field` scrubbed by its row's label when the label shows.
fn scrubbed<'n>(field: NumberField<'n>, label: &RowLabel<'_>) -> NumberField<'n> {
    match label.scrub() {
        Some(label) => field.scrub_by(label),
        None => field,
    }
}

/// The stroke width field: 0 to 288 px.
pub(super) fn stroke_width_field<'n>(salt: &'static str, labels: &ShapeLabels, width: f64, units: &'n Units) -> NumberField<'n> {
    Field::pixels(salt, &labels.stroke_width, width, 0.0..=288.0, units).number(units.numbers())
}

/// A paint chip, decorated by the app's `chips` closure.
#[expect(clippy::too_many_arguments, reason = "the chip's name, paint and fallback come from the caller")]
pub(super) fn paint_chip(
    ui: &mut Ui,
    chips: Option<&mut ChipDecor<'_>>,
    which: ShapePaint,
    salt: &'static str,
    name: &str,
    paint: &Paint,
    fallback: Srgba,
    numbers: &LocaleNumbers,
) -> EditResponse<Paint> {
    let chip = PaintChip::new(salt, name, paint, fallback, numbers);
    match chips {
        Some(decorate) => decorate(which, chip),
        None => chip,
    }
    .show(ui)
}

/// A chip's event as a change. A stroke at width 0 is no stroke: the edit that gives it a paint
/// carries the width that shows it (PhotoCraft's `enabled_stroke_width`), so the app applies one
/// change.
pub(super) fn paint_events(events: &mut Vec<EditEvent<ShapeChange>>, which: ShapePaint, event: Option<EditEvent<Paint>>, width: f64) {
    let Some(event) = event else { return };
    let paints = match &event {
        EditEvent::Set { value, .. } => !matches!(value, Paint::None),
        EditEvent::Begin { .. } => true,
        _ => false,
    };
    let enabling = (which == ShapePaint::Stroke && width <= 0.0 && paints).then_some(ShapeChange::ENABLED_STROKE_WIDTH);
    events.push(event.map(|paint| match which {
        ShapePaint::Fill => ShapeChange::Fill(paint),
        ShapePaint::Stroke => ShapeChange::Stroke { paint, width: enabling },
    }));
}

/// The width field's event. An arrow step that stays at 0 changes nothing, so it adds no stroke.
pub(super) fn width_events(events: &mut Vec<EditEvent<ShapeChange>>, event: Option<EditEvent<f64>>, width: f64) {
    let Some(event) = event else { return };
    let unchanged = matches!(&event, EditEvent::Set { value, .. } | EditEvent::Update { value, .. } if *value <= 0.0 && width <= 0.0);
    if !unchanged {
        events.push(event.map(ShapeChange::StrokeWidth));
    }
}

/// The Stroke style button over `appearance.stroke_style`. It reads "No Color" while the width is
/// 0; a pick then also carries the stroke's width in the same change.
pub(super) fn style_button(
    ui: &mut Ui,
    events: &mut Vec<EditEvent<ShapeChange>>,
    salt: &'static str,
    appearance: &ShapeAppearance,
    corners: bool,
    numbers: &LocaleNumbers,
    open: &mut bool,
) {
    let off = appearance.stroke_width <= 0.0;
    let response = StrokeButton::new(salt, &appearance.stroke_style, numbers).enabled(!off).corners(corners).show(ui, open);
    let Some(event) = response.event else { return };
    let width = (off && matches!(event, EditEvent::Set { .. } | EditEvent::Begin { .. })).then_some(ShapeChange::ENABLED_STROKE_WIDTH);
    events.push(event.map(|style| ShapeChange::StrokeStyle { style, width }));
}
