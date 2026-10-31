//! [`TransformFields`]: the options bar's X, Y, W, H, angle and skew fields of a transform box.

use craft_theme::ControlSize;
use craft_ui::testing::canvas_story::ViewArgs;
use egui::{Align, Layout, Response, Ui, UiBuilder, vec2};
use kurbo::{Rect, Vec2};

use craft_ui::canvas::quad::{BoxState, SKEW_LIMIT};
use craft_ui::canvas::{ReferencePoint, RulerUnits};
use craft_ui::composites::{PropertyLabel, PropertyRow};
use craft_ui::controls::number_field::{self, FieldRule};
use craft_ui::controls::{IconButton, NumberField, Select};
use craft_ui::direction;
use craft_ui::edit::ops::{self, ScalarOp};
use craft_ui::edit::{self, EditEvent, EditPolicy, EditResponse, Gesture, GestureId, parts};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels, LocaleNumbers, Unit, Word, Words};
use craft_ui::menu::MenuEntry;
use craft_ui::patterns::{CellWidth, OptionsBar};
use craft_ui::primitives::Handle;
use craft_ui::primitives::HandleShape;
use craft_ui::story::{Level, Story, StoryChoice, Surface};
use craft_ui::testing::canvas_story as story;
use craft_ui::theme::{ToEgui, Tokens, TokensOf};
use craft_ui::typed_story;

/// The largest position of the box's reference point, in document units (PhotoCraft).
pub const POSITION_LIMIT: f64 = 300_000.0;
/// The largest scale a W or H field takes, as a factor (1 000 000 %).
const SCALE_LIMIT: f64 = 10_000.0;
/// The largest angle.
const ANGLE_LIMIT: f64 = 180.0;

/// What a host's field parser reads besides the text: the field's current value, and for W and H
/// the field value that stands for the box's whole original side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParseContext {
    /// What the field shows now.
    pub shown: f64,
    /// For W and H: the field value of the box's original side (100 in percent, the side's length
    /// in the field's unit otherwise), so "50%" is half of it. `None` for the other fields.
    pub whole: Option<f64>,
}

/// The groups of cells [`TransformFields::show_bar`] shows; `Default` is all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarGroups {
    /// The reference point picker.
    pub reference: bool,
    /// X and Y.
    pub position: bool,
    /// W, the aspect link and H.
    pub size: bool,
    /// The angle.
    pub angle: bool,
    /// H and V skew.
    pub skew: bool,
    /// The interpolation select, when [`TransformFields::interpolation`] gives one.
    pub interpolation: bool,
}

impl BarGroups {
    pub const ALL: Self = Self { reference: true, position: true, size: true, angle: true, skew: true, interpolation: true };
}

impl Default for BarGroups {
    fn default() -> Self {
        Self::ALL
    }
}

/// The unit of the W and H fields.
// Built once per frame, so the size of the variants costs nothing.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum SizeUnit {
    /// The scale in percent of the box's size before the transform, shown and committed with up to
    /// `decimals` decimals (a parser's result is rounded to them and no further).
    Percent { decimals: usize },
    /// The box's length, in the host's units along each axis (the unit its W and H context menu
    /// picked; PhotoCraft keeps it apart from the rulers').
    Length { x: RulerUnits, y: RulerUnits },
}

impl Default for SizeUnit {
    fn default() -> Self {
        Self::Percent { decimals: 2 }
    }
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

/// The labels of [`TransformFields`]; `Default` is English, and a host passes its own translated
/// text or [`TransformFieldLabels::of`] for the words installed in the context.
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

/// One word per text of [`TransformFieldLabels`], in field order: the glyph and name of X, Y, W, H,
/// the link name, the glyph and name of Angle, skew H and skew V, then the two units.
const WORDS: [Word; 17] = [
    Word::keyed("transform.xGlyph", "X"),
    Word::keyed("transform.xName", "X position"),
    Word::keyed("transform.yGlyph", "Y"),
    Word::keyed("transform.yName", "Y position"),
    Word::keyed("transform.wGlyph", "W"),
    Word::keyed("transform.wName", "Width"),
    Word::keyed("transform.hGlyph", "H"),
    Word::keyed("transform.hName", "Height"),
    Word::keyed("transform.link", "Maintain aspect ratio"),
    Word::keyed("transform.angleGlyph", "°"),
    Word::keyed("transform.angleName", "Angle"),
    Word::keyed("transform.skewHGlyph", "H"),
    Word::keyed("transform.skewHName", "Horizontal skew"),
    Word::keyed("transform.skewVGlyph", "V"),
    Word::keyed("transform.skewVName", "Vertical skew"),
    Word::in_context("unit", "{n}%"),
    Word::in_context("unit", "{n}°"),
];

impl Labels for TransformFieldLabels {
    const WORDS: &'static [Word] = &WORDS;

    fn english() -> Self {
        Self::localized_in(Self::blank(), &Words::english())
    }

    fn localized_in(self, words: &Words) -> Self {
        let text = |i: usize| words.get(WORDS[i]).into_owned();
        let field = |i: usize| FieldLabel::new(text(i), text(i + 1));
        Self {
            x: field(0),
            y: field(2),
            w: field(4),
            h: field(6),
            link: text(8),
            angle: field(9),
            skew_h: field(11),
            skew_v: field(13),
            percent: Unit::new(&text(15)),
            degrees: Unit::new(&text(16)),
        }
    }
}

impl Default for TransformFieldLabels {
    fn default() -> Self {
        Self::english()
    }
}

impl TransformFieldLabels {
    fn blank() -> Self {
        let none = || FieldLabel::new("", "");
        Self {
            x: none(),
            y: none(),
            w: none(),
            h: none(),
            link: String::new(),
            angle: none(),
            skew_h: none(),
            skew_v: none(),
            percent: Unit::default(),
            degrees: Unit::default(),
        }
    }
}

/// One of the fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformField {
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
            SizeUnit::Percent { .. } => 100.0,
            SizeUnit::Length { x, y } => self.side(axis).max(1e-9) * if axis == 0 { x.per_doc } else { y.per_doc },
        }
    }

    /// The largest scale factor along `axis`: as far as the box stays within the position limit.
    fn limit(&self, axis: usize) -> f64 {
        let side = self.side(axis);
        if side.is_finite() && side >= 1e-6 { (POSITION_LIMIT / side).clamp(1.0, SCALE_LIMIT) } else { SCALE_LIMIT }
    }

    /// The largest value a field shows or takes.
    pub fn bound(&self, field: TransformField) -> f64 {
        match field {
            TransformField::X => POSITION_LIMIT * self.x.per_doc,
            TransformField::Y => POSITION_LIMIT * self.y.per_doc,
            TransformField::W => self.limit(0) * self.per_factor(0),
            TransformField::H => self.limit(1) * self.per_factor(1),
            TransformField::Angle => ANGLE_LIMIT,
            TransformField::SkewH | TransformField::SkewV => SKEW_LIMIT,
        }
    }

    /// What `field` shows for `state`, whose V skew is `skew_v`.
    pub fn shown(&self, state: &BoxState, skew_v: f64, field: TransformField) -> f64 {
        let parts = state.parts(self.rect, skew_v);
        match field {
            TransformField::X => state.pivot.x * self.x.per_doc,
            TransformField::Y => state.pivot.y * self.y.per_doc,
            TransformField::W => parts.w * self.per_factor(0),
            TransformField::H => parts.h * self.per_factor(1),
            TransformField::Angle => parts.angle,
            TransformField::SkewH => parts.skew_h,
            TransformField::SkewV => skew_v,
        }
    }

    /// The box and V skew after `field` is set to `value`: X and Y move the box with its reference
    /// point; the others scale, turn and skew it about the reference point. Linked, the other
    /// side of W or H follows by the same factor, each kept within its limit.
    pub fn edit(&self, state: &BoxState, skew_v: f64, field: TransformField, value: f64) -> (BoxState, f64) {
        let now = state.parts(self.rect, skew_v);
        let mut to = now;
        match field {
            TransformField::X => return (state.moved(Vec2::new(value / self.x.per_doc - state.pivot.x, 0.0)), skew_v),
            TransformField::Y => return (state.moved(Vec2::new(0.0, value / self.y.per_doc - state.pivot.y)), skew_v),
            TransformField::W | TransformField::H => {
                let axis = usize::from(field == TransformField::H);
                let factor = value / self.per_factor(axis);
                if self.linked {
                    let (side, other) = if axis == 0 { (now.w, now.h) } else { (now.h, now.w) };
                    let k = factor / nonzero(side);
                    let cap = (self.limit(axis) / nonzero(side.abs())).min(self.limit(1 - axis) / nonzero(other.abs()));
                    let capped = side * k.signum() * k.abs().min(cap);
                    (to.w, to.h) = BoxState::linked_scale((now.w, now.h), axis, capped);
                } else if axis == 0 {
                    to.w = factor;
                } else {
                    to.h = factor;
                }
            }
            TransformField::Angle => to.angle = value,
            TransformField::SkewH => to.skew_h = value,
            TransformField::SkewV => to.skew_v = value,
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
    fn row(&self, row: &TransformRow, field: TransformField, value: f64) -> TransformRow {
        let (state, skew_v) = self.edit(&row.state, row.skew_v, field, value);
        TransformRow { state, skew_v }
    }
}

/// What the row keeps between passes.
#[derive(Clone, Debug, Default)]
struct Kept {
    parts: parts::State<(TransformField, ScalarOp)>,
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
/// use craft_draw::ui::{TransformFields, TransformFieldLabels};
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
    size_menu: Option<(&'a [MenuEntry<usize>], &'a mut Option<usize>)>,
    interpolation: Option<InterpolationSelect<'a>>,
    parser: Option<FieldParser<'a>>,
    groups: BarGroups,
}

/// A host's reader of typed text for a field.
type FieldParser<'a> = Box<dyn Fn(TransformField, &str, ParseContext) -> Option<f64> + 'a>;

/// The interpolation select of the options-bar form.
struct InterpolationSelect<'a> {
    caption: String,
    value: &'a mut String,
    options: &'a [(String, String)],
}

/// How the fields are laid out.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Rows,
    Bar,
}

/// How a field's value is scrubbed: by nothing, by the caption beside it, or by an icon in it.
enum Scrub<'r> {
    No,
    Caption(&'r Response),
    Icon(Icon),
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
            size_unit: SizeUnit::default(),
            linked: None,
            skew_v: None,
            size: ControlSize::Md,
            enabled: true,
            policy: EditPolicy::default(),
            size_menu: None,
            interpolation: None,
            parser: None,
            groups: BarGroups::ALL,
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
    /// [`BoxRules::aspect_linked`](craft_ui::canvas::quad::BoxRules::aspect_linked). The toggle edits it.
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
    /// [`Debounced`](craft_ui::edit::UpdateRate::Debounced) updates or
    /// [`on_release_only`](EditPolicy::on_release_only).
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The options-bar form: a secondary click on W or H opens a menu of `entries` (the units to
    /// pick from; mark the current one with a radio row) and a pick is written to `picked`. The
    /// host applies the unit with [`size_unit`](Self::size_unit).
    pub fn size_menu(mut self, entries: &'a [MenuEntry<usize>], picked: &'a mut Option<usize>) -> Self {
        self.size_menu = Some((entries, picked));
        self
    }

    /// The options-bar form's interpolation select, last in the strip: `options` are the
    /// `(value, text)` rows, and a pick is written to `value`.
    pub fn interpolation(mut self, caption: impl Into<String>, value: &'a mut String, options: &'a [(String, String)]) -> Self {
        self.interpolation = Some(InterpolationSelect { caption: caption.into(), value, options });
        self
    }

    /// A reader for text the locale's number format doesn't read ("40px", "2in", "50%", "5+5"),
    /// tried after it, as [`NumberField::or_parse`] does. It gets the field, the text as typed, and
    /// the [`ParseContext`], and returns the field's value in its own unit (the one
    /// [`units`](Self::units) and [`size_unit`](Self::size_unit) set). Its result keeps the field's
    /// range, rounding and rejection of values that aren't finite.
    pub fn parser(mut self, parser: impl Fn(TransformField, &str, ParseContext) -> Option<f64> + 'a) -> Self {
        self.parser = Some(Box::new(parser));
        self
    }

    /// The groups [`show_bar`](Self::show_bar) shows (all by default). A divider separates the
    /// groups shown: the reference point and position, the size, the angle and skew, and the
    /// interpolation select. The fields shown keep their ids and events.
    pub fn groups(mut self, groups: BarGroups) -> Self {
        self.groups = groups;
        self
    }

    /// Shows the fields as a [`PropertyRow`]s and returns this frame's edit, with the box it gives.
    pub fn show(self, ui: &mut Ui) -> EditResponse<TransformRow> {
        self.show_in(ui, Form::Rows)
    }

    /// Shows the fields as the cells of an options bar and returns this frame's edit. The cells
    /// are the reference point, X and Y, a divider, W, the aspect link and H, a divider, the angle
    /// (scrubbed by an icon), H and V skew, and with [`interpolation`](Self::interpolation) a
    /// divider and the select. A pick on the reference point reports a Set of the box with
    /// its reference point moved. The cells are [`Sm`](ControlSize::Sm) fields whatever
    /// [`size`](Self::size) says. Put it in the bar's body.
    pub fn show_bar(self, ui: &mut Ui) -> EditResponse<TransformRow> {
        self.show_in(ui, Form::Bar)
    }

    fn show_in(self, ui: &mut Ui, form: Form) -> EditResponse<TransformRow> {
        let Self { id_salt, state, rect, labels, x, y, size_unit, linked, skew_v, size, enabled, policy, size_menu, interpolation, parser, groups } =
            self;
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
        let mut edits = parts::Edits::<TransformRow, (TransformField, ScalarOp)>::new(id, &kept.parts, &outer, scoped);
        let inner = EditPolicy::default().cancellable(policy.cancellable).cost(policy.cost);
        let tokens = Tokens::of(ui);
        let row_style = PropertyRow::style(&tokens, size);
        let cell = row_style.glyph_width + row_style.label_gap + NumberField::style(&tokens, size).field.width;
        let pair = 2.0 * cell + row_style.column_gap;
        let linked_pair = pair + row_style.link_width + 2.0 * row_style.column_gap;

        // Each field's unit and decimals.
        let format = |which: TransformField| match (which, &convert.size) {
            (TransformField::X, _) => (convert.x.unit.clone(), convert.x.decimals),
            (TransformField::Y, _) => (convert.y.unit.clone(), convert.y.decimals),
            (TransformField::W, SizeUnit::Length { x, .. }) => (x.unit.clone(), x.decimals),
            (TransformField::H, SizeUnit::Length { y, .. }) => (y.unit.clone(), y.decimals),
            (TransformField::W | TransformField::H, SizeUnit::Percent { decimals }) => (labels.percent.clone(), *decimals),
            _ => (labels.degrees.clone(), 2),
        };
        let rule = |which: TransformField| {
            let bound = convert.bound(which);
            FieldRule::new(&(-bound..=bound), 1.0, format(which).1)
        };
        // What a field shows on `row`: the part of the box it reads, or the value it gave while
        // the box is the one that value made.
        let reported = kept.reported;
        let reads = |which: TransformField, row: &TransformRow| match reported[which as usize] {
            Some((_, value, of)) if of == *row => value,
            _ => convert.shown(&row.state, row.skew_v, which),
        };
        // A field's operation applies to what the field shows on the row of its turn.
        let apply = |(which, op): &(TransformField, ScalarOp), now: &TransformRow| {
            Some(convert.row(now, *which, number_field::apply(op, reads(*which, now), &rule(*which))))
        };
        edits.replay(ui.ctx(), apply);
        let shown = *edits.value();

        let field = |ui: &mut Ui,
                     which: TransformField,
                     label: &FieldLabel,
                     size: ControlSize,
                     scrub: Scrub,
                     menu: Option<(&[MenuEntry<usize>], &mut Option<usize>)>| {
            let (unit, decimals) = format(which);
            let bound = convert.bound(which);
            let current = reads(which, &shown);
            let mut field = NumberField::new((id, which as u8), label.name.clone(), current, &numbers)
                .range(-bound..=bound)
                .decimals(0..=decimals)
                .unit(unit)
                .policy(inner)
                .part(true)
                .size(size)
                .enabled(enabled);
            if let Some(parser) = &parser {
                let whole =
                    matches!(which, TransformField::W | TransformField::H).then(|| convert.per_factor(usize::from(which == TransformField::H)));
                field = field.or_parse(move |text| parser(which, text, ParseContext { shown: current, whole }));
            }
            field = match scrub {
                Scrub::No => field,
                Scrub::Caption(caption) => field.scrub_by(caption),
                Scrub::Icon(icon) => field.scrub_icon(icon),
            };
            if let Some((entries, picked)) = menu {
                field = field.unit_menu(entries, picked);
            }
            field.show(ui)
        };
        let plain = |ui: &mut Ui, which: TransformField, label: &FieldLabel| field(ui, which, label, size, Scrub::No, None);
        let glyph = |label: &FieldLabel| PropertyLabel::glyph(label.glyph.clone());
        let group = |ui: &mut Ui, width: f32, add: &mut dyn FnMut(&mut Ui)| {
            ui.allocate_ui(vec2(width, 0.0), |ui| {
                ui.set_max_width(width);
                add(ui);
            });
        };

        let layout = if direction::is_rtl(ui) { Layout::right_to_left(Align::Min) } else { Layout::left_to_right(Align::Min) }.with_main_wrap(true);
        let rows = |ui: &mut Ui, fields: &mut Vec<(TransformField, EditResponse<f64>)>, linked: &mut bool| {
            group(ui, pair, &mut |ui| {
                let (a, b) = PropertyRow::pair(glyph(&labels.x), glyph(&labels.y))
                    .size(size)
                    .show(ui, |ui, _| plain(ui, TransformField::X, &labels.x), |ui, _| plain(ui, TransformField::Y, &labels.y))
                    .inner;
                fields.extend([(TransformField::X, a), (TransformField::Y, b)]);
            });
            group(ui, linked_pair, &mut |ui| {
                let (a, _, b) = PropertyRow::pair(glyph(&labels.w), glyph(&labels.h))
                    .size(size)
                    .show_linked(
                        ui,
                        |ui, _| plain(ui, TransformField::W, &labels.w),
                        |ui| {
                            let icon = if *linked { Icon::Link } else { Icon::Unlink };
                            ui.add(IconButton::new(icon, labels.link.clone()).size(ControlSize::Sm).enabled(enabled).toggle(linked))
                        },
                        |ui, _| plain(ui, TransformField::H, &labels.h),
                    )
                    .inner;
                fields.extend([(TransformField::W, a), (TransformField::H, b)]);
            });
            group(ui, cell, &mut |ui| {
                let edit = PropertyRow::new(glyph(&labels.angle)).size(size).show(ui, |ui, _| plain(ui, TransformField::Angle, &labels.angle)).inner;
                fields.push((TransformField::Angle, edit));
            });
            group(ui, pair, &mut |ui| {
                let (a, b) = PropertyRow::pair(glyph(&labels.skew_h), glyph(&labels.skew_v))
                    .size(size)
                    .show(ui, |ui, _| plain(ui, TransformField::SkewH, &labels.skew_h), |ui, _| plain(ui, TransformField::SkewV, &labels.skew_v))
                    .inner;
                fields.extend([(TransformField::SkewH, a), (TransformField::SkewV, b)]);
            });
        };
        let mut anchor = None;
        let mut size_pick = None;
        let bar_style = OptionsBar::style(&tokens, false);
        let bar = |ui: &mut Ui, fields: &mut Vec<(TransformField, EditResponse<f64>)>, linked: &mut bool| {
            if groups.reference {
                let reference = ReferencePoint::new((id, "reference"), state.anchor_of_pivot()).enabled(enabled).show(ui);
                if let Some(EditEvent::Set { value: Some(picked), modifiers, .. }) = reference.event {
                    anchor = Some((picked, modifiers));
                }
            }
            let sm = ControlSize::Sm;
            let wide = if matches!(convert.size, SizeUnit::Length { .. }) { CellWidth::Lg } else { CellWidth::Md };
            let menu = size_menu.as_ref().map(|(entries, _)| *entries);
            let cell = |ui: &mut Ui,
                        which: TransformField,
                        label: &FieldLabel,
                        width: CellWidth,
                        menu: Option<&[MenuEntry<usize>]>,
                        pick: &mut Option<usize>| {
                OptionsBar::cell(ui, &label.glyph, width, |ui, caption| {
                    field(ui, which, label, sm, Scrub::Caption(caption), menu.map(|entries| (entries, pick)))
                })
            };
            let mut none = None;
            // A divider separates the blocks shown; the reference point and position, and the angle
            // and skew, are one block each.
            let any = std::cell::Cell::new(groups.reference);
            let block = |ui: &mut Ui, shown: bool| {
                if shown && any.replace(true) {
                    OptionsBar::divider(ui);
                }
                shown
            };
            if groups.position {
                for (which, label) in [(TransformField::X, &labels.x), (TransformField::Y, &labels.y)] {
                    fields.push((which, cell(ui, which, label, CellWidth::Md, None, &mut none)));
                }
                any.set(true);
            }
            if block(ui, groups.size) {
                fields.push((TransformField::W, cell(ui, TransformField::W, &labels.w, wide, menu, &mut size_pick)));
                let icon = if *linked { Icon::Link } else { Icon::Unlink };
                ui.add(IconButton::new(icon, labels.link.clone()).size(sm).enabled(enabled).toggle(linked));
                fields.push((TransformField::H, cell(ui, TransformField::H, &labels.h, wide, menu, &mut size_pick)));
            }
            if block(ui, groups.angle || groups.skew) {
                if groups.angle {
                    // The angle has no caption: its icon scrubs it, in a box as wide as a captioned cell.
                    let side = bar_style.fields[0];
                    let turn = ui
                        .allocate_ui_with_layout(vec2(side, ui.available_height()), direction::row(ui).with_main_justify(true), |ui| {
                            ui.set_width(side);
                            field(ui, TransformField::Angle, &labels.angle, sm, Scrub::Icon(Icon::RotateCw), None)
                        })
                        .inner;
                    fields.push((TransformField::Angle, turn));
                }
                if groups.skew {
                    for (which, label) in [(TransformField::SkewH, &labels.skew_h), (TransformField::SkewV, &labels.skew_v)] {
                        fields.push((which, cell(ui, which, label, CellWidth::Sm, None, &mut none)));
                    }
                }
            }
            if let Some(InterpolationSelect { caption, value, options }) = interpolation.filter(|_| block(ui, groups.interpolation)) {
                OptionsBar::cell(ui, &caption, CellWidth::Content, |ui, _| {
                    let select = options
                        .iter()
                        .fold(Select::new((id, "interpolation"), caption.clone(), value), |select, (v, text)| select.option(v.clone(), text.clone()));
                    let _ = select.size(sm).enabled(enabled).show(ui);
                });
            }
        };

        let mut fields: Vec<(TransformField, EditResponse<f64>)> = Vec::with_capacity(FIELDS);
        let row = match form {
            Form::Rows => ui.scope_builder(UiBuilder::new().layout(layout), |ui| rows(ui, &mut fields, linked)).response,
            Form::Bar => ui.scope_builder(UiBuilder::new().layout(direction::row(ui)), |ui| bar(ui, &mut fields, linked)).response,
        };
        if let (Some((_, picked)), Some(pick)) = (size_menu, size_pick) {
            *picked = Some(pick);
        }
        // Each field's value applies to the row the fields before it in this pass left, with the
        // values that go out in this pass: one that waits for another field's gesture isn't in it.
        // A run's every Update and its Commit are made from the row the run began from: scaling
        // and skewing a perspective box about a pivot don't compose, so the row an earlier Update
        // left would drift the result.
        let run_start = outer.start().copied();
        let mut values: Vec<(TransformField, f64, TransformRow)> = Vec::new();
        for (which, edit) in &fields {
            let (which, base) = (*which, *edits.value());
            let origin = |now: &TransformRow| run_start.filter(|_| edit.event.as_ref().is_some_and(|e| e.id().is_some())).unwrap_or(*now);
            let field_ops = ops::take::<ScalarOp>(ui.ctx(), edit.response.id).into_iter().map(|op| (which, op));
            edits.typing(edit, field_ops, apply, |value, now| convert.row(&origin(now), which, value));
            if let Some(&value) = edit.event.as_ref().and_then(EditEvent::value) {
                values.push((which, value, convert.row(&origin(&base), which, value)));
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
        let event = event.or_else(|| {
            let (picked, modifiers) = anchor?;
            Some(EditEvent::set(TransformRow { state: BoxState { pivot: state.point_at(picked), ..*state }, skew_v }, modifiers))
        });
        EditResponse::new(row, event).submitted(submitted).escaped(escaped)
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

#[derive(craft_ui::story::StoryArgs, Debug, PartialEq)]
struct BoxArgs {
    #[arg(flatten)]
    view: ViewArgs,
    linked: bool,
    #[arg(name = "size unit")]
    size_unit: SizeUnitArg,
}

impl Default for BoxArgs {
    fn default() -> Self {
        Self { view: ViewArgs::default(), linked: true, size_unit: SizeUnitArg::Percent }
    }
}

fn show_story(ui: &mut Ui, args: &mut BoxArgs) {
    let key = ui.scope_id().with("transform-fields-story");
    let rect = Rect::new(200.0, 120.0, 440.0, 280.0);
    let (mut state, mut skew_v, mut before): (BoxState, f64, Option<TransformRow>) =
        ui.data(|d| d.get_temp(key)).unwrap_or((BoxState::from_rect(rect), 0.0, None));
    let mut linked = args.linked;
    let numbers = LocaleNumbers::from_tag("en");
    let size_unit = match args.size_unit {
        SizeUnitArg::Pixels => SizeUnit::Length { x: RulerUnits::pixels(numbers.clone()), y: RulerUnits::pixels(numbers) },
        SizeUnitArg::Percent => SizeUnit::default(),
    };
    ui.horizontal(|ui| {
        let edit = ReferencePoint::new("transform-fields-pivot", state.anchor_of_pivot()).show(ui);
        if let Some(EditEvent::Set { value: Some(anchor), .. }) = edit.event {
            state.pivot = state.point_at(anchor);
        }
        ui.vertical(|ui| {
            let edit = TransformFields::new("transform-fields", &state, rect, TransformFieldLabels::of(ui))
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
    let canvas = story::document(ui, args.view, "Transform fields");
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
