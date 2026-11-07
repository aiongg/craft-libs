//! `Button`: a push button with an optional leading and trailing icon and a busy state.

use super::face::{self, FaceLook, FaceSpec};
use crate::contract::{Common, Component, Cx, MeasureCx, PaintUi, Pointer as _, Props, Role};
use crate::runtime::commands::CommandRef;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, FocusLook, StyleCx, SurfaceLook};
use craft_behavior::{IconId, Metrics, Sense};
use craft_theme::{
    Variant,
    variant::{Emphasis, Tone},
};
use egui::{Response, Vec2};

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
    face: FaceLook,
}

impl ButtonStyle {
    /// The style of a push button in `cx`; the split button shares it.
    pub(super) fn of(cx: &StyleCx<'_>) -> ButtonStyle {
        ButtonStyle {
            surface: face::surface(cx, &cx.variant_look()),
            metrics: cx.control().metrics(),
            focus: face::focus(cx),
            face: FaceLook::of(cx),
        }
    }
}

/// A push button. A label wider than the room is cut with an ellipsis and shown whole in a tooltip. While `loading`
/// a spinner takes the place of the icon (of the label, without an icon) at the same width, the button ignores
/// activation, keeps focus and reports busy.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct Button {
    #[common]
    common: Common,
    /// A leading icon; it mirrors under right-to-left when it has a direction.
    #[prop]
    icon: Option<IconId>,
    /// An icon at the reading end: the chevron of a button that opens a menu.
    #[prop]
    trailing_icon: Option<IconId>,
    /// Working: a spinner, no activation, AccessKit busy.
    #[prop]
    loading: bool,
}

impl Button {
    /// A button showing `label`, which is also its accessible name.
    pub fn new(label: impl Into<Label>) -> Self {
        Button { common: Common::new(), icon: None, trailing_icon: None, loading: false }.label(label)
    }

    /// A button bound to `command`: its label, enabled state and activation follow the command.
    pub fn command(command: impl Into<CommandRef>) -> Self {
        <Button as Props>::command(Button::new(Label::Empty), command)
    }

    fn spec<'a>(&'a self, label: &'a str) -> FaceSpec<'a> {
        FaceSpec { label, role: None, lead: self.icon.as_ref(), trail: self.trailing_icon.as_ref(), loading: self.loading }
    }
}

impl Component for Button {
    const ROLE: Role = Role::BUTTON;
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
        face::paint(ui, cx, &cx.style.face, &self.spec(cx.resolved.label), cx.content_rect);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
