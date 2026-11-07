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
use crate::runtime::env::{Label, UnitKind, Units};
use crate::style::{ComponentStyle, Ink, StyleCx, SurfaceLook, TextLook, TypeRole};
use crate::value_drag::Slider;
use craft_behavior::nav::{LARGE_STEPS, StepResult, ValueAxes, ValueSteps};
use craft_behavior::{
    Codec, DisplayOrder, Flow, HitLevel, HitPriority, IconId, ItemSource, Metrics, MoveMods, NavMove, NotAValue, NumberCodec, PartKey, Selection,
    Sense, ValueDrag, decimals_for_step, wheel_step,
};
use craft_theme::Stateful;
use egui::{Pos2, Rect, Response, Vec2};
use std::borrow::Cow;
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

/// What a number field reports: the edit and the preset row picked this pass.
#[non_exhaustive]
pub struct NumberOutput {
    /// The edit response of the value.
    pub edit: EditResponse<Option<f64>>,
    /// The presets row the user picked this pass.
    pub preset: Option<NumberPick>,
}

impl Deref for NumberOutput {
    type Target = EditResponse<Option<f64>>;

    fn deref(&self) -> &Self::Target {
        &self.edit
    }
}

impl Outcome for NumberOutput {
    type Value = Option<f64>;

    fn response(&self) -> &Response {
        &self.edit.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        self.edit.story_facts()
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
/// loss restores the mixed display.
#[derive(Clone)]
struct Ruled {
    inner: NumberCodec,
    rule: Rule,
    snap: bool,
    decimals: Option<usize>,
    mixed: bool,
}

impl Ruled {
    fn held(&self, number: f64) -> f64 {
        let snapped = if self.snap { self.rule.snap(number) } else { number };
        self.rule.settle(self.decimals.map_or(snapped, |decimals| round_to(snapped, decimals)))
    }
}

impl Codec for Ruled {
    type Value = Option<f64>;

    fn format(&self, v: &Option<f64>) -> String {
        self.inner.format(v)
    }

    fn parse(&self, text: &str, start: &Option<f64>) -> Result<Option<f64>, NotAValue> {
        if self.mixed && text.trim().is_empty() {
            return Err(NotAValue);
        }
        self.inner.parse(text, start).map(|read| read.map(|number| self.held(number)))
    }

    fn applies_as_typed(&self, text: &str) -> bool {
        !self.snap && !self.mixed && is_plain(text) && matches!(self.inner.parse(text, &None), Ok(Some(_)))
    }

    fn filter(&self, c: char) -> bool {
        self.inner.filter(c)
    }

    fn normalise(&self, text: &str, v: &Option<f64>) -> String {
        self.inner.normalise(text, v)
    }

    fn caret_on_focus(&self, text: &str) -> craft_behavior::Caret {
        self.inner.caret_on_focus(text)
    }

    fn sample(&self, current: &Option<f64>) -> Cow<'_, str> {
        self.inner.sample(current)
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
#[component(overlay)]
pub struct NumberField<'a> {
    #[common]
    common: Common,
    value: Bind<'a, Option<f64>>,
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
    options: TextOptions,
    scrub: bool,
    picked: Option<NumberPick>,
    unit_size: Vec2,
}

impl<'a> NumberField<'a> {
    /// A number field named `label`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, Option<f64>>) -> Self {
        NumberField {
            common: Common::new(),
            value,
            codec: None,
            unit_kind: None,
            range: None,
            step: None,
            unit: None,
            presets: None,
            popup_slider: None,
            snap_typed: false,
            commit_decimals: None,
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

    fn rule(&self) -> Rule {
        Rule::new(self.range.as_ref(), self.step)
    }

    /// The field's codec: the one set, else the env's locale and the spellings of the unit kind.
    fn number_codec(&self, units: &Units) -> NumberCodec {
        self.codec.clone().unwrap_or_else(|| NumberCodec::new(units.reader(self.unit_kind), self.decimals()))
    }

    /// The most decimals the field writes.
    fn decimals(&self) -> usize {
        self.codec.as_ref().map_or_else(|| self.unit_kind.map_or(DEFAULT_DECIMALS, |kind| *kind.decimals().end()), NumberCodec::decimals)
    }

    fn ruled(&self, units: &Units, mixed: bool) -> Ruled {
        Ruled { inner: self.number_codec(units), rule: self.rule(), snap: self.snap_typed, decimals: self.commit_decimals, mixed }
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
        let current = cx.value().and_then(|value| *value);
        let held = |at: usize| match rows[at].value() {
            Some(NumberPick::Number(value)) => current.is_some_and(|current| (current - value).abs() <= 1e-4 * current.abs().max(1.0)),
            Some(NumberPick::Choice(_)) => rows[at].is_checked(),
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
            if let NumberPick::Number(value) = pick {
                cx.set(Some(self.rule().settle(value)));
            }
            self.picked = Some(pick);
        }
    }

    fn show_slider(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>, range: &RangeInclusive<f64>) {
        let popup = cx.popup("slider");
        let rule = self.rule();
        let start = cx.value().and_then(|value| *value).unwrap_or_else(|| rule.settle((range.start() + range.end()) / 2.0));
        let mut shown = start;
        let name = cx.label();
        let step = self.step;
        let edit = ui.show(popup, Overlay::popover(), |ui, _| {
            let slider = Slider::new(Bind::bound(&mut shown), range.clone()).label(name.clone());
            match step {
                Some(step) => slider.step(step).show(ui),
                None => slider.show(ui),
            }
        });
        if let Some(edit) = edit {
            cx.adopt(&edit, &Some(start), |value, mine| *mine = Some(rule.settle(*value)));
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

impl Component for NumberField<'_> {
    const ROLE: Role = Role::SPIN_BUTTON;
    const INPUT: Input = Input::Text;
    const KEYS: crate::runtime::focus::KeyUse = crate::runtime::focus::KeyUse::of(crate::contract::Input::Text).with_moves();
    type Style = NumberStyle;
    type Output = NumberOutput;

    fn style(cx: &StyleCx<'_>, _: NumberFieldStyleKey) -> NumberStyle {
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
        let codec = self.number_codec(&cx.env.units());
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
        self.unit_size = self.unit_extent(cx);
        let rtl = cx.flow.is_rtl();
        let chevrons = self.chevrons(&cx.words::<TextWords>());
        let placed = Self::laid_out(cx.style, rect, rtl, &chevrons);
        let (text, _) = self.text_and_unit(placed.text, rtl, cx.gap());
        self.options.steps_value = self.presets.is_none() && self.popup_slider.is_none();
        let shown = cx.value();
        cx.text(text_part::<Option<f64>>(), text, &self.ruled(&cx.env.units(), shown.is_none()), shown.unwrap_or(&None), &self.options);
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

    fn bind(&mut self) -> Option<BindMut<'_, Option<f64>>> {
        Some(self.value.reborrow())
    }

    fn scrub(&self) -> Option<Scrub<Option<f64>>> {
        let rule = self.rule();
        let drag = ValueDrag::relative(rule.step, rule.bounds()).warp(true).hide_cursor(true).coarse(LARGE_STEPS).fine(1.0 / LARGE_STEPS);
        // Rounded to the field's decimals, so a scrubbed value carries no float noise. The step is not a grid
        // here: an Alt drag moves by a share of it.
        let decimals = self.decimals();
        self.scrub.then(|| Scrub::new(drag, |value: &Option<f64>| value.unwrap_or(0.0), move |number| Some(round_to(number, decimals))))
    }

    /// Arrows, Page keys and the wheel step the value, AccessKit sets it, and the chevrons open the popups. Up and
    /// Down step by the step, with Shift by `LARGE_STEPS` steps and, on a field without a dropdown, with Alt by a
    /// tenth; on a field with presets or a popup slider Alt+Down opens the dropdown instead, and the wheel keeps its
    /// Alt fine step. A step starts from the applied value, or from 0 kept in the
    /// range while the field is blank.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let rule = self.rule();
        while let Some((mv, mods)) = cx.intents().take_move() {
            let (notches, scale) = match mv {
                NavMove::Up => (1, key_scale(mods)),
                NavMove::Down => (-1, key_scale(mods)),
                NavMove::PageUp => (1, LARGE_STEPS),
                NavMove::PageDown => (-1, LARGE_STEPS),
                _ => continue,
            };
            let from = cx.value().and_then(|value| *value).unwrap_or_else(|| rule.blank());
            cx.set(Some(rule.stepped(from, notches, scale)));
        }
        if let Some(wheel) = cx.take_wheel() {
            let from = cx.value().and_then(|value| *value).unwrap_or_else(|| rule.blank());
            let steps = ValueSteps::new(from, rule.low, rule.high, rule.step, rule.step * LARGE_STEPS).fine(rule.step / LARGE_STEPS);
            if let StepResult::Set(next) = wheel_step(&steps, ValueAxes::Both, &wheel, cx.env.direction().is_rtl()) {
                cx.set(Some(rule.settle(round_to(next, decimals_for_step(rule.step / LARGE_STEPS)))));
            }
        }
        if let Some(next) = cx.intents().take_set_value() {
            cx.set(Some(rule.settle(next)));
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

    /// The numeric value, and a button node per chevron named by its word (the painted face).
    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let rule = self.rule();
        let numeric = match cx.value() {
            Some(Some(value)) => Numeric::new(*value, rule.bounds()),
            _ => Numeric::empty(rule.bounds()),
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

    fn finish(self, cx: Cx<'_, Self>) -> NumberOutput {
        NumberOutput { edit: cx.edit_response(), preset: self.picked }
    }
}
