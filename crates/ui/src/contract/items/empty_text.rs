//! The empty-state text of a collection: centred muted text that is its own label node.

use crate::contract::{Common, Component, Cx, MeasureCx, PaintUi, Props as _, Role};
use crate::primitives::{HAlign, Span, TextRun, Wrap};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx, TypeRole};
use craft_behavior::{Metrics, Sense};
use egui::{Response, Vec2};

/// The style of the empty-state text.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
struct EmptyTextStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// The text a collection with no rows shows (`CollectionProps::empty`).
#[derive(crate::contract::Component)]
#[component(sample = EmptyText::sample)]
pub(super) struct EmptyText {
    #[common]
    common: Common,
}

impl EmptyText {
    #[cfg(feature = "testing")]
    fn sample() -> Self {
        EmptyText::new(Label::from("No items"))
    }

    pub(super) fn new(text: Label) -> Self {
        EmptyText { common: Common::new() }.label(text)
    }

    fn run<'t>(text: &'t str) -> TextRun<'t> {
        TextRun::spans(vec![Span::new(text).muted()]).role(TypeRole::Label).wrap(Wrap::Wrap).align(HAlign::Center)
    }
}

impl Component for EmptyText {
    const ROLE: Role = Role::LABEL;
    const PARTS: bool = false;
    const STOPS: bool = false;
    type Style = EmptyTextStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: EmptyTextStyleKey) -> EmptyTextStyle {
        EmptyTextStyle { metrics: Metrics::new() }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        cx.layout(&Self::run(cx.resolved.label)).size()
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.draw().text(cx.rect, &Self::run(cx.core.resolved.label));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.core.response.clone()
    }
}
