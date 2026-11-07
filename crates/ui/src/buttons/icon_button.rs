//! `IconButton`: a square ghost button that shows only an icon.

use super::face::{self, tinted};
use crate::contract::{Common, Component, Cx, MeasureCx, PaintUi, Props, Role, Styled as _};
use crate::primitives::{IconSpec, PaintShape, StrokeWidth};
use crate::runtime::commands::CommandRef;
use crate::style::{ComponentStyle, DataColor, FocusLook, IconLook, StyleColor, StyleCx, SurfaceLook};
use craft_behavior::{Metrics, Sense};
use craft_theme::{
    Srgba, Variant,
    variant::{Emphasis, Tone},
};
use egui::{Color32, Pos2, Response, Vec2};

/// The badge dot's diameter in icon sides.
const BADGE_RATIO: f32 = 0.4;

/// The style of an icon button.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[style(variants = [
    Variant::GHOST,
    Variant::look(Emphasis::Ghost, Tone::Accent),
    Variant::look(Emphasis::Ghost, Tone::Info),
    Variant::look(Emphasis::Ghost, Tone::Good),
    Variant::look(Emphasis::Ghost, Tone::Warn),
    Variant::look(Emphasis::Ghost, Tone::Bad),
])]
#[non_exhaustive]
pub struct IconButtonStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The glyph: its side, and its colour per state (the on look takes the accent text).
    #[style(icon)]
    glyph: IconLook,
    /// The badge dot's diameter.
    badge: f32,
    /// The ring around the badge dot.
    badge_edge: StyleColor,
}

/// A square button showing only an icon. Its label is its accessible name; a tooltip is the caller's. A selected
/// button (`selected(true)`) shows the on look: the host owns the state and a click is reported in the response.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct IconButton<'a> {
    #[common]
    common: Common,
    icon: IconSpec<'a>,
    /// Whether the button is a Tab stop; off for a button that repeats a key of the control it sits in, as a search
    /// field's clear button repeats Escape.
    #[prop]
    focusable: bool,
    /// A small dot of the colour at the end-bottom corner of the glyph, ringed so it shows on any fill. The colour
    /// is the user's content, so it has no contrast pair.
    #[prop(into)]
    badge: Option<Srgba>,
}

impl<'a> IconButton<'a> {
    /// An icon button showing `icon`; `label` it with the accessible name.
    pub fn new(icon: impl Into<IconSpec<'a>>) -> Self {
        IconButton { common: Common::new(), icon: icon.into(), focusable: true, badge: None }
    }

    /// An icon button bound to `command`: its label, enabled state and activation follow the command.
    pub fn command(icon: impl Into<IconSpec<'a>>, command: impl Into<CommandRef>) -> Self {
        <IconButton as Props>::command(IconButton::new(icon), command)
    }
}

impl Component for IconButton<'_> {
    const ROLE: Role = Role::BUTTON;
    const SHOWS_LABEL: bool = false;
    const OVERLAYS: bool = false;
    type Style = IconButtonStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: IconButtonStyleKey) -> IconButtonStyle {
        let control = cx.control();
        let (off, on) = (cx.variant_look(), face::on_parts(cx));
        let mut surface = face::surface(cx, &off);
        surface.fill = off.fill.clone().on(on.fill.off);
        surface.border = off.edge.clone().map(|edge| edge.on(on.edge.as_ref().map_or(on.fill.off, |edge| edge.off)));
        IconButtonStyle {
            surface,
            metrics: Metrics::new().min_size(Vec2::splat(control.height)),
            focus: face::focus(cx),
            glyph: IconLook::new(control.icon, off.text.clone().on(on.text.off)),
            badge: control.icon * BADGE_RATIO,
            badge_edge: cx.color(cx.tokens.color.line.mark),
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(cx.style.glyph.side)
    }

    fn sense(&self) -> Sense {
        if self.focusable { Sense::CLICK | Sense::FOCUS } else { Sense::CLICK }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.join_roving();
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let glyph = egui::Rect::from_center_size(cx.rect.center(), Vec2::splat(cx.style.glyph.side));
        let draw = ui.draw();
        draw.icon(glyph, &tinted(&self.icon, cx.resolve(&cx.style.glyph.color)));
        if let Some(badge) = self.badge {
            let radius = cx.style.badge / 2.0;
            let x = if cx.flow.is_rtl() { glyph.left() + radius } else { glyph.right() - radius };
            let centre = Pos2::new(x, glyph.bottom() - radius);
            draw.data_fill(&PaintShape::Circle(centre, radius), DataColor::new(Color32::from(badge)));
            draw.stroke(&PaintShape::Circle(centre, radius), StrokeWidth::Thin, cx.style.badge_edge);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
