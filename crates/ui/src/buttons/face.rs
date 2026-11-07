//! What a push or toggle button shows: a leading icon (or spinner), a label and a trailing icon, centred in the
//! button, and the look parts the buttons share.

use crate::contract::{Animate as _, Component, Cx, MeasureCx, PaintUi, Styled as _};
use crate::primitives::{IconSpec, PaintShape, StrokeWidth, TextRun};
use crate::style::{FocusLook, Stateful, StyleColor, StyleCx, SurfaceKind, SurfaceLook, TypeRole, VariantParts};
use craft_behavior::{Flow, IconId};
use craft_theme::{
    Variant,
    variant::{Emphasis, Tone},
};
use egui::{Pos2, Rect, Vec2, pos2, vec2};
use std::f32::consts::{PI, TAU};

/// How long the spinner takes for a turn, in seconds.
const SPIN_PERIOD: f64 = 1.0;
/// The arc the spinner draws, in radians.
const SPIN_SWEEP: f32 = PI * 4.0 / 3.0;
/// The points of the spinner's arc.
const SPIN_POINTS: usize = 24;

/// The parts of a button's look that the face paints.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct FaceLook {
    /// The label and icon colour per state.
    pub text: Stateful<StyleColor>,
    /// The side of an icon.
    pub icon: f32,
    /// The gap between icon and label.
    pub gap: f32,
    /// The radius of the spinner's ring.
    pub ring: f32,
}

impl FaceLook {
    /// The face look of the instance's variant: the variant's text colours, the control's icon and gap.
    pub(super) fn of(cx: &StyleCx<'_>) -> FaceLook {
        let control = cx.control();
        FaceLook { text: cx.variant_look().text, icon: control.icon, gap: control.gap, ring: control.icon / 2.0 - cx.tokens.space.gap.tight }
    }
}

/// What a face shows.
pub(super) struct FaceSpec<'a> {
    pub label: &'a str,
    pub role: Option<TypeRole>,
    pub lead: Option<&'a IconId>,
    pub trail: Option<&'a IconId>,
    /// A spinner takes the place of the leading icon, or of the label when there is no leading icon.
    pub loading: bool,
}

impl FaceSpec<'_> {
    fn run(&self) -> TextRun<'_> {
        let run = TextRun::new(self.label).elide();
        match self.role {
            Some(role) => run.role(role),
            None => run,
        }
    }

    fn icons(&self) -> usize {
        usize::from(self.lead.is_some()) + usize::from(self.trail.is_some())
    }

    /// The gaps between the parts that are shown.
    fn gaps(&self) -> usize {
        (self.icons() + usize::from(!self.label.is_empty())).saturating_sub(1)
    }
}

/// The surface of a button from its variant: raised unless ghost, the variant's fill and edge.
pub(super) fn surface(cx: &StyleCx<'_>, parts: &VariantParts) -> SurfaceLook {
    let mut look = SurfaceLook::new(parts.fill.clone()).radius(parts.radius);
    if !matches!(cx.variant, Variant::Look { emphasis: Emphasis::Ghost, .. }) {
        look = look.kind(SurfaceKind::Raised).kind_down(SurfaceKind::Pressed);
    }
    match &parts.edge {
        Some(edge) => look.border(edge.clone(), cx.tokens.stroke.thin),
        None => look,
    }
}

/// The look of a control that is on: the soft accent fill inside an accent edge, with accent text.
pub(super) fn on_parts(cx: &StyleCx<'_>) -> VariantParts {
    cx.with_variant(Variant::look(Emphasis::Soft, Tone::Accent)).variant_look()
}

/// The focus ring of a button.
pub(super) fn focus(cx: &StyleCx<'_>) -> FocusLook {
    crate::controls::focus_look(cx)
}

/// The size of the face: its parts and the gaps between them.
pub(super) fn measure<C: Component>(cx: &MeasureCx<'_, C>, spec: &FaceSpec<'_>) -> Vec2 {
    let text = if spec.label.is_empty() { Vec2::ZERO } else { cx.layout(&spec.run()).size() };
    let icon = cx.icon();
    Vec2::new(text.x + icon * spec.icons() as f32 + cx.gap() * spec.gaps() as f32, text.y.max(icon))
}

/// Paints the face centred in `area`: the leading icon at the reading start, the trailing icon at the reading end.
pub(super) fn paint<C: Component>(ui: &PaintUi<'_>, cx: &Cx<'_, C>, look: &FaceLook, spec: &FaceSpec<'_>, area: Rect) {
    let color = cx.resolve(&look.text);
    let draw = ui.draw();
    let fixed = look.icon * spec.icons() as f32 + look.gap * spec.gaps() as f32;
    let run = spec.run().color(color);
    let text = if spec.label.is_empty() { None } else { Some(draw.layout(&run, (area.width() - fixed).max(0.0))) };
    let text_width = text.as_ref().map_or(0.0, |text| text.size().x);
    let width = fixed + text_width;
    let (_, mut rest) = Flow::new(area, cx.flow.is_rtl()).take_start(((area.width() - width) / 2.0).max(0.0));
    let mut take = |extent: f32| {
        let (slot, next) = rest.take_start(extent);
        rest = next;
        slot
    };
    let centre_y = area.center().y;
    let icon_at = |slot: Rect| Rect::from_center_size(pos2(slot.center().x, centre_y), Vec2::splat(look.icon));
    let spinner = |slot: Rect| ring(&draw, cx, icon_at(slot).center(), look.ring, color);
    let mut gap = false;
    if let Some(lead) = spec.lead {
        let slot = take(look.icon);
        if spec.loading {
            spinner(slot);
        } else {
            draw.icon(icon_at(slot), &IconSpec::svg(lead.clone()).color(color));
        }
        gap = true;
    }
    if let Some(text) = &text {
        if gap {
            take(look.gap);
        }
        let slot = take(text_width);
        if spec.loading && spec.lead.is_none() {
            spinner(slot);
        } else {
            draw.galley(pos2(slot.min.x, centre_y - text.size().y / 2.0), text);
        }
        gap = true;
    }
    if let Some(trail) = spec.trail {
        if gap {
            take(look.gap);
        }
        let slot = take(look.icon);
        draw.icon(icon_at(slot), &IconSpec::svg(trail.clone()).color(color));
    }
}

/// A turning arc around `centre`: the busy mark. It stands still where motion is off.
fn ring<C: Component>(draw: &crate::primitives::Draw<'_>, cx: &Cx<'_, C>, centre: Pos2, radius: f32, color: StyleColor) {
    let start = cx.phase(SPIN_PERIOD, None).map_or(0.0, |turn| turn * TAU);
    let points: Vec<Pos2> = (0..=SPIN_POINTS)
        .map(|at| {
            let angle = start + SPIN_SWEEP * at as f32 / SPIN_POINTS as f32;
            centre + radius * vec2(angle.cos(), angle.sin())
        })
        .collect();
    draw.stroke(&PaintShape::Path(points), StrokeWidth::Thick, color);
}
