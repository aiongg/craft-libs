//! `Link`: text in the accent colour, always underlined, that reports a click.

use crate::contract::{Access, Common, Component, Cx, MeasureCx, PaintUi, Pointer as _, Role, Styled as _};
use crate::primitives::{PaintShape, StrokeWidth, TextRun};
use crate::style::{ComponentStyle, FocusLook, StyleColor, StyleCx, Stateful, TypeRole};
use craft_behavior::{CursorIcon, CursorPriority, Flow, Metrics};
use craft_theme::{Contrast, States};
use egui::{Pos2, Response, Vec2};

/// The style of a link.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct LinkStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The text and underline colour: `accent.solid`, and `accent.text` in high contrast, which meets the text contrast
    /// that `accent.solid` does not.
    color: Stateful<StyleColor>,
    /// How far the underline sits above the bottom of the text line.
    underline: f32,
}

/// A link: drawn in the accent colour in a type role (body by default) and underlined, so colour is not its only cue.
/// The caller acts on `clicked()`; Enter and Space activate it. AccessKit reports the role `Link`, with the URL when
/// it has one.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct Link {
    #[common]
    common: Common,
    /// The URL AccessKit reports for the link.
    #[prop(into)]
    url: Option<String>,
    /// The type role, to match the text around it.
    #[prop]
    role: TypeRole,
}

impl Link {
    /// A link labelled `label`, in body type.
    pub fn new(label: impl Into<crate::runtime::env::Label>) -> Self {
        use crate::contract::Props as _;
        Link { common: Common::new(), url: None, role: TypeRole::Body }.label(label)
    }
}

impl Component for Link {
    const ROLE: Role = Role::LINK;
    type Style = LinkStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: LinkStyleKey) -> LinkStyle {
        let colors = &cx.tokens.color;
        let ink = if cx.tokens.appearance.contrast == Contrast::High { colors.accent.text } else { colors.accent.solid };
        let color = cx.color(ink);
        LinkStyle {
            metrics: Metrics::new(),
            focus: crate::controls::focus_look(cx),
            color: Stateful::new(color).off(States::new(color, color, color, cx.color(colors.text.disabled))),
            underline: cx.tokens.stroke.thin / 2.0,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        cx.layout(&TextRun::new(cx.resolved.label).role(self.role)).size()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if cx.hit() {
            cx.cursor(CursorIcon::PointingHand, CursorPriority::Part);
        }
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        if let Some(url) = &self.url {
            access.url(url.as_str());
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let color = cx.resolve(&cx.style.color);
        let draw = ui.draw();
        let layout = draw.text(cx.content_rect, &TextRun::new(cx.resolved.label).role(self.role).color(color));
        let line = Flow::new(cx.content_rect, cx.flow.is_rtl()).take_start(layout.size().x).0;
        let y = cx.content_rect.top() + layout.size().y - cx.style.underline;
        draw.stroke(&PaintShape::Path(vec![Pos2::new(line.left(), y), Pos2::new(line.right(), y)]), StrokeWidth::Thin, color);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
