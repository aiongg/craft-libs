//! Text and number fields: `TextField<C>` and `NumberField` on the text part (18.3).

#[cfg(feature = "testing")]
mod stories;
mod words;

use crate::contract::{
    Accessible as _, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, MeasureCx, Overlay, OverlayUi, Overlays as _,
    PartsCx, Props as _, Remember as _, Role, Trigger,
};
use crate::primitives::text_adapter::{TextState, text_id};
use crate::primitives::{Completer, Completion, TextKey, TextOptions, TextRun};
use crate::runtime::egui::text_edit;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx, SurfaceLook};
use craft_behavior::{Codec, GesturePhase, Insets, Metrics, NumberCodec, PartKey, PlainText, PressPhase, Sense, ValueDrag};
use craft_keymap::{Chord, Key};
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

/// The completion list's state: the candidates shown, the highlighted one, and the draft Escape hid it for.
#[derive(Clone, Default)]
struct CompletionState {
    items: Vec<Completion>,
    index: usize,
    dismissed: Option<String>,
}

/// A text field over a codec: plain text by default.
#[derive(crate::contract::Component)]
#[component(probe(TextField<'static, PlainText>))]
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

    /// Completion candidates for the text before the caret. The field owns the list and the keys it takes.
    #[must_use]
    pub fn complete(mut self, completer: impl Completer + 'static) -> Self {
        self.options.complete = Some(Arc::new(completer));
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

    /// The completion list: candidates from the draft and caret, the keys it claims while open, acceptance by
    /// replacing the returned byte range of the draft.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let Some(completer) = self.options.complete.clone() else { return };
        let (ctx, id) = (cx.core.ctx.clone(), cx.core.id);
        let widget = text_id(id, TEXT);
        let popup = cx.popup("completions");
        popup.trigger(Trigger::Manual, cx);
        let draft = cx.text(&text_part::<C::Value>()).draft.clone();
        let mut items = Vec::new();
        let range = match &draft {
            Some(text) if cx.is_focused() => {
                let chars = text_edit::selection(&ctx, widget).map_or_else(|| text.chars().count(), |selection| selection.0);
                let caret = text.char_indices().nth(chars).map_or(text.len(), |(byte, _)| byte);
                completer.complete(text, caret, &mut items)
            }
            _ => None,
        };
        let down = cx.take_claimed(Chord::plain(Key::Down));
        let up = cx.take_claimed(Chord::plain(Key::Up));
        let escape = cx.take_claimed(Chord::plain(Key::Escape));
        let accept = cx.take_claimed(Chord::plain(Key::Enter)) | cx.take_claimed(Chord::plain(Key::Tab));
        let state = cx.memory::<CompletionState>();
        if escape {
            state.dismissed = draft.clone();
        }
        if state.dismissed.is_some() && state.dismissed != draft {
            state.dismissed = None;
        }
        let (Some(range), true) = (range, !items.is_empty() && state.dismissed.is_none()) else {
            *state = CompletionState::default();
            popup.close(cx);
            return;
        };
        if state.items.iter().map(|item| item.insert.as_str()).ne(items.iter().map(|item| item.insert.as_str())) {
            state.index = 0;
        }
        if down {
            state.index = (state.index + 1).min(items.len() - 1);
        }
        if up {
            state.index = state.index.saturating_sub(1);
        }
        state.items = items;
        let picked = state.items[state.index].clone();
        if accept {
            let text = draft.unwrap_or_default();
            let inserted = format!("{}{}{}", &text[..range.start], picked.insert, &text[range.end..]);
            let caret = inserted[..range.start + picked.insert.len()].chars().count();
            text_edit::select(&ctx, widget, caret, caret);
            let slot = cx.core.memory.shown::<TextState>(crate::runtime::memory::sub(id, "text_state"));
            slot.draft = Some(inserted);
            slot.dirty = true;
            *cx.memory::<CompletionState>() = CompletionState::default();
            popup.close(cx);
            return;
        }
        // Up, Down, Enter, Tab and Escape have one consumer while the list is open: the list.
        for key in [Key::Up, Key::Down, Key::Enter, Key::Tab, Key::Escape] {
            cx.claim(Chord::plain(key));
        }
        popup.open(cx);
    }

    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        if self.options.complete.is_none() {
            return;
        }
        let popup = cx.popup("completions");
        let state = cx.recall::<CompletionState>();
        ui.show(popup, Overlay::dropdown(), |ui, _| {
            for (at, item) in state.items.iter().enumerate() {
                let text = egui::RichText::new(item.label.as_str());
                ui.label(if at == state.index { text.strong() } else { text });
            }
        });
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<C::Value> {
        cx.edit_response()
    }
}

/// How far one point of label drag moves a scrubbed number.
const SCRUB_RATE: f64 = 1.0;

/// A number field: a text part over [`NumberCodec`]; empty text is `None`.
#[derive(crate::contract::Component)]
pub struct NumberField<'a> {
    #[common]
    common: Common,
    value: Bind<'a, Option<f64>>,
    codec: NumberCodec,
    options: TextOptions,
    scrub: bool,
}

impl<'a> NumberField<'a> {
    /// A number field named `label`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, Option<f64>>) -> Self {
        NumberField { common: Common::new(), value, codec: NumberCodec::default(), options: TextOptions::default(), scrub: false }.label(label)
    }

    /// The codec, to read another locale's separators or units.
    #[must_use]
    pub fn codec(mut self, codec: NumberCodec) -> Self {
        self.codec = codec;
        self
    }

    /// A drag on the field's side label scrubs the value.
    #[must_use]
    pub fn scrub_by_label(mut self) -> Self {
        self.scrub = true;
        self
    }
}

/// The value a scrub started from.
#[derive(Clone, Default)]
struct ScrubStart(f64);

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

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if !self.scrub || cx.core.resolved.read_only || !cx.core.resolved.enabled {
            return;
        }
        let Some(label) = cx.field_label() else { return };
        crate::runtime::access::scope::register_scrub(&cx.core.ctx);
        let rtl = cx.env.direction().is_rtl();
        let drag = ValueDrag::relative(SCRUB_RATE, f64::MIN..=f64::MAX);
        match label.press.phase {
            PressPhase::DragStart { .. } => {
                cx.memory::<ScrubStart>().0 = cx.value().copied().flatten().unwrap_or(0.0);
                cx.begin();
            }
            PressPhase::Drag { .. } => {
                let start = cx.memory::<ScrubStart>().0;
                if let Some(next) = drag.value(start, &label.press, rtl) {
                    cx.propose(Some(next));
                }
            }
            PressPhase::Release { .. } if cx.gesture().is_some_and(|phase| matches!(phase, GesturePhase::Begin | GesturePhase::Update)) => {
                cx.commit()
            }
            _ => {}
        }
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
