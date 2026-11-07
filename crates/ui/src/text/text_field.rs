//! `TextField<C>`: a text field over a codec, with the search form and the adornment layout.

#[cfg(feature = "testing")]
mod stories;

use super::adorn::{self, Adornment};
use super::{FieldStyle, completions, text_part, words};
use crate::contract::{
    Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, OverlayUi, PaintUi, PartsCx, Pointer as _, Props as _, Role,
    Styled as _, WordSetCx as _,
};
use crate::primitives::{Completer, IconSpec, TextOptions, TextRun};
use crate::runtime::env::Label;
use craft_behavior::{Codec, IconId, PartKey, PlainText, Sense};
use egui::{Id, Rect, Vec2};
use smallvec::SmallVec;
use std::borrow::Cow;
use std::sync::Arc;

/// The clear part of a search field.
const CLEAR: PartKey = PartKey::new("clear");

/// A text field over a codec: plain text by default.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(overlay, probe(TextField<'static, PlainText>))]
pub struct TextField<'a, C: Codec + Clone + 'static = PlainText> {
    #[common]
    common: Common,
    value: Bind<'a, C::Value>,
    codec: C,
    options: TextOptions,
    /// The value of an empty field, for a codec whose values have no natural empty: a mixed field shows its text and
    /// the clear part of a search field sets it.
    #[prop]
    empty: Option<C::Value>,
    /// The search form: a clear part while there is text, and Escape clears the text.
    search: bool,
    /// The adornments before the text, in reading order.
    leading: Vec<Adornment>,
    /// The field reserves and paints a chevron at the end, for the host's popup.
    arrow: bool,
    /// Monospace text.
    #[must_use]
    pub fn mono(mut self) -> Self {
        self.options.mono = true;
        self
    }

    /// Characters show as dots; the field exposes no value to assistive technology.
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
    /// The clear part shows while the field has text.
    fn clear_shown(&self, value: Option<&C::Value>) -> bool {
        self.search && value.is_some_and(|value| !self.codec.format(value).is_empty())
    }

    /// The adornments after the text, in reading order: the clear button when `clear` names it, then the arrow.
    fn trailing(&self, clear: Option<Label>) -> SmallVec<[Adornment; 2]> {
        let mut trailing = SmallVec::new();
        if let Some(name) = clear {
            trailing.push(Adornment::button(CLEAR, IconSpec::svg(IconId::new("close")), name));
        }
        if self.arrow {
            trailing.push(Adornment::glyph(IconSpec::svg(IconId::new("chevron_down"))));
        }
        trailing
    }

    /// The clear button's name, from the library words.
    fn clear_name(words: &words::TextWords) -> Label {
        words.clear.clone()
    }
}

impl<C: Codec + Clone + 'static> Component for TextField<'_, C> {
    const ROLE: Role = Role::TEXT_INPUT;
    const INPUT: crate::contract::Input = crate::contract::Input::Text;
    type Style = FieldStyle;
    type Output = EditResponse<C::Value>;

    fn style(cx: &crate::style::StyleCx<'_>, _: TextFieldStyleKey<C>) -> FieldStyle {
        FieldStyle::at(cx)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn parts_follow_value(&self) -> bool {
        self.search
    }

    /// The text, and the room the adornments take; a search field keeps the room of its clear button while the text is
    /// empty, so the field does not change width as the button comes and goes.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let sample: Cow<'_, str> = match cx.value() {
            Some(value) => self.codec.sample(value),
            None => Cow::Borrowed(""),
        };
        let text = if sample.is_empty() { self.options.placeholder.as_ref().map_or("", Label::as_str) } else { sample.as_ref() };
        let mut size = cx.layout(&TextRun::new(text)).size();
        let clear = self.search.then(|| Self::clear_name(&cx.words::<words::TextWords>()));
        size.x += adorn::reserve(cx.style, &self.leading, &self.trailing(clear));
        size
    }

    fn parts(&mut self, _: Rect, cx: &mut PartsCx<'_, Self>) {
        let value = cx.value();
        let clear = self.clear_shown(value).then(|| Self::clear_name(&cx.words::<words::TextWords>()));
        let trailing = self.trailing(clear);
        let placed = adorn::place(cx.content_rect, cx.style, &self.leading, &trailing, cx.flow.is_rtl());
        for (adornment, rect) in trailing.iter().zip(&placed.trailing) {
            if adornment.part() == Some(CLEAR) {
                cx.part(CLEAR, *rect).sense(Sense::CLICK | Sense::KEEP_FOCUS).pointer_only("Escape clears the text");
            }
        }
        if let Some(value) = value.or(self.empty.as_ref()) {
            cx.text(text_part::<C::Value>(), placed.text, &self.codec, value, &self.options);
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, C::Value>> {
        Some(self.value.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some(list) = self.drives {
            let empty = cx.value().is_none_or(|value| self.codec.format(value).is_empty());
            crate::runtime::focus::registration::drive(&cx.core.ctx, cx.core.id, list, empty);
        }
        if cx.part_activated(CLEAR).is_some()
            && let Some(empty) = self.empty.clone()
        {
            cx.set(empty);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let clear = cx.part_if_declared(CLEAR).is_some().then(|| Self::clear_name(&cx.words::<words::TextWords>()));
        let trailing = self.trailing(clear);
        let placed = adorn::place(cx.content_rect, cx.style, &self.leading, &trailing, cx.env.direction().is_rtl());
        let color = cx.resolve(&cx.style.glyph.color);
        for (adornment, rect) in self.leading.iter().chain(&trailing).zip(placed.leading.iter().chain(&placed.trailing)) {
            adornment.paint(ui, *rect, color);
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
