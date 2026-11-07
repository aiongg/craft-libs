//! `NumberField`: a text part over [`NumberCodec`] with a range, a step, a unit, presets and a popup slider.
//!
//! Typing, expressions, unit spellings and locale digits are the codec's (`quantity::Reader`); the field adds the
//! range every written value stays in and the step that arrows, the wheel and the label scrub move by.

#[cfg(feature = "testing")]
mod stories;

use super::{FieldStyle, text_part};
use crate::contract::{
    Access, Bind, BindMut, CollectionEvent, Common, Component, Cx, Edit as _, EditResponse, Focus as _, Input, LeadOnOpen, MeasureCx, Name, Numeric,
    Outcome, Overlay, OverlayUi, Overlays as _, PaintUi, PartsCx, Pointer as _, Props as _, Remember as _, Role, Scrub, SelectMode, ShowCollection as _,
    Show as _, Styled as _, ActivateOn, WordSetCx as _,
};
use crate::list::{List, ListRow};
use crate::primitives::{IconSpec, TextOptions, TextRun};
use crate::runtime::env::Label;
use crate::runtime::focus::{FocusPolicy, Initial};
use crate::style::{ComponentStyle, IconLook, Ink, StyleCx, SurfaceLook};
use crate::value_drag::Slider;
use craft_behavior::{
    Codec, DisplayOrder, Flow, HitLevel, HitPriority, IconId, Intent, ItemSource, Metrics, NavMove, NotAValue, NumberCodec, PartKey, Selection,
    Sense, ValueDrag,
};
use craft_theme::Stateful;
use egui::{Pos2, Rect, Response, Vec2};
use std::borrow::Cow;
use std::ops::{Deref, RangeInclusive};

/// The step of an arrow press, the wheel and the label scrub when the field sets none.
const DEFAULT_STEP: f64 = 1.0;
/// How many steps Shift and the Page keys move by.
const COARSE: f64 = 10.0;
/// The share of a step Ctrl or Cmd moves by.
const FINE: f64 = 0.1;
/// How far a value may sit off a multiple of the step and still count as on it.
const ON_GRID: f64 = 1e-9;

/// The chevron that opens the presets list.
const PRESETS: PartKey = PartKey::new("presets");
/// The chevron that opens the popup slider.
const SLIDER: PartKey = PartKey::new("slider");

/// One row of a number field's presets list.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Preset {
    /// A number: a pick sets the field to `value`. The row is selected while the field holds `value`.
    #[non_exhaustive]
    Value {
        /// The row's text.
        label: Label,
        /// The number a pick sets.
        value: f64,
    },
    /// A choice that is not a number (Auto, Metrics): a pick leaves the value alone and `NumberOutput::preset`
    /// names the row. `checked` says the choice is current.
    #[non_exhaustive]
    Choice {
        /// The row's text.
        label: Label,
        /// The choice is current.
        checked: bool,
    },
    /// A line between groups of rows.
    Separator,
}

impl Preset {
    /// A numeric row.
    pub fn value(label: impl Into<Label>, value: f64) -> Self {
        Preset::Value { label: label.into(), value }
    }

    /// A row that is not a number.
    pub fn choice(label: impl Into<Label>, checked: bool) -> Self {
        Preset::Choice { label: label.into(), checked }
    }
}

/// What a number field reports: the edit and the preset row picked this pass.
#[non_exhaustive]
pub struct NumberOutput {
    /// The edit response of the value.
    pub edit: EditResponse<Option<f64>>,
    /// The position of the `Preset::Choice` row the user picked.
    pub preset: Option<usize>,
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
        self.settle(round_to(multiple * size, places(size)))
    }
}

/// The decimal places of `size`.
fn places(size: f64) -> i32 {
    let text = format!("{size}");
    text.split_once('.').map_or(0, |(_, fraction)| fraction.len() as i32)
}

fn round_to(value: f64, places: i32) -> f64 {
    let scale = 10f64.powi(places);
    (value * scale).round() / scale
}

/// Whether `text` is a plain number rather than an expression or a relative input, which apply on commit only.
fn is_plain(text: &str) -> bool {
    let text = text.trim();
    !text.starts_with(['+', '*', '/']) && text.chars().skip(1).all(|c| !matches!(c, '+' | '-' | '*' | '/' | '(' | ')'))
}

/// The field's codec with the range applied to what it reads, and a plain number applying as it is typed.
#[derive(Clone)]
struct Ruled {
    inner: NumberCodec,
    rule: Rule,
}

impl Codec for Ruled {
    type Value = Option<f64>;

    fn format(&self, v: &Option<f64>) -> String {
        self.inner.format(v)
    }

    fn parse(&self, text: &str, start: &Option<f64>) -> Result<Option<f64>, NotAValue> {
        self.inner.parse(text, start).map(|read| read.map(|number| self.rule.settle(number)))
    }

    fn applies_as_typed(&self, text: &str) -> bool {
        is_plain(text) && matches!(self.inner.parse(text, &None), Ok(Some(_)))
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

/// The style of a number field: the field look and the chevron of its popups.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct NumberStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    /// The chevron of the presets list and the popup slider.
    #[style(icon)]
    chevron: IconLook,
}

/// The presses an opened popup counts, so each opening gives the presets list a new id and its lead starts again.
#[derive(Clone, Copy, Default)]
struct Opened(u32);

/// The row last highlighted in the open presets list.
#[derive(Clone, Copy, Default)]
struct Highlight(Option<usize>);

/// The field's regions along its inline axis.
struct Strip {
    text: Rect,
    unit: Option<Rect>,
    presets: Option<Rect>,
    slider: Option<Rect>,
}

/// A number field: a text part over [`NumberCodec`]; empty text is `None`.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(overlay)]
pub struct NumberField<'a> {
    #[common]
    common: Common,
    value: Bind<'a, Option<f64>>,
    /// The codec, to read another locale's separators or units.
    #[prop]
    codec: NumberCodec,
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
    presets: Option<&'a [Preset]>,
    /// A chevron after the field opens a slider over `range` below it.
    #[prop]
    popup_slider: Option<RangeInclusive<f64>>,
    options: TextOptions,
    scrub: bool,
    picked: Option<usize>,
    unit_size: Vec2,
}

impl<'a> NumberField<'a> {
    /// A number field named `label`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, Option<f64>>) -> Self {
        NumberField {
            common: Common::new(),
            value,
            codec: NumberCodec::default(),
            range: None,
            step: None,
            unit: None,
            presets: None,
            popup_slider: None,
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

    fn ruled(&self) -> Ruled {
        Ruled { inner: self.codec.clone(), rule: self.rule() }
    }

    /// The unit's size, the room it takes and the gap before it.
    fn unit_extent(&self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        self.unit.as_ref().map_or(Vec2::ZERO, |unit| cx.layout(&TextRun::new(unit.as_str())).size())
    }

    fn strip(&self, rect: Rect, rtl: bool, side: f32, gap: f32) -> Strip {
        let mut flow = Flow::new(rect, rtl);
        let mut take = |wanted: bool, width: f32| {
            wanted.then(|| {
                let (taken, rest) = flow.take_end(width);
                flow = rest;
                taken
            })
        };
        let slider = take(self.popup_slider.is_some(), side);
        let presets = take(self.presets.is_some(), side);
        let unit = take(self.unit_size.x > 0.0, self.unit_size.x + gap);
        Strip { text: flow.rect(), unit, presets, slider }
    }

    /// Toggles a popup from its chevron and opens the first from Alt+Down.
    fn drive_popups(&mut self, cx: &mut Cx<'_, Self>) {
        let presets = self.presets.map(|_| cx.popup("presets"));
        let slider = self.popup_slider.as_ref().map(|_| cx.popup("slider"));
        let mut asked = cx.intents().take(Intent::Open);
        let was_open = presets.is_some_and(|popup| popup.is_open(cx));
        for (popup, part) in [(presets, PRESETS), (slider, SLIDER)] {
            let Some(popup) = popup else { continue };
            if cx.part_if_declared(part).is_some_and(|result| result.press.phase.is_click()) {
                if popup.is_open(cx) {
                    popup.close(cx);
                } else {
                    popup.open(cx);
                }
                asked = false;
            }
        }
        if asked && let Some(popup) = presets.or(slider) && !popup.is_open(cx) {
            popup.open(cx);
        }
        if let Some(popup) = presets
            && popup.is_open(cx)
            && !was_open
        {
            *cx.memory::<Highlight>() = Highlight(None);
            cx.memory::<Opened>().0 += 1;
        }
    }

    fn show_presets(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>, rows: &[Preset]) {
        let popup = cx.popup("presets");
        let labels: Vec<Label> = rows
            .iter()
            .map(|row| match row {
                Preset::Value { label, .. } | Preset::Choice { label, .. } => cx.env.resolve(label),
                Preset::Separator => Label::Empty,
            })
            .collect();
        let current = cx.value().and_then(|value| *value);
        let held = |at: usize| match &rows[at] {
            Preset::Value { value, .. } => current.is_some_and(|current| (current - value).abs() <= 1e-4 * current.abs().max(1.0)),
            Preset::Choice { checked, .. } => *checked,
            Preset::Separator => false,
        };
        let marked = (0..rows.len()).find(|at| held(*at));
        let lead = cx.recall::<Highlight>().0.or(marked);
        let opened = cx.recall::<Opened>().0;
        let name = cx.label();
        let overlay = Overlay::dropdown().focus(FocusPolicy::POPUP.initial(Initial::First));
        let events = ui.show(popup, overlay, |ui, _| {
            let selection = lead.map_or_else(Selection::default, |at| Selection::new(vec![at], Some(at), Some(at)));
            let source = Presets { rows, labels: &labels };
            List::<usize>::new(name.clone())
                .select(SelectMode::Single)
                .activate(ActivateOn::Click)
                .lead_on_open(LeadOnOpen::First)
                .lead_follows_hover(true)
                .selection(Bind::given(selection))
                .id_salt(opened)
                .show_items(ui, &source, &mut |at, _| match rows[*at] {
                    Preset::Separator => ListRow::spacer(),
                    _ => ListRow::new(labels[*at].clone()).option(),
                })
                .events
        });
        let mut picked = None;
        for event in events.into_iter().flatten() {
            match event {
                CollectionEvent::LeadMoved(at) => cx.memory::<Highlight>().0 = Some(at),
                CollectionEvent::Activate(at) => picked = Some(at),
                _ => {}
            }
        }
        if let Some(at) = picked {
            match &rows[at] {
                Preset::Value { value, .. } => cx.set(Some(self.rule().settle(*value))),
                Preset::Choice { .. } => self.picked = Some(at),
                Preset::Separator => {}
            }
            popup.close(cx);
        }
    }

    fn show_slider(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>, range: &RangeInclusive<f64>) {
        let popup = cx.popup("slider");
        let rule = self.rule();
        let mut shown = cx.value().and_then(|value| *value).unwrap_or_else(|| rule.settle((range.start() + range.end()) / 2.0));
        let name = cx.label();
        let step = self.step;
        let changed = ui.show(popup, Overlay::popover(), |ui, _| {
            let before = shown;
            let slider = Slider::new(Bind::bound(&mut shown), range.clone()).label(name.clone());
            match step {
                Some(step) => slider.step(step).show(ui),
                None => slider.show(ui),
            };
            shown != before
        });
        if changed == Some(true) {
            cx.set(Some(rule.settle(shown)));
        }
    }
}

/// The preset rows as a collection source, keyed by position.
struct Presets<'a> {
    rows: &'a [Preset],
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
        matches!(self.rows[at], Preset::Separator)
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.labels[at].as_str()))
    }
}

impl ItemSource<usize> for Presets<'_> {}

impl Component for NumberField<'_> {
    const ROLE: Role = Role::SPIN_BUTTON;
    const INPUT: Input = Input::Text;
    type Style = NumberStyle;
    type Output = NumberOutput;

    fn style(cx: &StyleCx<'_>, _: NumberFieldStyleKey) -> NumberStyle {
        let FieldStyle { surface, metrics, .. } = FieldStyle::at(cx);
        NumberStyle { surface, metrics, chevron: IconLook::new(cx.control().icon, Stateful::new(cx.ink(Ink::OnPanel))) }
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let sample = cx.value().map_or(Cow::Borrowed(""), |value| self.codec.sample(value));
        let text = cx.layout(&TextRun::new(sample.as_ref())).size();
        self.unit_size = self.unit_extent(cx);
        let unit = if self.unit_size.x > 0.0 { self.unit_size.x + cx.gap() } else { 0.0 };
        let chevrons = f32::from(self.presets.is_some()) + f32::from(self.popup_slider.is_some());
        Vec2::new(text.x + unit + chevrons * cx.icon(), text.y.max(self.unit_size.y))
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        self.unit_size = self.unit_extent(cx);
        let strip = self.strip(rect, cx.flow.is_rtl(), cx.icon(), cx.gap());
        if let Some(value) = cx.value() {
            cx.text(text_part::<Option<f64>>(), strip.text, &self.ruled(), value, &self.options);
        }
        for (key, at, hint) in [(PRESETS, strip.presets, "Alt+Down opens the list"), (SLIDER, strip.slider, "Alt+Down opens the slider")] {
            if let Some(at) = at {
                cx.part(key, at).sense(Sense::CLICK).priority(HitPriority::new(HitLevel::Handle, 0)).pointer_only(hint);
            }
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, Option<f64>>> {
        Some(self.value.reborrow())
    }

    fn scrub(&self) -> Option<Scrub<Option<f64>>> {
        let rule = self.rule();
        let drag = ValueDrag::relative(rule.step, rule.bounds()).warp(true).hide_cursor(true);
        self.scrub.then(|| Scrub::new(drag, |value| value.unwrap_or(0.0), Some))
    }

    /// Arrows, Page keys and the wheel step the value, AccessKit sets it, and the chevrons open the popups. A
    /// step starts from the applied value, or from 0 kept in the range while the field is blank.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if self.presets.is_some() || self.popup_slider.is_some() {
            self.drive_popups(cx);
        }
        let rule = self.rule();
        while let Some((mv, mods)) = cx.intents().take_move() {
            let (notches, scale) = match mv {
                NavMove::Up => (1, if mods.extend { COARSE } else if mods.lead { FINE } else { 1.0 }),
                NavMove::Down => (-1, if mods.extend { COARSE } else if mods.lead { FINE } else { 1.0 }),
                NavMove::PageUp => (1, COARSE),
                NavMove::PageDown => (-1, COARSE),
                _ => continue,
            };
            let from = cx.value().and_then(|value| *value).unwrap_or_else(|| rule.blank());
            cx.set(Some(rule.stepped(from, notches, scale)));
        }
        if let Some(wheel) = cx.take_wheel()
            && wheel.scroll.y != 0.0
        {
            let notches = if wheel.scroll.y > 0.0 { 1 } else { -1 };
            let from = cx.value().and_then(|value| *value).unwrap_or_else(|| rule.blank());
            cx.set(Some(rule.stepped(from, notches, if wheel.mods.shift { COARSE } else { 1.0 })));
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

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let rule = self.rule();
        let numeric = match cx.value() {
            Some(Some(value)) => Numeric::new(*value, rule.bounds()),
            _ => Numeric::empty(rule.bounds()),
        };
        access.numeric(numeric.step(rule.step));
        for (key, shown) in [(PRESETS, self.presets.is_some()), (SLIDER, self.popup_slider.is_some())] {
            if shown {
                access.node(key, Role::BUTTON).name = Name::Text(cx.resolved.label.to_owned());
            }
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let strip = self.strip(cx.rect, cx.flow.is_rtl(), cx.style.chevron.side, cx.style.metrics.gap);
        let draw = ui.draw();
        if let (Some(unit), Some(label)) = (strip.unit, &self.unit) {
            let top = unit.center().y - self.unit_size.y / 2.0;
            let (_, rest) = Flow::new(unit, cx.flow.is_rtl()).take_start(cx.style.metrics.gap);
            let at = rest.rect();
            draw.text(Rect::from_min_size(Pos2::new(at.min.x, top), Vec2::new(at.width(), self.unit_size.y)), &TextRun::new(label.as_str()));
        }
        for (key, at) in [(PRESETS, strip.presets), (SLIDER, strip.slider)] {
            if let Some(at) = at {
                let glyph = Rect::from_center_size(at.center(), Vec2::splat(cx.style.chevron.side));
                draw.icon(glyph, &IconSpec::svg(IconId::new("chevron_down")).color(cx.resolve_part(key, &cx.style.chevron.color)));
            }
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> NumberOutput {
        NumberOutput { edit: cx.edit_response(), preset: self.picked }
    }
}
