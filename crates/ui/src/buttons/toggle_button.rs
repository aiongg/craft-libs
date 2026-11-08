//! `ToggleButton`: a button that stays pressed while on.

use super::face::{self, FaceLook, FaceSpec};
use crate::contract::{Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, PaintUi, Pointer as _, Props, Role};
use crate::primitives::IconSpec;
use crate::runtime::commands::CommandRef;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, FocusLook, IconLook, StyleCx, SurfaceLook, TextLook, TypeRole};
use craft_behavior::{Metrics, Sense};
use craft_theme::{Flags, Mix as _, Variant};
use egui::Vec2;

/// How far between the off fill and the on fill a mixed value sits.
const MIXED_SHARE: f32 = 0.5;

/// The style of a toggle button.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[style(variants = [Variant::SECONDARY])]
#[non_exhaustive]
pub struct ToggleButtonStyle {
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

/// A secondary-looking button that stays pressed while on: it takes the accent fill and edge, and its raised surface
/// draws pressed in. A click flips the value; AccessKit reports the button toggled. An `icon_only` button shows its
/// icon on a square and keeps the label as its accessible name.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct ToggleButton<'a> {
    #[common]
    common: Common,
    /// `None` for a button bound to a toggle command, which owns the state.
    value: Option<Bind<'a, bool>>,
    /// A leading icon.
    #[prop(into)]
    icon: Option<IconSpec<'a>>,
    /// The leading icon while the button is on, in place of `icon`.
    #[prop(into)]
    on_icon: Option<IconSpec<'a>>,
    /// An icon at the reading end, mirrored right to left.
    #[prop(into)]
    trailing_icon: Option<IconSpec<'a>>,
    /// The type role of the label, for a path or a value in `Mono`; the label role by default.
    #[prop]
    role: Option<TypeRole>,
    /// Shows only the icon, on a square; the label stays the accessible name.
    #[style_key]
    #[prop(flag)]
    icon_only: bool,
}

impl<'a> ToggleButton<'a> {
    /// A button named `label` that edits `value`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, bool>) -> Self {
        ToggleButton { common: Common::new(), value: Some(value), icon: None, on_icon: None, trailing_icon: None, role: None, icon_only: false }
            .label(label)
    }

    /// A button bound to a toggle `command`: its label, enabled state and checked state follow the command.
    pub fn command(command: impl Into<CommandRef>) -> Self {
        let button =
            ToggleButton { common: Common::new(), value: None, icon: None, on_icon: None, trailing_icon: None, role: None, icon_only: false };
        <ToggleButton as Props>::command(button, command)
    }

    fn spec<'s>(&'s self, label: &'s str, on: bool) -> FaceSpec<'s> {
        let lead = if on { self.on_icon.as_ref().or(self.icon.as_ref()) } else { self.icon.as_ref() };
        FaceSpec { label: if self.icon_only { "" } else { label }, role: self.role, lead, trail: self.trailing_icon.as_ref(), loading: false }
    }
}

impl Component for ToggleButton<'_> {
    const ROLE: Role = Role::BUTTON;
    const PARTS: bool = false;
    const OVERLAYS: bool = false;
    type Style = ToggleButtonStyle;
    type Output = EditResponse<bool>;

    fn shows_label(&self) -> bool {
        !self.icon_only
    }

    fn style(cx: &StyleCx<'_>, key: ToggleButtonStyleKey) -> ToggleButtonStyle {
        let control = cx.control();
        let (off, on) = (cx.variant_look(), face::on_parts(cx));
        let mut surface = face::surface(cx, &off);
        // A mixed value sits between off and on: the fill halfway, the ink of the off look.
        let mixed_fill = off.fill.off.rest.mix(on.fill.off.rest, MIXED_SHARE);
        surface.fill = off.fill.clone().on(on.fill.off).when(Flags::MIXED, mixed_fill);
        surface.border = off.edge.clone().map(|edge| edge.on(on.edge.as_ref().map_or(on.fill.off, |edge| edge.off)));
        let (mut text, mut icon) = face::ink(cx);
        text.color = text.color.on(on.text.off).when(Flags::MIXED, off.text.off.rest);
        icon.color = icon.color.on(on.text.off).when(Flags::MIXED, off.text.off.rest);
        let metrics = if key.icon_only { Metrics::new().min_size(Vec2::splat(control.height)) } else { control.metrics() };
        ToggleButtonStyle { surface, metrics, focus: face::focus(cx), text, icon, face: FaceLook::of(cx) }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        face::measure(cx, &self.spec(cx.resolved.label, true))
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn bind(&mut self) -> Option<BindMut<'_, bool>> {
        self.value.as_mut().map(Bind::reborrow)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.join_roving();
        if cx.activation().is_some() && self.value.is_some() {
            let on = cx.value().copied().unwrap_or(false);
            cx.set(!on);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let on = cx.value().copied().unwrap_or(false);
        face::paint(ui, cx, (&cx.style.text, &cx.style.icon), &cx.style.face, &self.spec(cx.resolved.label, on), cx.content_rect);
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<bool> {
        cx.edit_response()
    }
}
