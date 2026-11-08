//! `Button`: a push button with an optional leading and trailing icon and a busy state.

use super::face::{self, FaceLook, FaceSpec};
use crate::contract::{Common, Component, Cx, MeasureCx, PaintUi, PartsCx, Pointer as _, Props, Role};
use crate::display::Spinner;
use crate::primitives::IconSpec;
use crate::runtime::commands::CommandRef;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, FocusLook, IconLook, StyleCx, SurfaceLook, TextLook};
use craft_behavior::{Metrics, PartKey, Sense};
use craft_theme::{
    Variant,
    variant::{Emphasis, Tone},
};
use egui::{Rect, Response, Vec2};

/// The part a loading button shows its spinner in.
const SPINNER: PartKey = PartKey::new("spinner");

/// The style of a push button.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[style(variants = [Variant::SECONDARY, Variant::PRIMARY, Variant::GHOST, Variant::DANGER, Variant::look(Emphasis::Outline, Tone::Accent)])]
#[non_exhaustive]
pub struct ButtonStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The label.
    #[style(text)]
    text: TextLook,
    /// The leading and trailing icons.
    #[style(icon)]
    icon: IconLook,
    face: FaceLook,
}

impl ButtonStyle {
    /// The style of a push button in `cx`; the split button shares it.
    pub(super) fn of(cx: &StyleCx<'_>) -> ButtonStyle {
        let (text, icon) = face::ink(cx);
        ButtonStyle {
            surface: face::surface(cx, &cx.variant_look()),
            metrics: cx.control().metrics(),
            focus: face::focus(cx),
            text,
            icon,
            face: FaceLook::of(cx),
        }
    }
}

/// A push button. A label wider than the room is cut with an ellipsis and shown whole in a tooltip. While `loading`
/// a spinner takes the place of the icon (of the label, without an icon) at the same width, the button ignores
/// activation, keeps focus and reports busy.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct Button<'a> {
    #[common]
    common: Common,
    /// A leading icon; it mirrors under right-to-left when it has a direction.
    #[prop(into)]
    icon: Option<IconSpec<'a>>,
    /// An icon at the reading end: the chevron of a button that opens a menu.
    #[prop(into)]
    trailing_icon: Option<IconSpec<'a>>,
    /// Working: a spinner, no activation, AccessKit busy.
    #[prop]
    loading: bool,
    /// The text the face paints in place of the label, which stays the accessible name: the label with its access
    /// letter marked.
    #[prop(into)]
    painted: Option<Label>,
}

impl<'a> Button<'a> {
    /// A button showing `label`, which is also its accessible name.
    pub fn new(label: impl Into<Label>) -> Self {
        Button { common: Common::new(), icon: None, trailing_icon: None, loading: false, painted: None }.label(label)
    }

    /// A button bound to `command`: its label, enabled state and activation follow the command.
    pub fn command(command: impl Into<CommandRef>) -> Self {
        <Button as Props>::command(Button::new(Label::Empty), command)
    }

    fn spec<'s>(&'s self, label: &'s str) -> FaceSpec<'s> {
        let label = self.painted.as_ref().map_or(label, Label::as_str);
        FaceSpec { label, role: None, lead: self.icon.as_ref(), trail: self.trailing_icon.as_ref(), loading: self.loading }
    }
}

impl Component for Button<'_> {
    const ROLE: Role = Role::BUTTON;
    const OVERLAYS: bool = false;
    type Style = ButtonStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: ButtonStyleKey) -> ButtonStyle {
        ButtonStyle::of(cx)
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        face::measure(cx, &self.spec(cx.resolved.label))
    }

    fn sense(&self) -> Sense {
        if self.loading { Sense::FOCUS } else { Sense::CLICK | Sense::FOCUS }
    }

    /// A loading button declares the slot of its spinner, which paints as the face of that part.
    fn parts(&mut self, _: Rect, cx: &mut PartsCx<'_, Self>) {
        if !self.loading {
            return;
        }
        let (area, rtl) = (cx.content_rect, cx.flow.is_rtl());
        let spinner = face::spinner_rect(cx, &self.spec(cx.resolved.label), &cx.style.face, cx.style.icon.side, area, rtl);
        if let Some(rect) = spinner {
            cx.part(SPINNER, rect);
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.join_roving();
    }

    fn activated(&self, cx: &Cx<'_, Self>) -> bool {
        !self.loading && cx.activation().is_some()
    }

    fn flags(&self) -> crate::style::Flags {
        if self.loading { crate::style::Flags::BUSY } else { crate::style::Flags::empty() }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        face::paint(ui, cx, (&cx.style.text, &cx.style.icon), &cx.style.face, &self.spec(cx.resolved.label), cx.content_rect);
        if cx.part_if_declared(SPINNER).is_some() {
            ui.face(SPINNER, Spinner::new());
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
