//! Number widgets that write numbers the way the user's region does (its digits, decimal mark and
//! grouping) and read back any digits with the region's separators.
//!
//! [`LocaleNumbers`] wraps a [`craft_i18n::Format`], which writes and reads the numbers.
//! [`LocalizedDragValue`] puts them on an [`egui::DragValue`]; [`NumberField`] is a text field for
//! apps that edit numbers in a [`egui::TextEdit`]. Text that isn't a number never changes the
//! value: it is outlined in the error color with a tooltip saying why.

use std::cell::Cell;
use std::ops::RangeInclusive;
use std::rc::Rc;
use std::sync::Arc;

use craft_i18n::{Format, I18n, Lang, NumberError};
use egui::emath::{Numeric, round_to_decimals};
use egui::{AsIdSalt, DragValue, Id, IdSalt, Key, Modifiers, Response, Stroke, StrokeKind, TextEdit, Ui, Widget};

#[cfg(test)]
mod tests;

/// How long a [`LocalizedDragValue`] keeps marking text it could not read after reverting, in
/// seconds.
const REJECTED_FOR: f64 = 2.5;

type Messages = Arc<dyn Fn(&'static str) -> String + Send + Sync>;
type Fallback<'a> = Box<dyn 'a + Fn(&str) -> Option<f64>>;
type Formatter<'a> = Box<dyn 'a + Fn(f64, RangeInclusive<usize>) -> Option<String>>;
/// The last text egui asked the parser to read: whether it is still being typed, and the result.
type LastRead = Rc<Cell<Option<(bool, Result<f64, NumberError>)>>>;

/// How the user's region writes and reads numbers. Cheap to clone; build it from
/// [`I18n::format`] so widgets match the app's counts and sizes.
#[derive(Clone)]
pub struct LocaleNumbers {
    format: Arc<Format>,
    messages: Option<Messages>,
    grouping: bool,
}

impl std::fmt::Debug for LocaleNumbers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocaleNumbers").field("format", &self.format).finish_non_exhaustive()
    }
}

impl LocaleNumbers {
    pub fn new(format: Arc<Format>) -> LocaleNumbers {
        LocaleNumbers { format, messages: None, grouping: true }
    }

    /// The numbers of a locale tag (`de-CH`, `ar-EG`) with its own digits.
    pub fn from_tag(tag: &str) -> LocaleNumbers {
        LocaleNumbers::new(Arc::new(Format::from_tag(tag)))
    }

    /// Translate the error tooltips: `translate` gets the English text ([`message`]) and returns
    /// it in the interface language, e.g. `move |s| i18n.t(lang, s).to_owned()`.
    pub fn with_messages(mut self, translate: impl Fn(&'static str) -> String + Send + Sync + 'static) -> LocaleNumbers {
        self.messages = Some(Arc::new(translate));
        self
    }

    /// Write numbers without grouping separators, for pixel sizes and years: `1920`, not `1,920`.
    /// Grouped text still reads back.
    pub fn ungrouped(mut self) -> LocaleNumbers {
        self.grouping = false;
        self
    }

    /// `value` with between `decimals.start()` and `decimals.end()` digits after the decimal mark,
    /// trailing zeros dropped down to the minimum: `1,5`, `١٬٢٣٤٫٥`.
    pub fn format(&self, value: f64, decimals: RangeInclusive<usize>) -> String {
        let (min, max) = (*decimals.start(), (*decimals.end()).max(*decimals.start()));
        let full = round_to_decimals(value, max);
        let decimals = (min..max).find(|&d| round_to_decimals(value, d) == full).unwrap_or(max);
        // `-0.001` shown with two decimals is `0,00`, not `-0,00`.
        let value = if round_to_decimals(value, decimals) == 0.0 { 0.0 } else { value };
        if self.grouping { self.format.number(value, decimals) } else { self.format.number_ungrouped(value, decimals) }
    }

    /// The number in `text`, as [`Format::parse`] reads it: what [`LocaleNumbers::format`] writes,
    /// in any digits.
    pub fn parse(&self, text: &str) -> Result<f64, NumberError> {
        self.format.parse(text)
    }

    fn describe(&self, error: NumberError) -> String {
        let english = message(error);
        self.messages.as_ref().map_or_else(|| english.to_owned(), |translate| translate(english))
    }
}

/// The English text the widgets show for `error`; an app catalog key.
pub fn message(error: NumberError) -> &'static str {
    match error {
        NumberError::Empty => "Enter a number",
        NumberError::Unexpected(_) => "Not a number",
        NumberError::MixedDigits => "Use one kind of digits",
        NumberError::Separator => "Check the decimal and grouping marks",
        NumberError::TooLarge => "Number too large",
    }
}

/// A unit around a number, from a craft-i18n unit template with the number as `{n}`: `{n} px`,
/// `{n}%`, Turkish `%{n}`. Typed text may include it; it is removed before parsing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Unit {
    prefix: String,
    suffix: String,
}

impl Unit {
    /// The unit `template` spells; a template without `{n}` is a suffix.
    pub fn new(template: &str) -> Unit {
        match template.split_once("{n}") {
            Some((prefix, suffix)) => Unit { prefix: prefix.to_owned(), suffix: suffix.to_owned() },
            None => Unit { prefix: String::new(), suffix: template.to_owned() },
        }
    }

    /// The unit row `template` (context `unit`) in `lang`, the English template if untranslated.
    pub fn translated(i18n: &I18n, lang: Lang, template: &str) -> Unit {
        Unit::new(i18n.get(lang, "unit", template).unwrap_or(template))
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn suffix(&self) -> &str {
        &self.suffix
    }

    fn strip<'t>(&self, text: &'t str) -> &'t str {
        let text = text.trim();
        let text = text.strip_prefix(self.prefix.trim()).unwrap_or(text);
        text.strip_suffix(self.suffix.trim()).unwrap_or(text).trim()
    }
}

/// Parses `text` with `numbers`, then with `fallback` (an app's expression or timecode reader).
fn read(numbers: &LocaleNumbers, unit: &Unit, fallback: Option<&Fallback<'_>>, text: &str) -> Result<f64, NumberError> {
    numbers.parse(unit.strip(text)).or_else(|error| fallback.and_then(|f| f(text)).ok_or(error))
}

/// Text that may still become a number as the user types (`-`, `1,`): not marked while editing.
fn incomplete(text: &str, error: NumberError) -> bool {
    match error {
        NumberError::Empty => true,
        NumberError::Separator => text.trim_end().ends_with(['.', ',', '٫', '٬', '\'', '’', '．', '，']),
        _ => false,
    }
}

/// Text the widget could not read. `until` is `None` while the user is still editing it.
#[derive(Clone)]
struct Rejected {
    error: NumberError,
    until: Option<f64>,
}

fn rejected_id(id: Id) -> Id {
    id.with("craft_ui::localized::rejected")
}

/// Outlines `response` in the error color and, with `tell`, shows `text` beside it.
fn mark_invalid(ui: &Ui, response: &Response, text: String, tell: bool) {
    let visuals = ui.visuals();
    let radius = visuals.widgets.inactive.corner_radius;
    ui.painter().rect_stroke(response.rect, radius, Stroke::new(1.5, visuals.error_fg_color), StrokeKind::Inside);
    if tell {
        response.show_tooltip_text(text);
    }
}

/// An [`egui::DragValue`] that writes and reads numbers with [`LocaleNumbers`]. Set range, speed,
/// decimals and any prefix or suffix on the drag value; a [`Unit`] set here goes outside that
/// prefix and suffix (egui adds to them and can't take them off), so set one or the other. The
/// drag value's own `custom_formatter` and `custom_parser` are replaced: pass an app's formatter
/// and parser to [`LocalizedDragValue::or_format`] and [`LocalizedDragValue::or_parse`] instead.
///
/// Typed text that isn't a number is outlined while it is being edited. Committing it (Enter,
/// clicking away) keeps the old value, as egui does, and the field stays outlined with the reason
/// for a moment. Escape reverts without a mark.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use craft_ui::localized::{LocaleNumbers, LocalizedDragValue, Unit};
/// let numbers = LocaleNumbers::from_tag("de");
/// let mut width = 1.5;
/// ui.add(LocalizedDragValue::new(egui::DragValue::new(&mut width).speed(0.1), &numbers).unit(Unit::new("{n} px")));
/// # });
/// ```
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct LocalizedDragValue<'a> {
    drag: DragValue<'a>,
    numbers: LocaleNumbers,
    unit: Unit,
    formatter: Option<Formatter<'a>>,
    fallback: Option<Fallback<'a>>,
}

impl<'a> LocalizedDragValue<'a> {
    pub fn new(drag: DragValue<'a>, numbers: &LocaleNumbers) -> Self {
        LocalizedDragValue { drag, numbers: numbers.clone(), unit: Unit::default(), formatter: None, fallback: None }
    }

    /// The unit around the number, outside a prefix and suffix set on the drag value.
    pub fn unit(mut self, unit: Unit) -> Self {
        self.unit = unit;
        self
    }

    /// [`DragValue::range`] of the wrapped drag value.
    pub fn range<Num: Numeric>(mut self, range: RangeInclusive<Num>) -> Self {
        self.drag = self.drag.range(range);
        self
    }

    /// [`DragValue::speed`] of the wrapped drag value.
    pub fn speed(mut self, speed: impl Into<f64>) -> Self {
        self.drag = self.drag.speed(speed);
        self
    }

    /// Show the number without grouping separators ([`LocaleNumbers::ungrouped`]).
    pub fn ungrouped(mut self) -> Self {
        self.numbers.grouping = false;
        self
    }

    /// An app's own text for some values (a timecode, `Auto` for 0), tried before the locale's;
    /// `None` writes the number as the locale does. Pair it with [`LocalizedDragValue::or_parse`]
    /// so the text reads back.
    pub fn or_format(mut self, formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> Option<String>) -> Self {
        self.formatter = Some(Box::new(formatter));
        self
    }

    /// A parser for text that isn't a plain number (an expression, a timecode), tried after the
    /// locale's.
    pub fn or_parse(mut self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.fallback = Some(Box::new(parser));
        self
    }
}

impl Widget for LocalizedDragValue<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let LocalizedDragValue { drag, numbers, unit, formatter, fallback } = self;
        let parsed: LastRead = Rc::new(Cell::new(None));
        let mut drag = drag.custom_formatter({
            let numbers = numbers.clone();
            move |value, decimals| formatter.as_ref().and_then(|f| f(value, decimals.clone())).unwrap_or_else(|| numbers.format(value, decimals))
        });
        if !unit.prefix.is_empty() {
            drag = drag.prefix(unit.prefix.clone());
        }
        if !unit.suffix.is_empty() {
            drag = drag.suffix(unit.suffix.clone());
        }
        drag = drag.custom_parser({
            let (numbers, unit, parsed) = (numbers.clone(), unit.clone(), Rc::clone(&parsed));
            move |text| {
                let result = read(&numbers, &unit, fallback.as_ref(), text);
                let still_typing = result.err().is_some_and(|error| incomplete(text, error));
                parsed.set(Some((still_typing, result)));
                result.ok()
            }
        });
        let response = ui.add(drag);

        let key = rejected_id(response.id);
        let now = ui.input(|i| i.time);
        let editing = response.has_focus();
        let mut rejected = ui.data(|d| d.get_temp::<Rejected>(key));
        match parsed.take() {
            Some((_, Ok(_))) => rejected = None,
            Some((still_typing, Err(error))) if editing => rejected = (!still_typing).then_some(Rejected { error, until: None }),
            // egui reads the text once more the frame after the edit loses focus.
            Some((_, Err(error))) => rejected = Some(Rejected { error, until: Some(now + REJECTED_FOR) }),
            // Escape, or focus left without a final read.
            None if !editing && rejected.as_ref().is_some_and(|r| r.until.is_none()) => rejected = None,
            None => {}
        }
        // A drag or arrow key after a rejection sets a new value: the mark is moot.
        if response.changed() || rejected.as_ref().is_some_and(|r| r.until.is_some_and(|t| t <= now)) {
            rejected = None;
        }
        match rejected {
            Some(r) => {
                if let Some(until) = r.until {
                    ui.ctx().request_repaint_after_secs((until - now) as f32);
                }
                mark_invalid(ui, &response, numbers.describe(r.error), true);
                ui.data_mut(|d| d.insert_temp(key, r));
            }
            None => ui.data_mut(|d| d.remove::<Rejected>(key)),
        }
        response
    }
}

/// What a [`NumberField`] keeps between frames while its text differs from the value.
#[derive(Clone)]
struct FieldState {
    text: String,
    /// The value when `text` was stored; a different value means the app changed it, and the text
    /// is dropped.
    value: f64,
    error: Option<NumberError>,
}

/// A single-line text field for a number, read on commit (Enter or focus leaving) with
/// [`LocaleNumbers`]. Arrow up and down step the value.
///
/// Text that isn't a number leaves the value unchanged and stays in the field, outlined, with the
/// reason in a tooltip, until it is fixed or Escape restores the value. A number is rounded to the
/// most decimals shown, half away from zero (`2,5` is 3 and `-0,5` is -1 in an integer field),
/// then clamped to the range. The response is `changed()` when the value changed.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use craft_ui::localized::{LocaleNumbers, NumberField, Unit};
/// let numbers = LocaleNumbers::from_tag("fr");
/// let mut gap = 12.0;
/// ui.add(NumberField::new(&mut gap, &numbers).range(0.0..=100.0).step(0.5).unit(Unit::new("{n} mm")));
/// # });
/// ```
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct NumberField<'a> {
    get_set: Box<dyn 'a + FnMut(Option<f64>) -> f64>,
    numbers: LocaleNumbers,
    range: RangeInclusive<f64>,
    step: f64,
    decimals: RangeInclusive<usize>,
    unit: Unit,
    fallback: Option<Fallback<'a>>,
    width: Option<f32>,
    id_salt: Option<IdSalt>,
}

impl<'a> NumberField<'a> {
    /// A field for `value`. Integer types get no decimals and their full range.
    pub fn new<Num: Numeric>(value: &'a mut Num, numbers: &LocaleNumbers) -> Self {
        let field = NumberField {
            get_set: Box::new(move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = Num::from_f64(v);
                }
                value.to_f64()
            }),
            numbers: numbers.clone(),
            range: f64::NEG_INFINITY..=f64::INFINITY,
            step: 1.0,
            decimals: 0..=6,
            unit: Unit::default(),
            fallback: None,
            width: None,
            id_salt: None,
        };
        if Num::INTEGRAL { field.decimals(0..=0).range(Num::MIN.to_f64()..=Num::MAX.to_f64()) } else { field }
    }

    pub fn range(mut self, range: RangeInclusive<f64>) -> Self {
        self.range = range;
        self
    }

    /// The change per arrow key press; 0 turns stepping off. Default 1.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// The fewest and most digits shown after the decimal mark. Default `0..=6`.
    pub fn decimals(mut self, decimals: RangeInclusive<usize>) -> Self {
        self.decimals = decimals;
        self
    }

    pub fn unit(mut self, unit: Unit) -> Self {
        self.unit = unit;
        self
    }

    /// A parser for text that isn't a plain number (an expression, a timecode), tried after the
    /// locale's.
    pub fn or_parse(mut self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.fallback = Some(Box::new(parser));
        self
    }

    pub fn desired_width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }
}

impl Widget for NumberField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let NumberField { mut get_set, numbers, range, step, decimals, unit, fallback, width, id_salt } = self;
        let id = id_salt.map_or_else(|| ui.next_auto_id(), |salt| ui.make_persistent_id(salt));
        let old = get_set(None);
        let shown = numbers.format(old, decimals.clone());
        let clamp = |v: f64| if range.start() <= range.end() { v.clamp(*range.start(), *range.end()) } else { v };

        let mut state = ui.data(|d| d.get_temp::<FieldState>(id)).filter(|s| s.value == old);
        let focused = ui.memory(|m| m.has_focus(id));
        let mut value = old;
        if focused && step != 0.0 {
            let change = ui.input_mut(|i| {
                i.count_and_consume_key(Modifiers::NONE, Key::ArrowUp) as f64 - i.count_and_consume_key(Modifiers::NONE, Key::ArrowDown) as f64
            });
            if change != 0.0 {
                let base = state.as_ref().and_then(|s| read(&numbers, &unit, fallback.as_ref(), &s.text).ok()).unwrap_or(old);
                value = clamp(round_half_away(base + step * change, *decimals.end()));
                state = None;
            }
        }
        let mut text = match &state {
            Some(s) => s.text.clone(),
            None if value == old => shown,
            None => numbers.format(value, decimals.clone()),
        };

        let mut edit = TextEdit::singleline(&mut text).id(id);
        if !unit.prefix.is_empty() {
            edit = edit.prefix(unit.prefix.clone());
        }
        if !unit.suffix.is_empty() {
            edit = edit.suffix(unit.suffix.clone());
        }
        if let Some(width) = width {
            edit = edit.desired_width(width);
        }
        let mut response = ui.add(edit);
        let editing = response.has_focus();

        let escape = ui.input(|i| i.key_pressed(Key::Escape));
        let mut error = state.as_ref().and_then(|s| s.error);
        if response.lost_focus() && escape {
            state = None;
            error = None;
        } else if response.lost_focus() {
            match read(&numbers, &unit, fallback.as_ref(), &text) {
                Ok(parsed) => {
                    value = clamp(round_half_away(parsed, *decimals.end()));
                    state = None;
                    error = None;
                }
                Err(e) => {
                    error = Some(e);
                    state = Some(FieldState { text: text.clone(), value, error });
                }
            }
        } else if response.changed() {
            error = read(&numbers, &unit, fallback.as_ref(), &text).err().filter(|e| !incomplete(&text, *e));
            state = Some(FieldState { text: text.clone(), value, error });
        } else if response.has_focus() && state.is_none() {
            state = Some(FieldState { text: text.clone(), value, error: None });
        }

        if value != old {
            get_set(Some(value));
            response.mark_changed();
        } else {
            response.flags.remove(egui::response::Flags::CHANGED);
        }
        let value = get_set(None);
        match state {
            Some(mut s) => {
                s.value = value;
                ui.data_mut(|d| d.insert_temp(id, s));
            }
            None => ui.data_mut(|d| d.remove::<FieldState>(id)),
        }
        if let Some(error) = error {
            mark_invalid(ui, &response, numbers.describe(error), editing);
            if !editing {
                response = response.on_hover_text(numbers.describe(error));
            }
        }
        response
    }
}

/// `value` rounded to `decimals` digits after the point, ties away from zero; never `-0`.
fn round_half_away(value: f64, decimals: usize) -> f64 {
    let scale = 10_f64.powi(i32::try_from(decimals).unwrap_or(i32::MAX));
    let rounded = if scale.is_finite() { (value * scale).round() / scale } else { value };
    if rounded.is_finite() { rounded + 0.0 } else { value }
}
