//! `SelectFace`: the closed box of a dropdown picker, the text and a trailing chevron.

use crate::contract::{Common, Component, Cx, MeasureCx, PaintUi, Props as _, Role, Styled as _};
use crate::primitives::{IconSpec, TextRun};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, IconLook, Ink, StyleCx, TextLook, TypeRole};
use crate::text::{Adornment, FieldStyle, place, reserve};
use craft_behavior::{IconId, Metrics, Sense};
use craft_theme::Stateful;
use egui::{Response, Vec2};

/// The icon of the chevron after the text.
const CHEVRON: &str = "chevron_down";

/// The style of a select face: the field's metrics, the text, the muted placeholder and the chevron. The host owns the
/// surface (the field well), so a face paints only its content.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SelectFaceStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The picked option's text.
    #[style(text)]
    text: TextLook,
    /// The text shown when nothing is picked.
    #[style(text)]
    placeholder: TextLook,
    /// The chevron.
    #[style(icon)]
    chevron: IconLook,
    /// The field look the chevron is placed by.
    field: FieldStyle,
}

/// The closed box of a picker: `text` elided to the room left of a trailing chevron. The box is as wide as the text
/// and the chevron need, at least a field wide. A host shows it as the look of one of its parts with
/// `ui.face(part, SelectFace::new(text))` over the field well it paints itself, and gives the box the width of its
/// widest text so the width does not change with the pick.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(sample = SelectFace::sample)]
pub struct SelectFace {
    #[common]
    common: Common,
    /// The text is a placeholder, not a pick: it shows in the muted ink.
    #[prop(flag)]
    placeholder: bool,
}

impl SelectFace {
    /// A face showing `text`, which is also its name.
    pub fn new(text: impl Into<Label>) -> Self {
        SelectFace { common: Common::new(), placeholder: false }.label(text)
    }

    #[cfg(feature = "testing")]
    fn sample() -> Self {
        SelectFace::new("Multiply")
    }

    fn chevron() -> Adornment {
        Adornment::glyph(IconSpec::svg(IconId::new(CHEVRON)))
    }
}

impl Component for SelectFace {
    const ROLE: Role = Role::LABEL;
    const INPUT: craft_behavior::Input = craft_behavior::Input::Display;
    const OVERLAYS: bool = false;
    type Style = SelectFaceStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: SelectFaceStyleKey) -> SelectFaceStyle {
        let field = FieldStyle::at(cx);
        let muted = cx.color(cx.tokens.color.text.muted);
        SelectFaceStyle {
            metrics: field.metrics(),
            text: TextLook::new(TypeRole::Body, cx.host_ink(Stateful::new(cx.ink(Ink::OnPanel)))),
            placeholder: TextLook::new(TypeRole::Body, Stateful::new(muted)),
            chevron: IconLook::new(field.glyph().side, cx.host_ink(Stateful::new(cx.ink(Ink::OnPanel)))),
            field,
        }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let text = cx.layout(&TextRun::new(cx.resolved.label).role(cx.style.text.role)).size();
        Vec2::new(text.x + reserve(&cx.style.field, &[], &[Self::chevron()]), text.y)
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let chevron = Self::chevron();
        let placed = place(cx.content_rect, &cx.style.field, &[], std::slice::from_ref(&chevron), cx.flow.is_rtl());
        let look = if self.placeholder { &cx.style.placeholder } else { &cx.style.text };
        let run = TextRun::new(cx.resolved.label).role(look.role).elide().color(cx.resolve(&look.color));
        ui.draw().text(placed.text, &run);
        if let Some(rect) = placed.trailing.first() {
            chevron.paint(ui, *rect, cx.resolve(&cx.style.chevron.color));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
