//! [`TransformFields`]: the options bar's X, Y, W, H, angle and skew fields of a transform box.

use craft_theme::ControlSize;
use egui::{Align, Layout, Ui, UiBuilder, vec2};
use kurbo::{Rect, Vec2};

use super::RulerUnits;
use super::quad::{BoxState, SKEW_LIMIT};
use super::story;
use crate::composites::{PropertyLabel, PropertyRow};
use crate::controls::number_field::{self, FieldRule};
use crate::controls::{IconButton, NumberField};
use crate::direction;
use crate::edit::ops::{self, ScalarOp};
use crate::edit::{self, EditEvent, EditPolicy, EditResponse, Gesture, GestureId, parts};
use crate::icons::Icon;
use crate::localized::{LocaleNumbers, Unit};
use crate::primitives::Handle;
use crate::primitives::HandleShape;
use crate::story::{Level, Story, StoryChoice, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf};
use crate::typed_story;

/// The largest position of the box's reference point, in document units (PhotoCraft).
pub const POSITION_LIMIT: f64 = 300_000.0;
/// The largest scale a W or H field takes, as a factor (1 000 000 %).
const SCALE_LIMIT: f64 = 10_000.0;
/// The largest angle.
const ANGLE_LIMIT: f64 = 180.0;

/// The unit of the W and H fields.
// Built once per frame, so the size of the variants costs nothing.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Default)]
pub enum SizeUnit {
    /// The scale in percent of the box's size before the transform.
    #[default]
    Percent,
    /// The box's length, in the host's units along each axis (the unit its W and H context menu
    /// picked; PhotoCraft keeps it apart from the rulers').
    Length { x: RulerUnits, y: RulerUnits },
}

/// A field's visible label and its accessible name.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldLabel {
    /// The short text beside the field ("W").
    pub glyph: String,
    /// The field's accessible name ("Width").
    pub name: String,
}

impl FieldLabel {
    pub fn new(glyph: impl Into<String>, name: impl Into<String>) -> Self {
        Self { glyph: glyph.into(), name: name.into() }
    }
}

/// The host's translated labels of [`TransformFields`]; `Default` is English.
#[derive(Clone, Debug, PartialEq)]
pub struct TransformFieldLabels {
    pub x: FieldLabel,
    pub y: FieldLabel,
    pub w: FieldLabel,
    pub h: FieldLabel,
    /// The tooltip and name of the aspect link toggle.
    pub link: String,
    pub angle: FieldLabel,
    pub skew_h: FieldLabel,
    pub skew_v: FieldLabel,
    /// Around a W or H scale: `{n}%`.
    pub percent: Unit,
    /// Around an angle or a skew: `{n}°`.
    pub degrees: Unit,
}

impl Default for TransformFieldLabels {
    fn default() -> Self {
        Self {
            x: FieldLabel::new("X", "X position"),
            y: FieldLabel::new("Y", "Y position"),
            w: FieldLabel::new("W", "Width"),
            h: FieldLabel::new("H", "Height"),
            link: "Maintain aspect ratio".into(),
            angle: FieldLabel::new("°", "Angle"),
            skew_h: FieldLabel::new("H", "Horizontal skew"),
            skew_v: FieldLabel::new("V", "Vertical skew"),
            percent: Unit::new("{n}%"),
            degrees: Unit::new("{n}\u{b0}"),
        }
    }
}

/// One of the fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Field {
    X,
    Y,
    W,
    H,
    Angle,
    SkewH,
    SkewV,
}

/// How the fields read and write a box: the box before the transform (`rect`), the units, and
/// whether W and H are linked.
#[derive(Clone, Debug)]
pub(crate) struct Conversion {
    pub rect: Rect,
    pub x: RulerUnits,
    pub y: RulerUnits,
    pub size: SizeUnit,
    pub linked: bool,
}

impl Conversion {
    /// The length of the box's side before the transform along `axis` (0: W, 1: H).
    fn side(&self, axis: usize) -> f64 {
        (if axis == 0 { self.rect.width() } else { self.rect.height() }).abs()
    }

    /// Field units of the size fields per unit of scale factor.
    fn per_factor(&self, axis: usize) -> f64 {
        match &self.size {
            SizeUnit::Percent => 100.0,
            SizeUnit::Length { x, y } => self.side(axis).max(1e-9) * if axis == 0 { x.per_doc } else { y.per_doc },
        }
    }

    /// The largest scale factor along `axis`: as far as the box stays within the position limit.
    fn limit(&self, axis: usize) -> f64 {
        let side = self.side(axis);
        if side.is_finite() && side >= 1e-6 { (POSITION_LIMIT / side).clamp(1.0, SCALE_LIMIT) } else { SCALE_LIMIT }
    }

    /// The largest value a field shows or takes.
    pub fn bound(&self, field: Field) -> f64 {
        match field {
            Field::X => POSITION_LIMIT * self.x.per_doc,
            Field::Y => POSITION_LIMIT * self.y.per_doc,
            Field::W => self.limit(0) * self.per_factor(0),
            Field::H => self.limit(1) * self.per_factor(1),
            Field::Angle => ANGLE_LIMIT,
            Field::SkewH | Field::SkewV => SKEW_LIMIT,
        }
    }

    /// What `field` shows for `state`, whose V skew is `skew_v`.
    pub fn shown(&self, state: &BoxState, skew_v: f64, field: Field) -> f64 {
        let parts = state.parts(self.rect, skew_v);
        match field {
            Field::X => state.pivot.x * self.x.per_doc,
            Field::Y => state.pivot.y * self.y.per_doc,
            Field::W => parts.w * self.per_factor(0),
            Field::H => parts.h * self.per_factor(1),
            Field::Angle => parts.angle,
            Field::SkewH => parts.skew_h,
            Field::SkewV => skew_v,
        }
    }

    /// The box and V skew after `field` is set to `value`: X and Y move the box with its reference
    /// point; the others scale, turn and skew it about the reference point. Linked, the other
    /// side of W or H follows by the same factor, each kept within its limit.
    pub fn edit(&self, state: &BoxState, skew_v: f64, field: Field, value: f64) -> (BoxState, f64) {
        let now = state.parts(self.rect, skew_v);
        let mut to = now;
        match field {
            Field::X => return (state.moved(Vec2::new(value / self.x.per_doc - state.pivot.x, 0.0)), skew_v),
            Field::Y => return (state.moved(Vec2::new(0.0, value / self.y.per_doc - state.pivot.y)), skew_v),
            Field::W | Field::H => {
                let axis = usize::from(field == Field::H);
                let factor = value / self.per_factor(axis);
                if self.linked {
                    let (side, other) = if axis == 0 { (now.w, now.h) } else { (now.h, now.w) };
                    let k = factor / nonzero(side);
                    let cap = (self.limit(axis) / nonzero(side.abs())).min(self.limit(1 - axis) / nonzero(other.abs()));
                    let k = k.signum() * k.abs().min(cap);
                    to.w = now.w * k;
                    to.h = now.h * k;
                } else if axis == 0 {
                    to.w = factor;
                } else {
                    to.h = factor;
                }
            }
            Field::Angle => to.angle = value,
            Field::SkewH => to.skew_h = value,
            Field::SkewV => to.skew_v = value,
        }
        (state.with_parts(self.rect, skew_v, to), to.skew_v)
    }
}

fn nonzero(v: f64) -> f64 {
    if v.abs() < 1e-9 { 1e-9_f64.copysign(if v == 0.0 { 1.0 } else { v }) } else { v }
}

/// What the row edits and reports: the box and the V skew it reads with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformRow {
    pub state: BoxState,
    pub skew_v: f64,
}

impl Conversion {
    /// The row after `field` is set to `value` in `row`.
    fn row(&self, row: &TransformRow, field: Field, value: f64) -> TransformRow {
        let (state, skew_v) = self.edit(&row.state, row.skew_v, field, value);
        TransformRow { state, skew_v }
    }
}

/// What the row keeps between passes.
#[derive(Clone, Debug, Default)]
struct Kept {
    parts: parts::State<(Field, ScalarOp)>,
    /// Each field's last value in the row's gesture, with that gesture and the row the value gave.
    reported: [Option<(GestureId, f64, TransformRow)>; FIELDS],
    /// The row the gesture the host was told of began from, kept from its Begin: a cancel that
    /// arrives with the gesture already gone (`cancel_all`, disabling, an outside change) restores
    /// the row's own V skew from it. A host's V skew is the host's to revert.
    start: Option<(GestureId, TransformRow)>,
}

const FIELDS: usize = 7;

/// The options bar's fields of a transform box: reference point X and Y, W and H with the aspect
/// link, the angle, and the horizontal and vertical skew.
///
/// ```
/// use craft_ui::canvas::kurbo::Rect;
/// use craft_ui::canvas::quad::BoxState;
/// use craft_ui::canvas::{TransformFields, TransformFieldLabels};
/// # egui::__run_test_ui(|ui| {
/// let rect = Rect::new(0.0, 0.0, 200.0, 100.0);
/// let mut state = BoxState::from_rect(rect);
/// let mut skew_v = 0.0;
/// let mut linked = true;
/// let edit = TransformFields::new("transform-fields", &state, rect, TransformFieldLabels::default())
///     .skew_v(skew_v)
///     .linked(&mut linked)
///     .show(ui);
/// if let Some(row) = edit.event.as_ref().and_then(|e| e.value()) {
///     state = row.state;
///     skew_v = row.skew_v;
/// }
/// # let _ = (state, skew_v);
/// # });
/// ```
///
/// - **Fields** are [`NumberField`]s in [`PropertyRow`] cells, glyph labels as `PropertyRow` has
///   them. X and Y are the reference point's position in [`units`](Self::units) and move the box
///   with it; W and H scale it about the reference point, as a percentage of `start_rect` or as
///   a length ([`size_unit`](Self::size_unit)); the angle and the two skews turn and lean it about
///   the reference point ([`BoxState::with_parts`]). A flipped box reads as a negative W or H and
///   keeps its sign. Limits: position ±300 000 document units, angle −180° to 180°, skew ±89°,
///   scale as far as the box stays within the position limit.
/// - **Linked** ([`linked`](Self::linked)): W or H also scales the other by the same factor,
///   each kept within its own limit. The toggle is an `IconButton` between the two; without
///   `linked` the row keeps the link itself.
/// - **One gesture.** The row is one edit of the box, as a colour picker is of its colour: the
///   field whose gesture (a scrub, a typing run) begins it owns it, and its Commit or Cancel ends
///   it; the other fields' values in the meantime join it, and an arrow step outside a gesture is
///   a Set. A press on a field while another field's typing run is open waits for the run: the
///   run commits first, and the press begins a gesture of its own from the box the run left.
///   Every field's event in a frame counts, each from the box the ones before it left.
///   While the box is the one a field's value gave, the field reads that value back, so a
///   scrub or a capped linked size isn't taken for the box moving under it.
/// - **V skew.** The V field is the host's `skew_v` ([`skew_v`](Self::skew_v)): the box does not
///   hold it (any parallelogram reads with any V skew), so the host keeps it for the session and
///   gives it by value, or the row keeps its own. The events carry a [`TransformRow`], the box
///   with its V skew, and the host applies both as it applies the box: a cancel, whatever ends
///   it, reverts both with the host's document. A row that keeps its own V skew writes the
///   one each event carries, and a cancel restores the one the gesture began with.
/// - [`policy`](Self::policy) is the row's: its update rate and cost apply to the box's events;
///   Esc cancels a field's gesture, and that cancels the row's, unless the policy isn't
///   cancellable.
///
/// The host applies the events as a box session's steps.
#[must_use = "show the fields with `show`"]
pub struct TransformFields<'a> {
    id_salt: egui::IdSalt,
    state: &'a BoxState,
    rect: Rect,
    labels: TransformFieldLabels,
    x: RulerUnits,
    y: RulerUnits,
    size_unit: SizeUnit,
    linked: Option<&'a mut bool>,
    skew_v: Option<f64>,
    size: ControlSize,
    enabled: bool,
    policy: EditPolicy,
}

impl<'a> TransformFields<'a> {
    /// The fields of `state`, a box that was `start_rect` before the transform.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, state: &'a BoxState, start_rect: Rect, labels: TransformFieldLabels) -> Self {
        let pixels = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
        Self {
            id_salt: egui::IdSalt::new(id_salt),
            state,
            rect: start_rect,
            labels,
            x: pixels.clone(),
            y: pixels,
            size_unit: SizeUnit::Percent,
            linked: None,
            skew_v: None,
            size: ControlSize::Md,
            enabled: true,
            policy: EditPolicy::default(),
        }
    }

    /// The units of X and Y (pixels by default); the first one's number format is the fields'.
    pub fn units(mut self, x: &RulerUnits, y: &RulerUnits) -> Self {
        self.x = x.clone();
        self.y = y.clone();
        self
    }

    /// The unit of W and H; percent by default.
    pub fn size_unit(mut self, unit: SizeUnit) -> Self {
        self.size_unit = unit;
        self
    }

    /// The aspect link, which the host also gives
    /// [`BoxRules::aspect_linked`](super::quad::BoxRules::aspect_linked). The toggle edits it.
    pub fn linked(mut self, linked: &'a mut bool) -> Self {
        self.linked = Some(linked);
        self
    }

    /// The V skew the host keeps for the session, in degrees.
    pub fn skew_v(mut self, skew_v: f64) -> Self {
        self.skew_v = Some(skew_v);
        self
    }

    /// The fields' size (`Md` by default).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Whether the fields take input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// How the row reports its edits: a host previewing a large layer asks for
    /// [`Debounced`](crate::edit::UpdateRate::Debounced) updates or
    /// [`on_release_only`](EditPolicy::on_release_only).
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Shows the row and returns this frame's edit, with the box it gives.
    pub fn show(self, ui: &mut Ui) -> EditResponse<TransformRow> {
        let Self { id_salt, state, rect, labels, x, y, size_unit, linked, skew_v, size, enabled, policy } = self;
        let id = ui.make_persistent_id(id_salt);
        let (skew_key, link_key, kept_key) = (id.with("skew_v"), id.with("linked"), id.with("kept"));
        let own_skew: f64 = ui.data(|d| d.get_temp(skew_key)).unwrap_or(0.0);
        let mut own_link: bool = ui.data(|d| d.get_temp(link_key)).unwrap_or(true);
        let (host_skew, host_link) = (skew_v.is_some(), linked.is_some());
        let mut skew_v = skew_v.unwrap_or(own_skew);
        let linked = linked.unwrap_or(&mut own_link);
        let convert = Conversion { rect, x, y, size: size_unit, linked: *linked };
        let numbers = convert.x.numbers.clone();
        let enabled = enabled && ui.is_enabled();

        // Esc is the fields': each cancels its own gesture, which cancels the row's.
        let outer = Gesture::load(ui, id, &TransformRow { state: *state, skew_v }, policy.cancellable(false)).enabled(enabled);
        let mut kept: Kept = ui.data(|d| d.get_temp(kept_key)).unwrap_or_default();
        let gesture = outer.id();
        for reported in &mut kept.reported {
            if reported.is_some_and(|(of, ..)| Some(of) != gesture) {
                *reported = None;
            }
        }
        let scoped = edit::in_scope(ui);
        let mut edits = parts::Edits::<TransformRow, (Field, ScalarOp)>::new(id, &kept.parts, &outer, scoped);
        let inner = EditPolicy::default().cancellable(policy.cancellable).cost(policy.cost);
        let tokens = Tokens::of(ui);
        let row_style = PropertyRow::style(&tokens, size);
        let cell = row_style.glyph_width + row_style.label_gap + NumberField::style(&tokens, size).field.width;
        let pair = 2.0 * cell + row_style.column_gap;
        let linked_pair = pair + row_style.link_width + 2.0 * row_style.column_gap;

        // Each field's unit and decimals.
        let format = |which: Field| match (which, &convert.size) {
            (Field::X, _) => (convert.x.unit.clone(), convert.x.decimals),
            (Field::Y, _) => (convert.y.unit.clone(), convert.y.decimals),
            (Field::W, SizeUnit::Length { x, .. }) => (x.unit.clone(), x.decimals),
            (Field::H, SizeUnit::Length { y, .. }) => (y.unit.clone(), y.decimals),
            (Field::W | Field::H, SizeUnit::Percent) => (labels.percent.clone(), 1),
            _ => (labels.degrees.clone(), 2),
        };
        let rule = |which: Field| {
            let bound = convert.bound(which);
            FieldRule::new(&(-bound..=bound), 1.0, format(which).1)
        };
        // What a field shows on `row`: the part of the box it reads, or the value it gave while
        // the box is the one that value made.
        let reported = kept.reported;
        let reads = |which: Field, row: &TransformRow| match reported[which as usize] {
            Some((_, value, of)) if of == *row => value,
            _ => convert.shown(&row.state, row.skew_v, which),
        };
        // A field's operation applies to what the field shows on the row of its turn.
        let apply = |(which, op): &(Field, ScalarOp), now: &TransformRow| {
            Some(convert.row(now, *which, number_field::apply(op, reads(*which, now), &rule(*which))))
        };
        edits.replay(ui.ctx(), apply);
        let shown = *edits.value();

        let field = |ui: &mut Ui, which: Field, label: &FieldLabel| {
            let (unit, decimals) = format(which);
            let bound = convert.bound(which);
            NumberField::new((id, which as u8), label.name.clone(), reads(which, &shown), &numbers)
                .range(-bound..=bound)
                .decimals(0..=decimals)
                .unit(unit)
                .policy(inner)
                .part(true)
                .size(size)
                .enabled(enabled)
                .show(ui)
        };
        let glyph = |label: &FieldLabel| PropertyLabel::glyph(label.glyph.clone());
        let group = |ui: &mut Ui, width: f32, add: &mut dyn FnMut(&mut Ui)| {
            ui.allocate_ui(vec2(width, 0.0), |ui| {
                ui.set_max_width(width);
                add(ui);
            });
        };

        let mut fields: Vec<(Field, EditResponse<f64>)> = Vec::with_capacity(FIELDS);
        let layout = if direction::is_rtl(ui) { Layout::right_to_left(Align::Min) } else { Layout::left_to_right(Align::Min) }.with_main_wrap(true);
        let row = ui.scope_builder(UiBuilder::new().layout(layout), |ui| {
            group(ui, pair, &mut |ui| {
                let (a, b) = PropertyRow::pair(glyph(&labels.x), glyph(&labels.y))
                    .size(size)
                    .show(ui, |ui, _| field(ui, Field::X, &labels.x), |ui, _| field(ui, Field::Y, &labels.y))
                    .inner;
                fields.extend([(Field::X, a), (Field::Y, b)]);
            });
            group(ui, linked_pair, &mut |ui| {
                let (a, _, b) = PropertyRow::pair(glyph(&labels.w), glyph(&labels.h))
                    .size(size)
                    .show_linked(
                        ui,
                        |ui, _| field(ui, Field::W, &labels.w),
                        |ui| {
                            let icon = if *linked { Icon::Link } else { Icon::Unlink };
                            ui.add(IconButton::new(icon, labels.link.clone()).size(ControlSize::Sm).enabled(enabled).toggle(linked))
                        },
                        |ui, _| field(ui, Field::H, &labels.h),
                    )
                    .inner;
                fields.extend([(Field::W, a), (Field::H, b)]);
            });
            group(ui, cell, &mut |ui| {
                let edit = PropertyRow::new(glyph(&labels.angle)).size(size).show(ui, |ui, _| field(ui, Field::Angle, &labels.angle)).inner;
                fields.push((Field::Angle, edit));
            });
            group(ui, pair, &mut |ui| {
                let (a, b) = PropertyRow::pair(glyph(&labels.skew_h), glyph(&labels.skew_v))
                    .size(size)
                    .show(ui, |ui, _| field(ui, Field::SkewH, &labels.skew_h), |ui, _| field(ui, Field::SkewV, &labels.skew_v))
                    .inner;
                fields.extend([(Field::SkewH, a), (Field::SkewV, b)]);
            });
        });

        // Each field's value applies to the row the fields before it in this pass left, with the
        // values that go out in this pass: one that waits for another field's gesture isn't in it.
        let mut values: Vec<(Field, f64, TransformRow)> = Vec::new();
        for (which, edit) in &fields {
            let (which, base) = (*which, *edits.value());
            let field_ops = ops::take::<ScalarOp>(ui.ctx(), edit.response.id).into_iter().map(|op| (which, op));
            edits.typing(edit, field_ops, apply, |value, now| convert.row(now, which, value));
            if let Some(&value) = edit.event.as_ref().and_then(EditEvent::value) {
                values.push((which, value, convert.row(&base, which, value)));
            }
        }
        let outer = edits.feed(outer, &mut kept.parts);
        if let Some(gesture) = outer.id() {
            for (which, value, row) in values {
                kept.reported[which as usize] = Some((gesture, value, row));
            }
        }
        let (now, start) = (*outer.shown(), outer.start().copied());
        let event = outer.finish();
        match &event {
            Some(EditEvent::Begin { id, .. }) => kept.start = start.map(|start| (*id, start)),
            Some(EditEvent::Cancel { id, .. }) => {
                if let Some((_, start)) = kept.start.take().filter(|(of, _)| of == id) {
                    skew_v = start.skew_v;
                }
            }
            Some(e) => {
                if e.value().is_some() {
                    skew_v = now.skew_v;
                }
                if matches!(e, EditEvent::Commit { .. } | EditEvent::Set { .. }) {
                    kept.start = None;
                }
            }
            None => {}
        }

        let (submitted, escaped) = (fields.iter().any(|(_, e)| e.submitted), fields.iter().any(|(_, e)| e.escaped));
        ui.data_mut(|d| {
            d.insert_temp(kept_key, kept);
            if !host_skew {
                d.insert_temp(skew_key, skew_v);
            }
            if !host_link {
                d.insert_temp(link_key, *linked);
            }
        });
        EditResponse::new(row.response, event).submitted(submitted).escaped(escaped)
    }
}

/// A box with the fields above it and a reference point picker beside them: edit the box with the
/// fields or pick its reference point.
#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SizeUnitArg {
    #[choice(name = "%")]
    Percent,
    #[choice(name = "px")]
    Pixels,
}

story::canvas_args!(BoxArgs {
    linked: bool = true,
    #[arg(name = "size unit")]
    size_unit: SizeUnitArg = SizeUnitArg::Percent,
});

fn show_story(ui: &mut Ui, args: &mut BoxArgs) {
    let key = ui.scope_id().with("transform-fields-story");
    let rect = Rect::new(200.0, 120.0, 440.0, 280.0);
    let (mut state, mut skew_v, mut before): (BoxState, f64, Option<TransformRow>) =
        ui.data(|d| d.get_temp(key)).unwrap_or((BoxState::from_rect(rect), 0.0, None));
    let mut linked = args.linked;
    let numbers = LocaleNumbers::from_tag("en");
    let size_unit = match args.size_unit {
        SizeUnitArg::Pixels => SizeUnit::Length { x: RulerUnits::pixels(numbers.clone()), y: RulerUnits::pixels(numbers) },
        SizeUnitArg::Percent => SizeUnit::Percent,
    };
    ui.horizontal(|ui| {
        let edit = super::ReferencePoint::new("transform-fields-pivot", state.anchor_of_pivot()).show(ui);
        if let Some(EditEvent::Set { value: Some(anchor), .. }) = edit.event {
            state.pivot = state.point_at(anchor);
        }
        ui.vertical(|ui| {
            let edit = TransformFields::new("transform-fields", &state, rect, TransformFieldLabels::default())
                .size_unit(size_unit)
                .linked(&mut linked)
                .skew_v(skew_v)
                .show(ui);
            match edit.event {
                Some(EditEvent::Begin { .. }) => before = Some(TransformRow { state, skew_v }),
                Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) => {
                    (state, skew_v) = (value.state, value.skew_v)
                }
                Some(EditEvent::Cancel { .. }) => {
                    if let Some(start) = before.take() {
                        (state, skew_v) = (start.state, start.skew_v);
                    }
                }
                _ => {}
            }
        });
    });
    let canvas = story::document(ui, args.view(), "Transform fields");
    let tokens = Tokens::of(ui);
    let painter = ui.painter_at(canvas.view.rect);
    let corners = state.quad.map(|p| canvas.view.to_screen(p));
    let line = egui::Stroke::new(tokens.component.transform.line, tokens.color.overlay.line.to_egui());
    painter.add(egui::Shape::closed_line(corners.to_vec(), line));
    painter.extend(Handle::shapes(&tokens, HandleShape::Round, canvas.view.to_screen(state.pivot), ui.pixels_per_point()));
    ui.data_mut(|d| d.insert_temp(key, (state, skew_v, before)));
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["TransformFields"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("composites/transform-fields/box", Level::Composites, "TransformFields", "Box", BoxArgs::default(), show_story)
            .on(Surface::Chrome),
    ]
}

#[cfg(test)]
#[path = "transform_fields_tests.rs"]
mod tests;
