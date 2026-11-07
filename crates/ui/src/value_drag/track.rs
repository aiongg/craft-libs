//! The track and fill look shared by `Slider`, `RangeSlider` and `ThumbRow`: a rounded groove across the control, with
//! the part between two points along it filled.

use super::slider::Scale;
use crate::contract::{PaintUi, Show};
use crate::primitives::{Draw, PaintShape, TextRun};
use crate::runtime::env::Label;
use crate::style::{StyleColor, StyleCx, TextLook, TypeRole};
use craft_behavior::{Flow, PartKey, PartRef};
use craft_theme::{Rgba, Stateful, States};
use egui::{CornerRadius, Pos2, Rect, Vec2};

/// The part a track face paints as: the groove across the middle of the control.
pub(crate) const GROOVE: PartKey = PartKey::new("groove");

/// A colour that rests, lights on hover and press, and takes the disabled fill when disabled.
pub(crate) fn states(cx: &StyleCx<'_>, rest: Rgba, hover: Rgba, pressed: Rgba) -> Stateful<StyleColor> {
    let (rest, hover, pressed, disabled) = (cx.color(rest), cx.color(hover), cx.color(pressed), cx.color(cx.tokens.color.state.disabled_fill));
    Stateful::new(rest).off(States::new(rest, hover, pressed, disabled))
}

/// The groove and the fill over it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TrackLook {
    /// The groove.
    pub groove: Stateful<StyleColor>,
    /// The part of the groove the value fills.
    pub filled: Stateful<StyleColor>,
    /// The thickness of the groove.
    pub thickness: f32,
    /// The length of a mark or tick below the groove.
    pub mark_length: f32,
    /// The thickness of a mark or tick.
    pub mark_width: f32,
    /// The colour of marks and ticks.
    pub mark_color: Stateful<StyleColor>,
    /// How near a mark, along the track, a drag snaps to it.
    pub snap: f32,
}

/// The look of the labels under labelled marks: caption type in the muted text colour.
pub(crate) fn mark_text(cx: &StyleCx<'_>) -> TextLook {
    TextLook::new(TypeRole::Caption, Stateful::new(cx.color(cx.tokens.color.text.muted)))
}

impl TrackLook {
    /// The look of the control at the ambient size: a sunken groove with an accent fill.
    pub fn new(cx: &StyleCx<'_>) -> Self {
        let colors = &cx.tokens.color;
        let (sunken, accent) = (colors.surface.sunken, &colors.accent);
        TrackLook {
            groove: states(cx, sunken, sunken, sunken),
            filled: states(cx, accent.strong, accent.strong_hover, accent.strong_pressed),
            thickness: cx.tokens.stroke.thick,
            mark_length: cx.tokens.space.gap.tight,
            mark_width: cx.tokens.stroke.thin,
            mark_color: Stateful::new(cx.color(colors.line.mark)),
            snap: cx.control().icon / 2.0,
        }
    }

    /// The groove across the middle of `rect`.
    pub fn groove(&self, rect: Rect) -> Rect {
        Rect::from_center_size(rect.center(), Vec2::new(rect.width(), self.thickness))
    }
}

/// A caller's look for the groove: a component shown as the face of the `GROOVE` part, such as a colour gradient. It
/// replaces the groove and the fill.
pub struct TrackFace<'a>(FaceFn<'a>);

type FaceFn<'a> = Box<dyn Fn(&mut PaintUi<'_>, PartRef) + 'a>;

impl<'a> TrackFace<'a> {
    /// A track painted by `face`.
    pub fn new<C: Show + Clone + 'a>(face: C) -> Self {
        TrackFace(Box::new(move |ui, part| ui.face(part, face.clone())))
    }

    pub(crate) fn paint(&self, ui: &mut PaintUi<'_>) {
        (self.0)(ui, GROOVE.into());
    }
}

/// Marks and ticks along a track.
#[derive(Clone, Default)]
pub(crate) struct Marks {
    /// The values with a mark, which a drag snaps to.
    values: Vec<f64>,
    /// The labels under marks, with their values.
    labels: Vec<(f64, Label)>,
    /// Evenly spaced ticks: one more than this, none for 0.
    pub ticks: usize,
    /// The labels resolved and measured by the last `lay_out`.
    shaped: Vec<(f64, String, Vec2)>,
}

impl Marks {
    pub fn set(&mut self, values: &[f64]) {
        self.values = values.iter().copied().filter(|value| value.is_finite()).collect();
    }

    pub fn set_labelled<L: Into<Label>>(&mut self, marks: impl IntoIterator<Item = (f64, L)>) {
        self.labels = marks.into_iter().map(|(value, label)| (value, label.into())).filter(|(value, _)| value.is_finite()).collect();
        self.values = self.labels.iter().map(|(value, _)| *value).collect();
    }

    /// Resolves and measures the labels in the caption role.
    pub fn lay_out(&mut self, look: &TextLook, resolve: impl Fn(&Label) -> Label, measure: impl Fn(&TextRun<'_>) -> Vec2) {
        self.shaped = self
            .labels
            .iter()
            .map(|(value, label)| {
                let text = resolve(label).as_str().to_owned();
                let size = measure(&TextRun::new(text.clone()).role(look.role));
                (*value, text, size)
            })
            .collect();
    }

    /// The room the labels take below the track: their tallest line and the gaps around it; none without labels.
    pub fn below(&self, look: &TrackLook) -> f32 {
        let tallest = self.shaped.iter().map(|(_, _, size)| size.y).fold(0.0, f32::max);
        if self.shaped.is_empty() { 0.0 } else { tallest + 2.0 * look.mark_length }
    }

    /// The part of `rect` the track and thumbs use: all of it, less the room of the labels.
    pub fn area(&self, rect: Rect, look: &TrackLook) -> Rect {
        Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.max.y - self.below(look)))
    }

    /// The marked value whose position along a track of `length` is within the snap distance of `position`.
    pub fn snapped(&self, scale: &Scale, look: &TrackLook, length: f32, position: f64) -> Option<f64> {
        self.values
            .iter()
            .copied()
            .filter(|value| (scale.min..=scale.max).contains(value))
            .map(|value| (value, (scale.position(value) - position).abs() * f64::from(length)))
            .filter(|(_, distance)| *distance <= f64::from(look.snap))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(value, _)| value)
    }
}

/// Where positions lie along a track: the area it runs in, and the room a thumb keeps at each end.
#[derive(Clone, Copy)]
pub(crate) struct Along {
    pub area: Rect,
    pub flow: Flow,
}

impl Along {
    /// The x of `position` (0 to 1) along the track, mirrored under right-to-left.
    fn x(&self, position: f64) -> f32 {
        let half = self.area.height() / 2.0;
        let length = (self.area.width() - self.area.height()).max(0.0);
        self.flow.span(half + position as f32 * length, 0.0).center().x
    }
}

/// The colours of marks and their labels.
pub(crate) struct MarkInk {
    pub marks: StyleColor,
    pub labels: StyleColor,
    pub role: TypeRole,
}

/// Paints the ticks, the marks and the labels under the groove of `along`, the labels kept inside `whole`.
pub(crate) fn paint_marks(draw: &Draw<'_>, look: &TrackLook, marks: &Marks, scale: &Scale, along: Along, whole: Rect, ink: &MarkInk) {
    let top = look.groove(along.area).bottom() + look.mark_length;
    let ticks = (marks.ticks > 0).then(|| (0..=marks.ticks).map(|at| at as f64 / marks.ticks as f64));
    let positions = ticks
        .into_iter()
        .flatten()
        .chain(marks.values.iter().filter(|value| (scale.min..=scale.max).contains(value)).map(|value| scale.position(*value)));
    for position in positions {
        let line = Rect::from_min_size(Pos2::new(along.x(position) - look.mark_width / 2.0, top), Vec2::new(look.mark_width, look.mark_length));
        draw.fill(&PaintShape::Rect(line, CornerRadius::ZERO), ink.marks);
    }
    let label_top = top + 2.0 * look.mark_length;
    for (value, text, size) in &marks.shaped {
        if !(scale.min..=scale.max).contains(value) {
            continue;
        }
        let left = (along.x(scale.position(*value)) - size.x / 2.0).clamp(whole.left(), (whole.right() - size.x).max(whole.left()));
        draw.text(Rect::from_min_size(Pos2::new(left, label_top), *size), &TextRun::new(text.as_str()).role(ink.role).color(ink.labels));
    }
}

/// The part of the groove painted in `color`, from `from` to `to` along the reading direction, measured from the start edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Fill {
    pub color: StyleColor,
    pub from: f32,
    pub to: f32,
}

/// Paints the groove across the middle of `rect` in `groove`, and `fill` over it.
pub(crate) fn paint(draw: &Draw<'_>, look: &TrackLook, rect: Rect, flow: Flow, groove: StyleColor, fill: Option<Fill>) {
    let track = Rect::from_center_size(rect.center(), Vec2::new(rect.width(), look.thickness));
    let round = CornerRadius::same((look.thickness / 2.0).round() as u8);
    draw.fill(&PaintShape::Rect(track, round), groove);
    if let Some(fill) = fill {
        let filled = Flow::new(track, flow.is_rtl()).span(fill.from, (fill.to - fill.from).max(0.0));
        draw.fill(&PaintShape::Rect(filled, round), fill.color);
    }
}
