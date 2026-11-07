//! `AltTextField`: an object's description for people who cannot see it, with a Mark as decorative check.

#[cfg(feature = "testing")]
mod stories;

use super::{FieldStyle, text_part};
use crate::contract::{
    Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, MeasureCx, PaintUi, PartsCx, Pointer as _, Props as _, Role,
    WordSetCx as _,
};
use crate::controls::check::Checkbox;
use crate::primitives::{Span, TextOptions, TextRun, Wrap};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx, SurfaceLook};
use craft_behavior::{Codec, Metrics, NotAValue, PartKey, PartRef, Proposal, Sense};
use egui::{Rect, Vec2, pos2, vec2};
use std::borrow::Cow;

/// The description box shows this many lines.
const LINES: usize = 4;

/// The check that marks the object decorative.
const DECORATIVE: PartKey = PartKey::new("decorative");
/// The description box while it is not an editable text part: decorative or mixed.
const BOX: PartKey = PartKey::new("box");

craft_ui::words! {
    /// The words of the alt text field.
    pub struct AltTextWords {
        /// The box's placeholder.
        alt_text: "altText.altText" = "Alt text";
        decorative: "altText.decorative" = "Mark as decorative";
        /// Shown in place of the description while the selection's descriptions differ.
        mixed: "altText.mixed" = "Several descriptions";
    }
}

/// An object's alt text: its description and whether it is decorative (read by no one).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct AltText {
    /// The description; empty when none is set.
    pub text: String,
    /// `true` when the object is decorative and needs no description.
    pub decorative: bool,
}

impl AltText {
    /// A description of `text`, not decorative.
    pub fn new(text: impl Into<String>) -> Self {
        AltText { text: text.into(), decorative: false }
    }

    /// Whether the object is decorative.
    #[must_use]
    pub fn decorative(mut self, decorative: bool) -> Self {
        self.decorative = decorative;
        self
    }

    /// Whether Mark as decorative starts checked for objects with these flags: only when there is at least one and
    /// every one is decorative.
    pub fn starts_decorative(flags: impl IntoIterator<Item = bool>) -> bool {
        let mut any = false;
        for flag in flags {
            if !flag {
                return false;
            }
            any = true;
        }
        any
    }
}

/// The description box's text form: the description as typed, with the decorative flag kept.
#[derive(Clone, Copy, Debug)]
struct AltCodec;

impl Codec for AltCodec {
    type Value = AltText;
    const DRAFTS_ARE_VALUES: bool = true;

    fn format(&self, value: &AltText) -> String {
        value.text.clone()
    }

    fn parse(&self, text: &str, start: &AltText) -> Result<AltText, NotAValue> {
        Ok(AltText { text: text.to_owned(), decorative: start.decorative })
    }

    fn sample(&self, _: &AltText) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
}

/// The style of the alt text field: the field look with a gap between the box and the check.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct AltTextFieldStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
}

/// An alt text editor: a multi-line description box and a Mark as decorative check. While the check is on, the box
/// keeps its text and does not edit it. The field's label is the prompt that names the box.
#[derive(crate::contract::Component)]
pub struct AltTextField<'a> {
    #[common]
    common: Common,
    value: Bind<'a, AltText>,
}

impl<'a> AltTextField<'a> {
    /// An alt text field named `label` over `value`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, AltText>) -> Self {
        AltTextField { common: Common::new(), value }.label(label)
    }
}

/// The check as a face over `on`.
fn check_face(on: bool, words: &AltTextWords) -> Checkbox<'static> {
    Checkbox::new(Bind::given(on)).label(words.decorative.clone())
}

/// The height of the description box and the size of the check.
fn sizes(cx: &MeasureCx<'_, AltTextField<'_>>) -> (f32, Vec2) {
    let words = cx.words::<AltTextWords>();
    let line = cx.layout(&TextRun::new("M")).size().y;
    (line * LINES as f32, cx.child("decorative", &mut check_face(false, &words), Proposal::Ideal))
}

impl Component for AltTextField<'_> {
    const ROLE: Role = Role::TEXT_INPUT;
    const INPUT: crate::contract::Input = crate::contract::Input::Text;
    const OVERLAYS: bool = false;
    type Style = AltTextFieldStyle;
    type Output = EditResponse<AltText>;

    fn style(cx: &StyleCx<'_>, _: AltTextFieldStyleKey) -> AltTextFieldStyle {
        let field = FieldStyle::at(cx);
        AltTextFieldStyle { surface: field.surface, metrics: field.metrics.gap(cx.control().gap) }
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let (text, check) = sizes(cx);
        let padding = cx.style.metrics.padding;
        let width = cx.room.map_or(check.x, |room| (room - padding.start - padding.end).max(0.0));
        Vec2::new(width, text + cx.gap() + check.y)
    }

    /// The description box (a text part, unless the object is decorative or the descriptions are mixed) and the check.
    fn parts(&mut self, _: Rect, cx: &mut PartsCx<'_, Self>) {
        let content = cx.content_rect;
        let (text, check) = sizes(cx);
        let words = cx.words::<AltTextWords>();
        let area = Rect::from_min_size(content.min, vec2(content.width(), text));
        let check_at = Rect::from_min_size(pos2(content.min.x, area.max.y + cx.gap()), vec2(content.width(), check.y));
        cx.part(DECORATIVE, check_at).sense(Sense::CLICK | Sense::FOCUS).focusable();
        match cx.value() {
            Some(value) if !value.decorative => {
                let mut options = TextOptions::default();
                options.lines = Some(LINES);
                options.placeholder = Some(words.alt_text.clone().into());
                cx.text(text_part::<AltText>(), area, &AltCodec, value, &options);
            }
            _ => {
                cx.part(BOX, area).sense(Sense::HOVER);
            }
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, AltText>> {
        Some(self.value.reborrow())
    }

    /// A click or key activation on the check flips the flag and keeps the text.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let Some(value) = cx.value().cloned() else { return };
        let by_key = cx.activation().filter(|activation| activation.by_key).and(cx.active_part());
        if cx.part(DECORATIVE).press.phase.is_click() || by_key == Some(PartRef::from(DECORATIVE)) {
            cx.set(AltText { decorative: !value.decorative, ..value });
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let words = cx.words::<AltTextWords>();
        let value = cx.value();
        ui.face(DECORATIVE, check_face(value.is_some_and(|value| value.decorative), &words));
        if let Some(area) = cx.part_if_declared(BOX).map(|part| part.rect) {
            let text = value.map_or_else(|| words.mixed.as_str(), |value| value.text.as_str());
            ui.draw().text(area, &TextRun::spans(vec![Span::new(text).muted()]).wrap(Wrap::Wrap));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<AltText> {
        cx.edit_response()
    }
}
