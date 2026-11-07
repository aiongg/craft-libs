//! Text and number fields: `TextField<C>` and `NumberField` on the text part (18.3).

mod alt_text;
mod apply;
mod chips;
mod completions;
mod linked;
mod number;
#[cfg(feature = "testing")]
mod stories;
mod words;

pub use alt_text::{AltText, AltTextField, AltTextFieldStyle, AltTextWords};
pub use apply::{ApplyRow, ApplyRowStyle};
pub use chips::{ChipCodec, ChipField, ChipFieldStyle, ChipWords};
pub use linked::{LinkMode, LinkedFields, LinkedFieldsStyle, LinkedLayout, LinkedOutput, LinkedWords, follow_equal, follow_ratio, link_toggle};
pub use number::{NumberField, NumberOutput, NumberStyle, Preset};

use crate::buttons::IconButton;
use crate::contract::{
    Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, OverlayUi, PaintUi, PartsCx, Pointer as _, Props as _, Role, Scrub,
    Styled as _, WordSetCx as _,
};
use crate::primitives::{Completer, IconSpec, TextKey, TextOptions, TextRun};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, IconLook, Ink, StyleCx, SurfaceLook};
use craft_behavior::{Codec, Flow, IconId, Insets, Metrics, NumberCodec, PartKey, PlainText, Sense, ValueDrag};
use craft_theme::Stateful;
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
    /// The search glyph.
    #[style(icon)]
    glyph: IconLook,
}

impl FieldStyle {
    fn at(cx: &StyleCx<'_>) -> FieldStyle {
        let control = cx.control();
        let metrics = Metrics::new()
            .padding(Insets::new(control.pad_x, 0.0, control.pad_x, 0.0))
            .gap(control.gap)
            .min_size(Vec2::new(control.height * MIN_WIDTH_IN_HEIGHTS, control.height));
        FieldStyle { surface: cx.field(), metrics, glyph: IconLook::new(control.icon, Stateful::new(cx.ink(Ink::OnPanel))) }
    }
}

/// The part key of the one text part.
const TEXT: PartKey = PartKey::new("text");

/// The clear part of a search field.
const CLEAR: PartKey = PartKey::new("clear");

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
    /// The value of an empty field: the text a mixed field shows and what the clear part sets.
    empty: Option<C::Value>,
    /// The search form: a leading glyph and a clear part, and Escape clears the text first.
    search: bool,
}

impl<'a> TextField<'a, PlainText> {
    /// A plain text field named `label`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, String>) -> Self {
        let mut field = TextField::with_codec(label, value, PlainText);
        field.empty = Some(String::new());
        field
    }

    /// A search field: a search glyph before the text, a clear part after it while there is text, and Escape
    /// clears the text first.
    pub fn search(label: impl Into<Label>, value: Bind<'a, String>) -> Self {
        let mut field = TextField::new(label, value);
        field.options.clear_on_escape = true;
        field.search = true;
        field
    }
}

impl<'a, C: Codec + Clone + 'static> TextField<'a, C> {
    /// A field named `label` that shows `value` through `codec`.
    pub fn with_codec(label: impl Into<Label>, value: Bind<'a, C::Value>, codec: C) -> Self {
        TextField { common: Common::new(), value, codec, options: TextOptions::default(), empty: None, search: false }.label(label)
    }

    /// The value of an empty field, for a codec whose values have no natural empty: a mixed field shows its text and
    /// the clear part of a search field sets it.
    #[must_use]
    pub fn empty(mut self, value: C::Value) -> Self {
        self.empty = Some(value);
        self
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

impl<C: Codec + Clone + 'static> TextField<'_, C> {
    /// The rects of the search form's glyph, clear part and text inside `rect`.
    fn search_rects(rect: Rect, rtl: bool, glyph: f32, gap: f32) -> (Rect, Rect, Rect) {
        let (lead, rest) = Flow::new(rect, rtl).take_start(glyph);
        let (_, rest) = rest.take_start(gap);
        let (clear, rest) = rest.take_end(rect.height());
        let (_, rest) = rest.take_end(gap);
        (lead, clear, rest.rect())
    }

    /// The clear part shows while the field has text.
    fn clear_shown(&self, value: Option<&C::Value>) -> bool {
        self.search && value.is_some_and(|value| !self.codec.format(value).is_empty())
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
        let mut size = cx.layout(&TextRun::new(text)).size();
        if self.search {
            size.x += cx.style.glyph.side + cx.style.metrics.min_size.y + 2.0 * cx.style.metrics.gap;
        }
        size
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let value = cx.value();
        let mut text = rect;
        if self.search {
            let (_, clear, rest) = Self::search_rects(rect, cx.flow.is_rtl(), cx.style.glyph.side, cx.style.metrics.gap);
            text = rest;
            if self.clear_shown(value) {
                cx.part(CLEAR, clear).sense(Sense::CLICK | Sense::KEEP_FOCUS).pointer_only("Escape clears the text");
            }
        }
        if let Some(value) = value.or(self.empty.as_ref()) {
            cx.text(text_part::<C::Value>(), text, &self.codec, value, &self.options);
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, C::Value>> {
        Some(self.value.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if cx.part_if_declared(CLEAR).is_some_and(|part| part.press.phase.is_click())
            && let Some(empty) = self.empty.clone()
        {
            cx.set(empty);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        if !self.search {
            return;
        }
        let glyph = &cx.style.glyph;
        let (lead, _, _) = Self::search_rects(cx.content_rect, cx.env.direction().is_rtl(), glyph.side, cx.style.metrics.gap);
        ui.draw().icon(lead, &IconSpec::svg(IconId::new("search")).color(cx.resolve(&glyph.color)));
        if cx.part_if_declared(CLEAR).is_some() {
            let label = cx.words::<words::TextWords>().clear.clone();
            ui.face(CLEAR, IconButton::new(IconId::new("close")).label(label));
        }
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
