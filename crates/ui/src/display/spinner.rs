//! `Spinner`: the busy mark, an arc that turns once a period and stands still where motion is off. It shows in
//! the flow, or as the face of a part (`ui.face(part, Spinner::new())`) where a component takes a busy state.

use crate::contract::{Access, Animate as _, Common, Component, Cx, MeasureCx, PaintUi, Props as _, Role, Styled as _};
use crate::primitives::{PaintShape, StrokeWidth};
use crate::style::{ComponentStyle, Flags, IconLook, StyleCx};
use craft_behavior::{Metrics, Sense};
use craft_theme::{
    Variant,
    variant::{Emphasis, Tone},
};
use egui::{Pos2, Response, Vec2, vec2};
use std::f32::consts::{PI, TAU};

/// How long the spinner takes for a turn, in seconds.
const PERIOD: f64 = 1.0;
/// The arc the spinner draws, in radians.
const SWEEP: f32 = PI * 4.0 / 3.0;
/// The points of the arc.
const POINTS: usize = 24;

/// The style of a spinner. It accepts the variants of a button, so a spinner shown as a button's face reads in the
/// button's text colour.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[style(variants = [Variant::SECONDARY, Variant::PRIMARY, Variant::DANGER, Variant::look(Emphasis::Outline, Tone::Accent)])]
#[non_exhaustive]
pub struct SpinnerStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The arc: the icon side of the control and the variant's text colour.
    #[style(icon)]
    arc: IconLook,
    /// How far the arc sits inside the square.
    inset: f32,
}

/// The busy mark. Named by its label it is a busy progress indicator; without a label it is decoration, as inside
/// a button that reports busy itself.
#[derive(crate::contract::Component)]
pub struct Spinner {
    #[common]
    common: Common,
}

impl Spinner {
    /// A spinner at the icon side of the control size.
    pub fn new() -> Self {
        Spinner { common: Common::new() }
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Spinner::new()
    }
}

impl Component for Spinner {
    const ROLE: Role = Role::PROGRESS_INDICATOR;
    const ALT_ROLES: &'static [Role] = &[Role::None];
    const INPUT: craft_behavior::Input = craft_behavior::Input::Display;
    const SHOWS_LABEL: bool = false;
    const OVERLAYS: bool = false;
    type Style = SpinnerStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: SpinnerStyleKey) -> SpinnerStyle {
        let control = cx.control();
        SpinnerStyle {
            metrics: Metrics::new().min_size(Vec2::splat(control.height)),
            arc: IconLook::new(control.icon, cx.variant_look().text),
            inset: cx.tokens.space.gap.tight,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(cx.style.arc.side)
    }

    fn sense(&self) -> Sense {
        super::display_sense(&self.common, true)
    }

    fn flags(&self) -> Flags {
        Flags::BUSY
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        if cx.resolved.label.is_empty() {
            access.role(Role::None);
        } else {
            access.busy(true);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let centre = cx.rect.center();
        let radius = (cx.rect.width().min(cx.rect.height()) / 2.0 - cx.style.inset).max(0.0);
        let start = cx.phase(PERIOD, None).map_or(0.0, |turn| turn * TAU);
        let points: Vec<Pos2> = (0..=POINTS)
            .map(|at| {
                let angle = start + SWEEP * at as f32 / POINTS as f32;
                centre + radius * vec2(angle.cos(), angle.sin())
            })
            .collect();
        ui.draw().stroke(&PaintShape::Path(points), StrokeWidth::Thick, cx.resolve(&cx.style.arc.color));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
