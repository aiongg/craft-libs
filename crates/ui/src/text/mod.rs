//! Text and number fields: `TextField<C>` and `NumberField` on the text part (18.3).

mod completions;
#[cfg(feature = "testing")]
mod stories;
mod words;

use crate::contract::{Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, OverlayUi, PartsCx, Props as _, Role, Scrub};
use crate::primitives::{Completer, TextKey, TextOptions, TextRun};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx, SurfaceLook};
use craft_behavior::{Codec, Insets, Metrics, NumberCodec, PartKey, PlainText, Sense, ValueDrag};
use egui::{Rect, Vec2};
use std::borrow::Cow;
use std::sync::Arc;

/// A field is at least this many control heights wide, so a field with no text or placeholder still has a place.
const MIN_WIDTH_IN_HEIGHTS: f32 = 8.0;

/// The style of a text field: padding and minimum size from the control metrics.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FieldStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
}

impl FieldStyle {
    fn at(cx: &StyleCx<'_>) -> FieldStyle {
        let control = cx.control();
        let metrics = Metrics::new()
            .padding(Insets::new(control.pad_x, 0.0, control.pad_x, 0.0))
            .min_size(Vec2::new(control.height * MIN_WIDTH_IN_HEIGHTS, control.height));
        FieldStyle { surface: cx.field(), metrics }
    }
}

/// The part key of the one text part.
const TEXT: PartKey = PartKey::new("text");

fn text_part<V>() -> TextKey<V> {
    TextKey::new(TEXT)
}

/// A text field over a codec: plain text by default.
#[derive(crate::contract::Component)]
#[component(overlay, probe(TextField<'static, PlainText>))]
pub struct TextField<'a, C: Codec + Clone + 'static = PlainText> {
    #[common]
    common: Common,
    value: Bind<'a, C::Value>,
    codec: C,
    options: TextOptions,
}

impl<'a> TextField<'a, PlainText> {
    /// A plain text field named `label`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, String>) -> Self {
        TextField::with_codec(label, value, PlainText)
    }

    /// A search field: Escape clears the text first.
    pub fn search(label: impl Into<Label>, value: Bind<'a, String>) -> Self {
        let mut field = TextField::new(label, value);
        field.options.clear_on_escape = true;
        field
    }
}

impl<'a, C: Codec + Clone + 'static> TextField<'a, C> {
    /// A field named `label` that shows `value` through `codec`.
    pub fn with_codec(label: impl Into<Label>, value: Bind<'a, C::Value>, codec: C) -> Self {
        TextField { common: Common::new(), value, codec, options: TextOptions::default() }.label(label)
    }

    /// Shown while the text is empty.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<Label>) -> Self {
        self.options.placeholder = Some(placeholder.into());
        self
    }

    /// Monospace text.
    #[must_use]
    pub fn mono(mut self) -> Self {
        self.options.mono = true;
        self
    }

    /// Characters show as dots.
    #[must_use]
    pub fn masked(mut self) -> Self {
        self.options.mask = true;
        self
    }

    /// The field shows `lines` lines and wraps its text; 1 is a single-line field.
    #[must_use]
    pub fn lines(mut self, lines: usize) -> Self {
        self.options.lines = Some(lines);
        self
    }

    /// The most characters the text holds.
    #[must_use]
    pub fn limit(mut self, limit: usize) -> Self {
        self.options.limit = Some(limit);
        self
    }

    /// Tab inserts a tab.
    #[must_use]
    pub fn tab_inserts(mut self, on: bool) -> Self {
        self.options.tab_inserts = on;
        self
    }

    /// Completion candidates for the text before the caret. The text adapter owns the list and the keys it takes.
    #[must_use]
    pub fn complete(mut self, completer: impl Completer + 'static) -> Self {
        self.options.complete = Some(Arc::new(completer));
        self
    }

    /// `text` replaces the selection in this pass: at the caret it inserts. Set it for one pass only.
    #[must_use]
    pub fn replace_selection(mut self, text: impl Into<String>) -> Self {
        self.options.replace_selection = Some(text.into());
        self
    }
}

impl<C: Codec + Clone + 'static> Component for TextField<'_, C> {
    const ROLE: Role = Role::TEXT_INPUT;
    const INPUT: crate::contract::Input = crate::contract::Input::Text;
    type Style = FieldStyle;
    type Output = EditResponse<C::Value>;

    fn style(cx: &StyleCx<'_>, _: TextFieldStyleKey<C>) -> FieldStyle {
        FieldStyle::at(cx)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let sample: Cow<'_, str> = match cx.value() {
            Some(value) => self.codec.sample(value),
            None => Cow::Borrowed(""),
        };
        let text = if sample.is_empty() { self.options.placeholder.as_ref().map_or("", Label::as_str) } else { sample.as_ref() };
        cx.layout(&TextRun::new(text)).size()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        if let Some(value) = cx.value() {
            cx.text(text_part::<C::Value>(), rect, &self.codec, value, &self.options);
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, C::Value>> {
        Some(self.value.reborrow())
    }

    /// The completion list under the field, over what the text adapter holds.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        if self.options.complete.is_some() {
            completions::show(ui, cx, &text_part::<C::Value>());
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<C::Value> {
        cx.edit_response()
    }
}

/// How far one point of label drag moves a scrubbed number.
const SCRUB_RATE: f64 = 1.0;

/// A number field: a text part over [`NumberCodec`]; empty text is `None`.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct NumberField<'a> {
    #[common]
    common: Common,
    value: Bind<'a, Option<f64>>,
    /// The codec, to read another locale's separators or units.
    #[prop]
    codec: NumberCodec,
    options: TextOptions,
    scrub: bool,
}

impl<'a> NumberField<'a> {
    /// A number field named `label`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, Option<f64>>) -> Self {
        NumberField { common: Common::new(), value, codec: NumberCodec::default(), options: TextOptions::default(), scrub: false }.label(label)
    }

    /// A drag on the field's side label scrubs the value.
    #[must_use]
    pub fn scrub_by_label(mut self) -> Self {
        self.scrub = true;
        self
    }
}

impl Component for NumberField<'_> {
    const ROLE: Role = Role::SPIN_BUTTON;
    const INPUT: crate::contract::Input = crate::contract::Input::Text;
    type Style = FieldStyle;
    type Output = EditResponse<Option<f64>>;

    fn style(cx: &StyleCx<'_>, _: NumberFieldStyleKey) -> FieldStyle {
        FieldStyle::at(cx)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let sample = cx.value().map_or(Cow::Borrowed(""), |value| self.codec.sample(value));
        cx.layout(&TextRun::new(sample.as_ref())).size()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        if let Some(value) = cx.value() {
            cx.text(text_part::<Option<f64>>(), rect, &self.codec, value, &self.options);
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, Option<f64>>> {
        Some(self.value.reborrow())
    }

    fn scrub(&self) -> Option<Scrub<Option<f64>>> {
        self.scrub.then(|| Scrub::new(ValueDrag::relative(SCRUB_RATE, f64::MIN..=f64::MAX), |value| value.unwrap_or(0.0), Some))
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<Option<f64>> {
        cx.edit_response()
    }
}

/// The style of selectable text: padding from the control metrics, no surface.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SelectableStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// Read-only text the user can focus, select and copy.
#[derive(crate::contract::Component)]
#[component(exempt(case = "state:Mixed", reason = "Read-only text has no mixed state."))]
#[component(exempt(case = "state:Invalid", reason = "Read-only text has no value a check can refuse."))]
#[component(exempt(case = "values", reason = "Read-only text has no value a drive can change."))]
pub struct SelectableText<'a> {
    #[common]
    common: Common,
    value: Bind<'a, String>,
    options: TextOptions,
}

impl SelectableText<'_> {
    /// Selectable `text` named `label`.
    pub fn new(label: impl Into<Label>, text: &str) -> Self {
        SelectableText { common: Common::new(), value: Bind::given(text.to_owned()), options: TextOptions::default() }.label(label).read_only(true)
    }

    /// The text wraps over several lines.
    #[must_use]
    pub fn multiline(mut self) -> Self {
        self.options.lines = Some(usize::MAX);
        self
    }

    /// Monospace text.
    #[must_use]
    pub fn mono(mut self) -> Self {
        self.options.mono = true;
        self
    }
}

impl Component for SelectableText<'_> {
    const ROLE: Role = Role::TEXT_INPUT;
    const INPUT: crate::contract::Input = crate::contract::Input::Text;
    type Style = SelectableStyle;
    type Output = EditResponse<String>;

    fn style(cx: &StyleCx<'_>, _: SelectableTextStyleKey) -> SelectableStyle {
        SelectableStyle { metrics: Metrics::new().padding(Insets::new(cx.control().pad_x, 0.0, cx.control().pad_x, 0.0)) }
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let text = cx.value().map_or("", String::as_str);
        let run = TextRun::new(text);
        cx.layout(&if self.options.multiline() { run.wrap(crate::primitives::Wrap::Line) } else { run }).size()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        if let Some(value) = cx.value() {
            cx.text(text_part::<String>(), rect, &PlainText, value, &self.options);
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, String>> {
        Some(self.value.reborrow())
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<String> {
        cx.edit_response()
    }
}
