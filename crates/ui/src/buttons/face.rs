//! What a push or toggle button shows: a leading icon (or spinner), a label and a trailing icon, centred in the
//! button, and the look parts the buttons share.

use crate::contract::{Component, Cx, MeasureCx, PaintUi, Styled as _};
use crate::primitives::{IconSpec, TextRun};
use crate::style::{FocusLook, IconLook, StyleColor, StyleCx, SurfaceKind, SurfaceLook, TextLook, TypeRole, VariantParts};
use craft_behavior::Flow;
use craft_theme::{
    Stateful, States, Variant,
    variant::{Emphasis, Tone},
};
use egui::{Rect, Vec2, pos2};

/// The parts of a button's look that the face paints besides its text and icon looks.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct FaceLook {
    /// The gap between icon and label.
    pub gap: f32,
}

impl FaceLook {
    /// The face look of the instance's variant: the control's gap.
    pub(super) fn of(cx: &StyleCx<'_>) -> FaceLook {
        FaceLook { gap: cx.control().gap }
    }
}

/// The label and icon looks of the instance's variant: the variant's text colours at the control's icon side.
pub(super) fn ink(cx: &StyleCx<'_>) -> (TextLook, IconLook) {
    let color = cx.variant_look().text;
    (TextLook::new(TypeRole::Label, color.clone()), IconLook::new(cx.control().icon, color))
}

/// What a face shows.
pub(super) struct FaceSpec<'a> {
    pub label: &'a str,
    pub role: Option<TypeRole>,
    pub lead: Option<&'a IconSpec<'a>>,
    pub trail: Option<&'a IconSpec<'a>>,
    /// A spinner takes the place of the leading icon, or of the label when there is no leading icon: the face keeps
    /// the slot empty and the button shows a `Spinner` there.
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

/// The look of a control that is on: the accent's soft fill with its text colour pushed to the contrast text needs,
/// inside the soft accent edge.
pub(super) fn on_parts(cx: &StyleCx<'_>) -> VariantParts {
    let soft = cx.with_variant(Variant::look(Emphasis::Soft, Tone::Accent)).variant_look();
    let colors = &cx.tokens.color;
    let fill = cx.color(colors.accent.soft);
    let ink = cx.color(craft_theme::ensure_contrast(colors.accent.text, colors.accent.soft, craft_theme::text_contrast_min(cx.contrast())));
    VariantParts { fill: Stateful::new(fill), text: Stateful::new(ink).off(States::new(ink, ink, ink, soft.text.off.disabled)), ..soft }
}

/// The focus ring of a button.
pub(super) fn focus(cx: &StyleCx<'_>) -> FocusLook {
    cx.focus_look()
}

/// The size of the face: its parts and the gaps between them.
pub(super) fn measure<C: Component>(cx: &MeasureCx<'_, C>, spec: &FaceSpec<'_>) -> Vec2 {
    let text = if spec.label.is_empty() { Vec2::ZERO } else { cx.layout(&spec.run()).size() };
    let icon = cx.icon();
    Vec2::new(text.x + icon * spec.icons() as f32 + cx.gap() * spec.gaps() as f32, text.y.max(icon))
}

/// `icon` in `color`, unless the spec names a colour of its own.
pub(super) fn tinted<'a>(icon: &IconSpec<'a>, color: StyleColor) -> IconSpec<'a> {
    let icon = icon.clone();
    if icon.color.is_some() { icon } else { icon.color(color) }
}

/// The slots of the parts a face shows, centred in an area.
struct Slots {
    lead: Option<Rect>,
    text: Option<Rect>,
    trail: Option<Rect>,
}

/// Places the icons of `side` and a label `text_width` wide with `gap` between them, centred in `area`: the leading
/// icon at the reading start, the trailing icon at the reading end.
fn slots(spec: &FaceSpec<'_>, side: f32, gap: f32, text_width: f32, area: Rect, rtl: bool) -> Slots {
    let width = side * spec.icons() as f32 + gap * spec.gaps() as f32 + text_width;
    let (_, mut rest) = Flow::new(area, rtl).take_start(((area.width() - width) / 2.0).max(0.0));
    let mut take = |extent: f32| {
        let (slot, next) = rest.take_start(extent);
        rest = next;
        slot
    };
    let mut slots = Slots { lead: None, text: None, trail: None };
    let mut gapped = false;
    if spec.lead.is_some() {
        slots.lead = Some(take(side));
        gapped = true;
    }
    if !spec.label.is_empty() {
        if gapped {
            take(gap);
        }
        slots.text = Some(take(text_width));
        gapped = true;
    }
    if spec.trail.is_some() {
        if gapped {
            take(gap);
        }
        slots.trail = Some(take(side));
    }
    slots
}

/// The square an icon of `side` takes in `slot`, vertically centred in `area`.
fn icon_at(slot: Rect, area: Rect, side: f32) -> Rect {
    Rect::from_center_size(pos2(slot.center().x, area.center().y), Vec2::splat(side))
}

/// Paints the face centred in `area`: the leading icon at the reading start, the trailing icon at the reading end.
pub(super) fn paint<C: Component>(ui: &PaintUi<'_>, cx: &Cx<'_, C>, ink: (&TextLook, &IconLook), look: &FaceLook, spec: &FaceSpec<'_>, area: Rect) {
    let (text_look, icon_look) = ink;
    let (text_color, icon_color, side) = (cx.resolve(&text_look.color), cx.resolve(&icon_look.color), icon_look.side);
    let draw = ui.draw();
    let fixed = side * spec.icons() as f32 + look.gap * spec.gaps() as f32;
    let run = spec.run().color(text_color);
    let text = if spec.label.is_empty() { None } else { Some(draw.layout(&run, (area.width() - fixed).max(0.0))) };
    let text_width = text.as_ref().map_or(0.0, |text| text.size().x);
    let slots = slots(spec, side, look.gap, text_width, area, cx.flow.is_rtl());
    if let (Some(lead), Some(slot)) = (spec.lead, slots.lead)
        && !spec.loading
    {
        draw.icon(icon_at(slot, area, side), &tinted(lead, icon_color));
    }
    if let (Some(text), Some(slot)) = (&text, slots.text)
        && !(spec.loading && spec.lead.is_none())
    {
        draw.galley(pos2(slot.min.x, area.center().y - text.size().y / 2.0), text);
    }
    if let (Some(trail), Some(slot)) = (spec.trail, slots.trail) {
        draw.icon(icon_at(slot, area, side), &tinted(trail, icon_color));
    }
}

/// Where the busy spinner of a loading face goes: the square of the icon side in the leading icon's slot, or in the
/// label's slot when there is no leading icon. `None` when the face has neither.
pub(super) fn spinner_rect<C: Component>(
    cx: &MeasureCx<'_, C>,
    spec: &FaceSpec<'_>,
    look: &FaceLook,
    side: f32,
    area: Rect,
    rtl: bool,
) -> Option<Rect> {
    let fixed = side * spec.icons() as f32 + look.gap * spec.gaps() as f32;
    let text_width = if spec.label.is_empty() { 0.0 } else { cx.layout_at(&spec.run(), (area.width() - fixed).max(0.0)).size().x };
    let slots = slots(spec, side, look.gap, text_width, area, rtl);
    let slot = if spec.lead.is_some() { slots.lead } else { slots.text }?;
    Some(icon_at(slot, area, side))
}
