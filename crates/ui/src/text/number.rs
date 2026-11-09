//! `NumberField`: a text part over [`NumberCodec`] with a range, a step, a unit, presets and a popup slider.
//!
//! Typing, expressions, unit spellings and locale digits are the codec's (`quantity::Reader`); the field adds the
//! range every written value stays in and the step that arrows, the wheel and the label scrub move by.

#[cfg(feature = "testing")]
mod stories;

use super::adorn::{Adornment, Placed, place};
use super::words::TextWords;
use super::{FieldStyle, text_part};
use crate::buttons::IconButton;
use crate::contract::{
    Access, Bind, BindMut, CollectionEvent, Common, Component, Cx, Edit as _, EditResponse, Entry, Focus as _, Input, MeasureCx, Numeric, Outcome,
    Overlay, OverlayUi, Overlays as _, PaintUi, PartsCx, Pointer as _, Props as _, Role, Scrub, Show as _, ShowCollection as _, Styled as _,
    WordSetCx as _,
};
use crate::list::{List, ListRow};
use crate::primitives::{IconSpec, TextOptions, TextRun};
use crate::runtime::env::{Label, LengthUnits, UnitKind, Units};
use crate::style::{ComponentStyle, Ink, StyleCx, SurfaceLook, TextLook, TypeRole};
use crate::value_drag::Slider;
use craft_behavior::Value;
use craft_behavior::nav::{LARGE_STEPS, StepResult, ValueAxes, ValueSteps};
use craft_behavior::{
    Codec, DisplayOrder, Flow, HitLevel, HitPriority, IconId, ItemSource, Metrics, MoveMods, NavMove, NotAValue, NumberCodec, PartKey, Selection,
    Sense, ValueDrag, decimals_for_step, wheel_step,
};
use craft_theme::Stateful;
use egui::{Pos2, Rect, Response, Vec2};
use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::{Deref, RangeInclusive};

/// The step of an arrow press, the wheel and the label scrub when the field sets none.
const DEFAULT_STEP: f64 = 1.0;
/// The decimals a field writes when neither a codec nor a unit kind says.
const DEFAULT_DECIMALS: usize = 3;
/// How far a value may sit off a multiple of the step and still count as on it.
const ON_GRID: f64 = 1e-9;

/// The chevron that opens the presets list.
const PRESETS: PartKey = PartKey::new("presets");
/// The chevron that opens the popup slider.
const SLIDER: PartKey = PartKey::new("slider");

/// The chevron's icon.
const CHEVRON: &str = "chevron_down";

/// What a presets row picks: a number the field takes, or a choice that is not a number (Auto, Metrics) and
/// leaves the value alone.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum NumberPick {
    /// A pick sets the field to this number. The row is checked while the field holds it.
    Number(f64),
    /// A pick leaves the value alone and `NumberOutput::preset` names it. The row is checked by its entry's
    /// `checked`.
    Choice(u32),
}

/// A value a number field holds: a number, or a number that may also be a name from the presets list.
/// `Option<f64>` is the plain value (empty text is `None`); [`NamedOr`] adds named entries.
pub trait NumberValue: Value {
    /// The number the value holds; `None` for a blank value or a name.
    fn number(&self) -> Option<f64>;
    /// The value that holds `number`.
    fn of_number(number: f64) -> Self;
    /// The value empty text stands for; `None` when a blank is not a value of this type.
    fn blank() -> Option<Self>;
    /// The id of the `NumberPick::Choice` row the value is, `None` for a number.
    fn named(&self) -> Option<u32>;
    /// The value that is the name behind a `NumberPick::Choice` row; `None` when this type holds no names.
    fn from_name(id: u32) -> Option<Self>;
}

impl NumberValue for Option<f64> {
    fn number(&self) -> Option<f64> {
        *self
    }

    fn of_number(number: f64) -> Self {
        Some(number)
    }

    fn blank() -> Option<Self> {
        Some(None)
    }

    fn named(&self) -> Option<u32> {
        None
    }

    fn from_name(_: u32) -> Option<Self> {
        None
    }
}

/// A name a [`NamedOr`] value holds, tied to a `NumberPick::Choice` id of the field's presets.
pub trait NumberName: Value + Copy {
    /// The id of the `NumberPick::Choice` row that names it.
    fn id(self) -> u32;
    /// The name behind a row id, `None` for an id that names nothing.
    fn from_id(id: u32) -> Option<Self>;
}

/// A number or a name from the presets list (kerning: Metrics, Optical or a number). Empty text is not a value.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum NamedOr<K> {
    /// A name; the presets row `NumberPick::Choice(name.id())` shows its label.
    Named(K),
    /// A number.
    Number(f64),
}

impl<K: NumberName> NumberValue for NamedOr<K> {
    fn number(&self) -> Option<f64> {
        match self {
            NamedOr::Number(number) => Some(*number),
            NamedOr::Named(_) => None,
        }
    }

    fn of_number(number: f64) -> Self {
        NamedOr::Number(number)
    }

    fn blank() -> Option<Self> {
        None
    }

    fn named(&self) -> Option<u32> {
        match self {
            NamedOr::Named(name) => Some(name.id()),
            NamedOr::Number(_) => None,
        }
    }

    fn from_name(id: u32) -> Option<Self> {
        K::from_id(id).map(NamedOr::Named)
    }
}

/// What a number field reports: the edit and the preset row picked this pass.
#[non_exhaustive]
pub struct NumberOutput<V: NumberValue = Option<f64>> {
    /// The edit response of the value.
    pub edit: EditResponse<V>,
    /// The presets row the user picked this pass.
    pub preset: Option<NumberPick>,
}

impl<V: NumberValue> Deref for NumberOutput<V> {
    type Target = EditResponse<V>;

    fn deref(&self) -> &Self::Target {
        &self.edit
    }
}

impl<V: NumberValue> Outcome for NumberOutput<V> {
    type Value = V;

    fn response(&self) -> &Response {
        &self.edit.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        self.edit.story_facts()
    }
}

/// The value one step from a value in a direction: the up arrow, the wheel and the Page keys of a field whose values
/// lie on a ladder instead of a grid (`NumberField::stepper`). The direction is `1` up and `-1` down.
pub struct NumberStepper<'a>(Box<dyn Fn(f64, i32) -> f64 + 'a>);

impl<'a> NumberStepper<'a> {
    /// The stepper that runs `next`.
    pub fn new(next: impl Fn(f64, i32) -> f64 + 'a) -> Self {
        NumberStepper(Box::new(next))
    }

    fn next(&self, from: f64, direction: i32) -> f64 {
        (self.0)(from, direction)
    }
}

/// The range and step values are held to.
#[derive(Clone, Debug, PartialEq)]
struct Rule {
    low: f64,
    high: f64,
    step: f64,
}

impl Rule {
    fn new(range: Option<&RangeInclusive<f64>>, step: Option<f64>) -> Self {
        let (a, b) = range.map_or((f64::NEG_INFINITY, f64::INFINITY), |range| (*range.start(), *range.end()));
        let step = step.filter(|step| step.is_finite() && *step > 0.0).unwrap_or(DEFAULT_STEP);
        Rule { low: a.min(b), high: a.max(b), step }
    }

    /// `value` kept in the range.
    fn settle(&self, value: f64) -> f64 {
        value.clamp(self.low, self.high)
    }

    /// Where a blank field steps from: 0, kept in the range.
    fn blank(&self) -> f64 {
        self.settle(0.0)
    }

    /// The finite ends of the range.
    fn bounds(&self) -> RangeInclusive<f64> {
        self.settle(f64::MIN)..=self.settle(f64::MAX)
    }

    /// The value `notches` steps of `scale` times the step from `from`: a value off the grid of that size first goes
    /// to the next multiple in the direction of the move (by 1, 5.1 goes up to 6 and down to 5).
    fn stepped(&self, from: f64, notches: i32, scale: f64) -> f64 {
        let size = self.step * scale;
        let at = from / size;
        let nearest = at.round();
        let n = f64::from(notches);
        let multiple = if (at - nearest).abs() < ON_GRID {
            nearest + n
        } else if notches > 0 {
            at.ceil() + n - 1.0
        } else {
            at.floor() + n + 1.0
        };
        self.settle(round_to(multiple * size, decimals_for_step(size)))
    }

    /// `value` on the nearest multiple of the step.
    fn snap(&self, value: f64) -> f64 {
        round_to((value / self.step).round() * self.step, decimals_for_step(self.step))
    }
}

/// `value` rounded to `decimals` decimal places.
fn round_to(value: f64, decimals: usize) -> f64 {
    let scale = 10f64.powi(i32::try_from(decimals).unwrap_or(0));
    (value * scale).round() / scale
}

/// The scale of a step by an arrow key: Shift moves `LARGE_STEPS` steps, Alt a share of one, both together one.
fn key_scale(mods: MoveMods) -> f64 {
    match (mods.extend, mods.fine) {
        (true, false) => LARGE_STEPS,
        (false, true) => 1.0 / LARGE_STEPS,
        _ => 1.0,
    }
}

/// Whether `text` is a plain number rather than an expression or a relative input, which apply on commit only.
fn is_plain(text: &str) -> bool {
    let text = text.trim();
    !text.starts_with(['+', '*', '/']) && text.chars().skip(1).all(|c| !matches!(c, '+' | '-' | '*' | '/' | '(' | ')'))
}

/// The field's codec with what a committed value is held to: the range, the step grid when typed values snap, and
/// the decimals a committed value is rounded to. A plain number applies as it is typed unless typed values snap,
/// which apply on commit only. While the field shows no value (a mixed binding) typed numbers apply on commit only,
/// and empty text is not a value: it would clear every target, so the draft stays open until Escape or a focus
/// loss restores the mixed display. Text that spells a name of the presets (case ignored) is that name.
struct Ruled<V> {
    inner: NumberCodec,
    rule: Rule,
    snap: bool,
    decimals: Option<usize>,
    mixed: bool,
    names: Vec<(u32, String)>,
    /// How many shown units make one stored unit (1 for a field that shows what it holds).
    scale: f64,
    value: PhantomData<fn() -> V>,
}

impl<V> Clone for Ruled<V> {
    fn clone(&self) -> Self {
        Ruled {
            inner: self.inner.clone(),
            rule: self.rule.clone(),
            snap: self.snap,
            decimals: self.decimals,
            mixed: self.mixed,
            names: self.names.clone(),
            scale: self.scale,
            value: PhantomData,
        }
    }
}

impl<V: NumberValue> Ruled<V> {
    fn held(&self, number: f64) -> f64 {
        let snapped = if self.snap { self.rule.snap(number) } else { number };
        self.rule.settle(self.decimals.map_or(snapped, |decimals| round_to(snapped, decimals)))
    }

    /// The number the value holds, as the field shows it.
    fn shown(&self, v: &V) -> Option<f64> {
        v.number().map(|number| number * self.scale)
    }

    /// The name `text` spells, as a value of the field's type.
    fn name_of(&self, text: &str) -> Option<V> {
        let text = text.trim();
        self.names.iter().find(|(_, name)| name.eq_ignore_ascii_case(text)).and_then(|(id, _)| V::from_name(*id))
    }
}

impl<V: NumberValue> Codec for Ruled<V> {
    type Value = V;

    fn format(&self, v: &V) -> String {
        // A mixed field of a type with no blank value is handed a placeholder number; it shows empty.
        if self.mixed && V::blank().is_none() {
            return String::new();
        }
        match v.named().and_then(|id| self.names.iter().find(|(name, _)| *name == id)) {
            Some((_, label)) => label.clone(),
            None => self.inner.format(&self.shown(v)),
        }
    }

    fn parse(&self, text: &str, start: &V) -> Result<V, NotAValue> {
        if let Some(name) = self.name_of(text) {
            return Ok(name);
        }
        if text.trim().is_empty() && (self.mixed || V::blank().is_none()) {
            return Err(NotAValue);
        }
        let read = self.inner.parse(text, &self.shown(start))?;
        match read {
            Some(number) => Ok(V::of_number(self.held(number) / self.scale)),
            None => V::blank().ok_or(NotAValue),
        }
    }

    fn applies_as_typed(&self, text: &str) -> bool {
        !self.snap && !self.mixed && is_plain(text) && matches!(self.inner.parse(text, &None), Ok(Some(_)))
    }

    fn filter(&self, c: char) -> bool {
        self.inner.filter(c) || (!self.names.is_empty() && c.is_alphabetic())
    }

    fn normalise(&self, text: &str, v: &V) -> String {
        match v.named() {
            Some(_) => self.format(v),
            None => self.inner.normalise(text, &self.shown(v)),
        }
    }

    fn caret_on_focus(&self, text: &str) -> craft_behavior::Caret {
        self.inner.caret_on_focus(text)
    }

    fn sample(&self, current: &V) -> Cow<'_, str> {
        match current.named() {
            Some(_) => Cow::Owned(self.format(current)),
            None => self.inner.sample(&self.shown(current)),
        }
    }
}

/// The style of a number field: the field look, which the chevrons are laid out by, and the unit's text.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct NumberStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    /// The unit shown after the number.
    #[style(text)]
    unit: TextLook,
    /// The field look the adornments are placed by; `surface` and `metrics` are its own.
    field: FieldStyle,
}

/// A number field: a text part over [`NumberCodec`]; empty text is `None`.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(overlay, probe(NumberField<'static, Option<f64>>))]
pub struct NumberField<'a, V: NumberValue + 'static = Option<f64>> {
    #[common]
    common: Common,
    value: Bind<'a, V>,
    /// The codec, to read another locale's separators or units, or to read words such as "Auto" (`or_parse`).
    /// Without one the field reads and writes with the env's locale and the spellings of `unit_kind`.
    #[prop]
    codec: Option<NumberCodec>,
    /// What the number counts: typed text may carry this kind's unit spellings ("12 mm") in the user's language or
    /// English, and the field shows at most the kind's decimals.
    #[prop]
    unit_kind: Option<UnitKind>,
    /// The range typed, stepped and scrubbed values stay in.
    #[prop]
    range: Option<RangeInclusive<f64>>,
    /// What one arrow press, one wheel notch and one point of label scrub change; the default is 1.
    #[prop(quantity = "step")]
    step: Option<f64>,
    /// Where an arrow press, a Page key and a wheel notch go from the current value, for values on a ladder (the zoom
    /// ladder); the step is ignored for them. The range still holds the result.
    #[prop(func = "Fn(f64, i32) -> f64")]
    stepper: Option<NumberStepper<'a>>,
    /// A unit shown after the number.
    #[prop(into)]
    unit: Option<Label>,
    /// The rows of the list a chevron after the field opens; Alt+Down opens it too.
    #[prop]
    presets: Option<&'a [Entry<NumberPick>]>,
    /// A chevron after the field opens a slider over `range` below it.
    #[prop]
    popup_slider: Option<RangeInclusive<f64>>,
    /// A typed value goes to the nearest multiple of the step when it commits.
    #[prop]
    snap_typed: bool,
    /// The decimals a committed value is rounded to; the display keeps the codec's.
    #[prop]
    commit_decimals: Option<usize>,
    /// The lengths a length field keeps and shows (`NumberField::length`).
    lengths: Option<LengthUnits>,
    /// How many shown units make one stored unit, read from the env in measure and parts; 1 for a plain field.
    scale: f64,
    options: TextOptions,
    scrub: bool,
    picked: Option<NumberPick>,
    unit_size: Vec2,
}

impl<'a> NumberField<'a, Option<f64>> {
    /// A number field named `label`; empty text is `None`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, Option<f64>>) -> Self {
        Self::of(label, value)
    }

    /// A number field over a length the binding keeps in `lengths.stored` (the document's unit) and shows in
    /// `lengths.shown` (the user's unit), after the shown unit's symbol. Typed text reads any pixel or length unit
    /// spelling ("10 mm" in a field that shows inches) and a bare number in the shown unit; the binding, its check
    /// and its edit events are in the stored unit. `range` is in the stored unit and `popup_slider` and presets
    /// numbers too; the step is in the shown unit and defaults to the shown unit's own.
    pub fn length(label: impl Into<Label>, value: Bind<'a, Option<f64>>, lengths: LengthUnits) -> Self {
        let mut field = Self::of(label, value);
        field.lengths = Some(lengths);
        field.unit_kind = Some(lengths.shown);
        field.step = Some(lengths.shown.step());
        field
    }
}

impl<'a, V: NumberValue + 'static> NumberField<'a, V> {
    /// A number field named `label` over a value of another type, such as [`NamedOr`].
    pub fn named(label: impl Into<Label>, value: Bind<'a, V>) -> Self {
        Self::of(label, value)
    }

    fn of(label: impl Into<Label>, value: Bind<'a, V>) -> Self {
        NumberField {
            common: Common::new(),
            value,
            codec: None,
            unit_kind: None,
            range: None,
            step: None,
            stepper: None,
            unit: None,
            presets: None,
            popup_slider: None,
            snap_typed: false,
            commit_decimals: None,
            lengths: None,
            scale: 1.0,
            options: TextOptions::default(),
            scrub: false,
            picked: None,
            unit_size: Vec2::ZERO,
        }
        .label(label)
    }

    /// A drag on the field's side label scrubs the value.
    #[must_use]
    pub fn scrub_by_label(mut self) -> Self {
        self.scrub = true;
        self
    }

    /// The range and step in the shown unit.
    fn rule(&self) -> Rule {
        let range = self.range.as_ref().map(|range| range.start() * self.scale..=range.end() * self.scale);
        Rule::new(range.as_ref(), self.step)
    }

    /// Reads what a length field keeps from the env: the scale from stored to shown, and the shown unit's symbol
    /// after the number unless the field sets a unit.
    fn settle_lengths(&mut self, units: &Units) {
        let Some(lengths) = self.lengths else { return };
        self.scale = units.length_scale(lengths).unwrap_or(1.0);
        if self.unit.is_none() {
            self.unit = Some(units.symbol(lengths.shown).into());
        }
    }

    /// `stored` in the shown unit.
    fn to_shown(&self, stored: f64) -> f64 {
        stored * self.scale
    }

    /// `shown` in the stored unit.
    fn to_stored(&self, shown: f64) -> f64 {
        shown / self.scale
    }

    /// `stored` kept in the range.
    fn settle_stored(&self, stored: f64) -> f64 {
        self.to_stored(self.rule().settle(self.to_shown(stored)))
    }

    /// The number the field holds, in the shown unit.
    fn shown_number(&self, value: Option<&V>) -> Option<f64> {
        value.and_then(NumberValue::number).map(|stored| self.to_shown(stored))
    }

    /// The field's codec: the one set, else the env's locale and the spellings of the unit kind (a length field
    /// reads every length spelling).
    fn number_codec(&self, units: &Units) -> NumberCodec {
        self.codec.clone().unwrap_or_else(|| {
            let reader = match self.lengths {
                Some(lengths) => units.length_reader(lengths),
                None => units.reader(self.unit_kind),
            };
            NumberCodec::new(reader, self.decimals())
        })
    }

    /// The most decimals the field writes.
    fn decimals(&self) -> usize {
        self.codec.as_ref().map_or_else(|| self.unit_kind.map_or(DEFAULT_DECIMALS, |kind| *kind.decimals().end()), NumberCodec::decimals)
    }

    /// The names the presets list holds that a value of this type can be: the labels of its `Choice` rows.
    fn names(&self, label: impl Fn(&Entry<NumberPick>) -> Label) -> Vec<(u32, String)> {
        let rows = self.presets.unwrap_or_default();
        rows.iter()
            .filter_map(|row| match row.value() {
                Some(NumberPick::Choice(id)) if V::from_name(*id).is_some() => Some((*id, label(row).as_str().to_owned())),
                _ => None,
            })
            .collect()
    }

    fn ruled(&self, units: &Units, mixed: bool, names: Vec<(u32, String)>) -> Ruled<V> {
        Ruled {
            inner: self.number_codec(units),
            rule: self.rule(),
            snap: self.snap_typed,
            decimals: self.commit_decimals,
            mixed,
            names,
            scale: self.scale,
            value: PhantomData,
        }
    }

    /// The unit's size, the room it takes and the gap before it.
    fn unit_extent(&self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        self.unit.as_ref().map_or(Vec2::ZERO, |unit| cx.layout(&TextRun::new(unit.as_str()).role(cx.style.unit.role)).size())
    }

    /// The chevrons after the text, in order, with their parts and accessible names.
    fn chevrons(&self, words: &TextWords) -> Vec<(PartKey, Label)> {
        let mut chevrons = Vec::new();
        if self.presets.is_some() {
            chevrons.push((PRESETS, words.presets.clone()));
        }
        if self.popup_slider.is_some() {
            chevrons.push((SLIDER, words.slider.clone()));
        }
        chevrons
    }

    fn adornments(chevrons: &[(PartKey, Label)]) -> Vec<Adornment> {
        chevrons.iter().map(|(key, name)| Adornment::button(*key, IconSpec::svg(IconId::new(CHEVRON)), name.clone())).collect()
    }

    /// The rect of the text and the chevrons inside `rect`.
    fn laid_out(style: &NumberStyle, rect: Rect, rtl: bool, chevrons: &[(PartKey, Label)]) -> Placed {
        place(rect, &style.field, &[], &Self::adornments(chevrons), rtl)
    }

    /// The text rect and the unit's: the unit sits between the text and the chevrons.
    fn text_and_unit(&self, text: Rect, rtl: bool, gap: f32) -> (Rect, Option<Rect>) {
        if self.unit_size.x <= 0.0 {
            return (text, None);
        }
        let (unit, rest) = Flow::new(text, rtl).take_end(self.unit_size.x + gap);
        (rest.rect(), Some(unit))
    }

    fn show_presets(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>, rows: &[Entry<NumberPick>]) {
        let popup = cx.popup("presets");
        let labels: Vec<Label> =
            rows.iter().map(|row| if row.is_separator() { Label::Empty } else { cx.env.resolve(&row.label().clone()) }).collect();
        let current = cx.value().and_then(NumberValue::number);
        let named = cx.value().and_then(NumberValue::named);
        let held = |at: usize| match rows[at].value() {
            Some(NumberPick::Number(value)) => current.is_some_and(|current| (current - value).abs() <= 1e-4 * current.abs().max(1.0)),
            Some(NumberPick::Choice(id)) => rows[at].is_checked() || named == Some(*id),
            None => false,
        };
        let marked = (0..rows.len()).find(|at| held(*at));
        let name = cx.label();
        let events = ui.show(popup, Overlay::dropdown(), |ui, _| {
            let selection = marked.map_or_else(Selection::default, |at| Selection::new(vec![at], Some(at), Some(at)));
            let source = Presets { rows, labels: &labels };
            List::<usize>::options(name.clone())
                .selection(Bind::given(selection))
                .show_items(ui, &source, &mut |at, _| {
                    if rows[*at].is_separator() {
                        ListRow::spacer()
                    } else {
                        ListRow::new(labels[*at].clone()).enabled(rows[*at].is_enabled()).option()
                    }
                })
                .events
        });
        let picked = events.into_iter().flatten().find_map(|event| match event {
            CollectionEvent::Activate(at) => rows[at].value().copied(),
            _ => None,
        });
        if let Some(pick) = picked {
            match pick {
                NumberPick::Number(value) => cx.set(V::of_number(self.settle_stored(value))),
                NumberPick::Choice(id) => {
                    if let Some(name) = V::from_name(id) {
                        cx.set(name);
                    }
                }
            }
            self.picked = Some(pick);
        }
    }

    fn show_slider(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>, range: &RangeInclusive<f64>) {
        let popup = cx.popup("slider");
        let start = cx.value().and_then(NumberValue::number).unwrap_or_else(|| self.settle_stored((range.start() + range.end()) / 2.0));
        let mut shown = start;
        let name = cx.label();
        let step = self.step.map(|step| self.to_stored(step));
        let (rule, scale) = (self.rule(), self.scale);
        let edit = ui.show(popup, Overlay::popover(), |ui, _| {
            let slider = Slider::new(Bind::bound(&mut shown), range.clone()).label(name.clone());
            match step {
                Some(step) => slider.step(step).show(ui),
                None => slider.show(ui),
            }
        });
        if let Some(edit) = edit {
            cx.adopt(&edit, &V::of_number(start), |value, mine| *mine = V::of_number(rule.settle(*value * scale) / scale));
        }
    }
}

/// The preset rows as a collection source, keyed by position.
struct Presets<'a> {
    rows: &'a [Entry<NumberPick>],
    labels: &'a [Label],
}

impl DisplayOrder<usize> for Presets<'_> {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.rows.len()).then_some(*key)
    }

    fn is_section(&self, at: usize) -> bool {
        self.rows[at].is_separator()
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.labels[at].as_str()))
    }
}

impl ItemSource<usize> for Presets<'_> {}

impl<V: NumberValue + 'static> Component for NumberField<'_, V> {
    const ROLE: Role = Role::SPIN_BUTTON;
    const INPUT: Input = Input::Text;
    const KEYS: crate::runtime::focus::KeyUse = crate::runtime::focus::KeyUse::of(crate::contract::Input::Text).with_moves();
    type Style = NumberStyle;
    type Output = NumberOutput<V>;

    fn style(cx: &StyleCx<'_>, _: NumberFieldStyleKey<V>) -> NumberStyle {
        let field = FieldStyle::at(cx);
        NumberStyle {
            surface: field.surface.clone(),
            metrics: field.metrics,
            unit: TextLook::new(TypeRole::Body, Stateful::new(cx.ink(Ink::OnPanel))),
            field,
        }
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        self.settle_lengths(&cx.env.units());
        let codec = self.ruled(&cx.env.units(), false, self.names(|row| cx.env.resolve(&row.label().clone())));
        let sample = cx.value().map_or(Cow::Borrowed(""), |value| codec.sample(value));
        let text = cx.layout(&TextRun::new(sample.as_ref())).size();
        self.unit_size = self.unit_extent(cx);
        let unit = if self.unit_size.x > 0.0 { self.unit_size.x + cx.gap() } else { 0.0 };
        // The chevrons' extent is the span from the text's end to the last chevron's far edge, laid out in the
        // field's minimum size, which is wide enough for any adornments.
        let chevrons = self.chevrons(&cx.words::<TextWords>());
        let probe = Rect::from_min_size(Pos2::ZERO, cx.style.metrics.min_size);
        let placed = Self::laid_out(cx.style, probe, false, &chevrons);
        let adornments = placed.trailing.last().map_or(0.0, |last| last.max.x - placed.text.max.x);
        Vec2::new(text.x + unit + adornments, text.y.max(self.unit_size.y))
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        self.settle_lengths(&cx.env.units());
        self.unit_size = self.unit_extent(cx);
        let rtl = cx.flow.is_rtl();
        let chevrons = self.chevrons(&cx.words::<TextWords>());
        let placed = Self::laid_out(cx.style, rect, rtl, &chevrons);
        let (text, _) = self.text_and_unit(placed.text, rtl, cx.gap());
        self.options.steps_value = self.presets.is_none() && self.popup_slider.is_none();
        let shown = cx.value();
        let names = self.names(|row| cx.env.resolve(&row.label().clone()));
        let blank = V::blank().unwrap_or_else(|| V::of_number(self.rule().blank()));
        cx.text(text_part::<V>(), text, &self.ruled(&cx.env.units(), shown.is_none(), names), shown.unwrap_or(&blank), &self.options);
        let slider_reason =
            if self.presets.is_some() { "Alt+Down opens the presets list; the slider opens by pointer" } else { "Alt+Down opens the slider" };
        for ((key, _), at) in chevrons.iter().zip(&placed.trailing) {
            let reason = if *key == PRESETS { "Alt+Down opens the list" } else { slider_reason };
            let popup = if *key == PRESETS { "presets" } else { "slider" };
            cx.part(*key, *at)
                .sense(Sense::CLICK | Sense::KEEP_FOCUS)
                .priority(HitPriority::new(HitLevel::Handle, 0))
                .pointer_only(reason)
                .toggles(popup);
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, V>> {
        Some(self.value.reborrow())
    }

    fn scrub(&self) -> Option<Scrub<V>> {
        let rule = self.rule();
        let drag = ValueDrag::relative(rule.step, rule.bounds()).warp(true).hide_cursor(true).coarse(LARGE_STEPS).fine(1.0 / LARGE_STEPS);
        // Rounded to the field's decimals, so a scrubbed value carries no float noise. The step is not a grid
        // here: an Alt drag moves by a share of it.
        let decimals = self.decimals();
        let scale = self.scale;
        self.scrub.then(|| {
            Scrub::new(drag, move |value: &V| value.number().unwrap_or(0.0) * scale, move |number| V::of_number(round_to(number, decimals) / scale))
        })
    }

    /// Arrows, Page keys, the wheel and AccessKit Increment and Decrement step the value, AccessKit sets it, and the
    /// chevrons open the popups. Up and Down step by the step, with Shift by `LARGE_STEPS` steps and, on a field
    /// without a dropdown, with Alt by a tenth; on a field with presets or a popup slider Alt+Down opens the dropdown
    /// instead, and the wheel keeps its Alt fine step. A step starts from the applied value, or from 0 kept in the
    /// range while the field is blank.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let rule = self.rule();
        while let Some((mv, mods)) = cx.intents().take_move() {
            let (notches, scale) = match mv {
                NavMove::Up => (1, key_scale(mods)),
                NavMove::Down => (-1, key_scale(mods)),
                NavMove::PageUp => (1, LARGE_STEPS),
                NavMove::PageDown => (-1, LARGE_STEPS),
                // AccessKit Increment and Decrement: the text keeps the horizontal arrows, so only those arrive.
                NavMove::Forward => (1, 1.0),
                NavMove::Back => (-1, 1.0),
                _ => continue,
            };
            let from = self.shown_number(cx.value()).unwrap_or_else(|| rule.blank());
            let next = match &self.stepper {
                Some(stepper) => rule.settle(stepper.next(from, notches)),
                None => rule.stepped(from, notches, scale),
            };
            cx.set(V::of_number(self.to_stored(next)));
        }
        if let Some(wheel) = cx.take_wheel() {
            let from = self.shown_number(cx.value()).unwrap_or_else(|| rule.blank());
            let steps = ValueSteps::new(from, rule.low, rule.high, rule.step, rule.step * LARGE_STEPS).fine(rule.step / LARGE_STEPS);
            if let StepResult::Set(next) = wheel_step(&steps, ValueAxes::Both, &wheel, cx.env.direction().is_rtl()) {
                let next = match &self.stepper {
                    Some(stepper) if next != from => stepper.next(from, if next > from { 1 } else { -1 }),
                    _ => round_to(next, decimals_for_step(rule.step / LARGE_STEPS)),
                };
                cx.set(V::of_number(self.to_stored(rule.settle(next))));
            }
        }
        if let Some(next) = cx.intents().take_set_value() {
            cx.set(V::of_number(self.to_stored(rule.settle(next))));
        }
    }

    /// The presets list and the popup slider.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        if let Some(rows) = self.presets {
            self.show_presets(ui, cx, rows);
        }
        if let Some(range) = self.popup_slider.clone() {
            self.show_slider(ui, cx, &range);
        }
    }

    /// The numeric value, and a button node per chevron named by its word (the painted face). The text part adds the
    /// text value, the selection and the text actions to the same node.
    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let rule = self.rule();
        let numeric = match self.shown_number(cx.value()) {
            Some(value) => Numeric::new(value, rule.bounds()),
            None => Numeric::empty(rule.bounds()),
        };
        access.numeric(numeric.step(rule.step));
        for (key, shown) in [(PRESETS, self.presets.is_some()), (SLIDER, self.popup_slider.is_some())] {
            if shown {
                access.node(key, Role::BUTTON);
            }
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let rtl = cx.flow.is_rtl();
        let chevrons = self.chevrons(&cx.words::<TextWords>());
        let placed = Self::laid_out(cx.style, cx.rect, rtl, &chevrons);
        let (_, unit) = self.text_and_unit(placed.text, rtl, cx.style.metrics.gap);
        if let (Some(unit), Some(label)) = (unit, &self.unit) {
            let top = unit.center().y - self.unit_size.y / 2.0;
            let (_, rest) = Flow::new(unit, rtl).take_start(cx.style.metrics.gap);
            let at = rest.rect();
            let run = TextRun::new(label.as_str()).role(cx.style.unit.role).color(cx.resolve(&cx.style.unit.color));
            ui.draw().text(Rect::from_min_size(Pos2::new(at.min.x, top), Vec2::new(at.width(), self.unit_size.y)), &run);
        }
        for (key, name) in chevrons {
            ui.face(key, IconButton::new(IconId::new(CHEVRON)).label(name));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> NumberOutput<V> {
        NumberOutput { edit: cx.edit_response(), preset: self.picked }
    }
}
