//! `RangeSlider`: a lower and an upper value on one track, with a thumb for each. The thumbs never cross. A press on a
//! thumb drags it; a press between them drags both and keeps the distance; a press elsewhere on the track moves the
//! nearer thumb there. Where the thumbs meet, the direction of the drag picks the one that moves.

use super::slider::{LENGTH_IN_ICONS, Scale, SliderScale, THUMB_INSET, thumb_rect};
use super::track::{self, Along, Fill, MarkInk, Marks, TrackFace, TrackLook};
use super::words::ValueDragWords;
use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, Input, MeasureCx, Name, Numeric, PaintUi, PartsCx, Pointer as _,
    Remember as _, Role, Styled as _, WordSetCx as _,
};
use crate::primitives::{PaintShape, StrokeWidth};
use crate::runtime::env::Label;
use crate::runtime::focus::{ActivateKeys, FocusShape, KeepKeys, KeyUse};
use crate::style::{ComponentStyle, FocusLook, StyleColor, StyleCx, TextLook};
use craft_behavior::{HitLevel, HitPriority, Metrics, PartKey, PressPhase, Sense, ValueDrag};
use craft_theme::Stateful;
use egui::{CornerRadius, Rect, Vec2};
use std::cmp::Ordering;
use std::ops::RangeInclusive;

#[cfg(feature = "testing")]
mod stories;

/// The track both thumbs ride on.
pub(crate) const TRACK: PartKey = PartKey::new("track");
/// The lower thumb.
pub(crate) const LOWER: PartKey = PartKey::new("lower");
/// The upper thumb.
pub(crate) const UPPER: PartKey = PartKey::new("upper");

/// The width of a drawn thumb as a share of its height.
const THUMB_ASPECT: f32 = 0.4;
/// The corner of a drawn thumb, in points.
const THUMB_CORNER: u8 = 1;

/// The style of a range slider.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct RangeSliderStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The labels under labelled marks.
    #[style(text)]
    mark_text: TextLook,
    /// The track and the part of it between the thumbs.
    track: TrackLook,
    /// The fill of a thumb.
    thumb: Stateful<StyleColor>,
    /// The edge of a thumb.
    thumb_edge: Stateful<StyleColor>,
    /// The width of a drawn thumb.
    thumb_width: f32,
    /// The height of a drawn thumb.
    thumb_height: f32,
}

/// What a press moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Moving {
    #[default]
    Lower,
    Upper,
    /// Both thumbs by the same distance: a press between them.
    Span,
    /// Thumbs that meet under the press: the drag's direction picks one.
    Either,
}

/// Where a press can begin.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Lower,
    Upper,
    Track,
}

impl Source {
    fn key(self) -> PartKey {
        match self {
            Source::Lower => LOWER,
            Source::Upper => UPPER,
            Source::Track => TRACK,
        }
    }
}

/// The running drag: the value when it began and what it moves; and the thumb moved last, which the wheel moves when
/// no thumb has focus.
#[derive(Clone, Default)]
struct Drag {
    from: (f64, f64),
    moving: Moving,
    upper_last: bool,
}

/// The thumbs' values in order and inside the range, `None` for a value with no pair to show.
fn shown(scale: &Scale, value: Option<(f64, f64)>) -> Option<(f64, f64)> {
    value.map(|(a, b)| {
        let (a, b) = (scale.place(a), scale.place(b));
        if a <= b { (a, b) } else { (b, a) }
    })
}

/// What a press from `source` at `offset` from the start edge moves, with the thumbs' centres at `centres`.
fn pressed(source: Source, (lower, upper): (f64, f64), offset: f32, (low, high): (f32, f32)) -> Moving {
    let together = lower == upper;
    match source {
        Source::Lower | Source::Upper if together => Moving::Either,
        Source::Lower => Moving::Lower,
        Source::Upper => Moving::Upper,
        Source::Track if together => match offset.partial_cmp(&low) {
            Some(Ordering::Less) => Moving::Lower,
            Some(Ordering::Greater) => Moving::Upper,
            _ => Moving::Either,
        },
        Source::Track if offset <= low => Moving::Lower,
        Source::Track if offset >= high => Moving::Upper,
        Source::Track => Moving::Span,
    }
}

/// The thumbs after a drag that began at `from`. For a thumb, `input` is the pointer's position along the track; for
/// the span it is the distance the pointer travelled, in track lengths. Thumbs stop at each other, and a thumb near a
/// mark snaps to it.
fn moved(scale: &Scale, snap: &dyn Fn(f64) -> Option<f64>, moving: Moving, from: (f64, f64), input: f64) -> (f64, f64) {
    let at = |position: f64| scale.snap(scale.at_position(position));
    let thumb = |position: f64| snap(position).unwrap_or_else(|| at(position));
    match moving {
        Moving::Lower => (thumb(input).min(from.1), from.1),
        Moving::Upper => (from.0, thumb(input).max(from.0)),
        Moving::Either if input < scale.position(from.0) => moved(scale, snap, Moving::Lower, from, input),
        Moving::Either => moved(scale, snap, Moving::Upper, from, input),
        Moving::Span => {
            let (a, b) = (scale.position(from.0), scale.position(from.1));
            let low = (a + input).clamp(0.0, 1.0 - (b - a));
            (at(low), at(low + (b - a)))
        }
    }
}

/// `pair` with the thumb `upper` set to `value`.
fn with(pair: (f64, f64), upper: bool, value: f64) -> (f64, f64) {
    if upper { (pair.0, value) } else { (value, pair.1) }
}

/// A range slider over `range`.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct RangeSlider<'a> {
    #[common]
    common: Common,
    value: Bind<'a, (f64, f64)>,
    range: RangeInclusive<f64>,
    /// The change of one arrow press, and the stops the values snap to; the default is a hundredth of the range and
    /// no stops.
    #[prop(quantity = "step")]
    step: Option<f64>,
    /// How positions along the track map to values.
    #[prop]
    scale: SliderScale,
    /// Whether the part of the track between the thumbs is filled.
    #[prop]
    fill: bool,
    /// Evenly spaced ticks under the track: one more than this, none for 0.
    #[prop]
    ticks: usize,
    marks: Marks,
    track_face: Option<TrackFace<'a>>,
}

impl<'a> RangeSlider<'a> {
    /// A range slider of `value`, lower end first, over `range`.
    pub fn new(value: Bind<'a, (f64, f64)>, range: RangeInclusive<f64>) -> Self {
        RangeSlider {
            common: Common::new(),
            value,
            range,
            step: None,
            scale: SliderScale::Linear,
            fill: true,
            ticks: 0,
            marks: Marks::default(),
            track_face: None,
        }
    }

    /// Values a thumb snaps to when a drag brings it near, drawn as marks under the track; see [`Slider::marks`].
    ///
    /// [`Slider::marks`]: super::Slider::marks
    pub fn marks(mut self, marks: &[f64]) -> Self {
        self.marks.set(marks);
        self
    }

    /// [`RangeSlider::marks`] at the values, each with a caption centred under its mark. The slider is taller by one
    /// caption line.
    pub fn marks_labelled<L: Into<Label>>(mut self, marks: impl IntoIterator<Item = (f64, L)>) -> Self {
        self.marks.set_labelled(marks);
        self
    }

    /// Paints the groove with `face`, a component shown as the look of the track. The fill is not drawn over it.
    pub fn track_face(mut self, face: TrackFace<'a>) -> Self {
        self.track_face = Some(face);
        self
    }

    fn track_scale(&self) -> Scale {
        Scale::new(&self.range, self.step, self.scale)
    }

    /// The values the next interaction starts from: the shown pair, or both thumbs at the start of the range.
    fn current(scale: &Scale, cx: &Cx<'_, Self>) -> (f64, f64) {
        shown(scale, cx.value().copied()).unwrap_or((scale.min, scale.min))
    }

    /// The range thumb `upper` moves in: it stops at the other thumb.
    fn bounds(scale: &Scale, pair: (f64, f64), upper: bool) -> (f64, f64) {
        if upper { (pair.0, scale.max) } else { (scale.min, pair.1) }
    }
}

impl Component for RangeSlider<'_> {
    const ROLE: Role = Role::GROUP;
    const OVERLAYS: bool = false;
    const KEYS: KeyUse = KeyUse::of(Input::Pointer).with_keep(KeepKeys::BlockArrows).with_activate(ActivateKeys::Neither);
    type Style = RangeSliderStyle;
    type Output = EditResponse<(f64, f64)>;

    fn style(cx: &StyleCx<'_>, _: RangeSliderStyleKey) -> RangeSliderStyle {
        let control = cx.control();
        let colors = &cx.tokens.color;
        let accent = &colors.accent;
        let height = control.icon - THUMB_INSET * 2.0;
        RangeSliderStyle {
            metrics: Metrics::new().min_size(Vec2::new(control.icon * LENGTH_IN_ICONS, control.height)),
            focus: crate::controls::focus_look(cx),
            mark_text: track::mark_text(cx),
            track: TrackLook::new(cx),
            thumb: track::states(cx, colors.surface.knob, colors.surface.knob, colors.surface.knob),
            thumb_edge: track::states(cx, colors.line.knob_edge, accent.strong_hover, accent.strong_pressed),
            thumb_width: height * THUMB_ASPECT,
            thumb_height: height,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        self.marks.lay_out(&cx.style.mark_text, |label| cx.env.resolve(label), |run| cx.layout(run).size());
        Vec2::new(cx.icon() * LENGTH_IN_ICONS, cx.icon() + self.marks.below(&cx.style.track))
    }

    fn sense(&self) -> Sense {
        Sense::WHEEL | Sense::WHEEL_X
    }

    /// The track, and a thumb each that is its own Tab stop and takes the arrow keys.
    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        self.marks.lay_out(&cx.style.mark_text, |label| cx.env.resolve(label), |run| cx.layout(run).size());
        let scale = self.track_scale();
        let area = self.marks.area(rect, &cx.style.track);
        let (lower, upper) = shown(&scale, cx.value().copied()).unwrap_or((scale.min, scale.min));
        if self.track_face.is_some() {
            cx.part(track::GROOVE, cx.style.track.groove(area));
        }
        cx.part(TRACK, area).sense(Sense::CLICK | Sense::DRAG).value_part().pointer_only("the arrow keys step the focused thumb");
        for (key, value) in [(LOWER, lower), (UPPER, upper)] {
            cx.part(key, thumb_rect(&scale, value, area, cx.flow))
                .sense(Sense::DRAG | Sense::TAKE_FOCUS)
                .priority(HitPriority::new(HitLevel::Handle, 0))
                .focus_shape(FocusShape::Rect)
                .value_part()
                .focusable();
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, (f64, f64)>> {
        Some(self.value.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if cx.resolved.read_only || !cx.resolved.enabled {
            return;
        }
        let scale = self.track_scale();
        let area = self.marks.area(cx.rect, &cx.style.track);
        let (rtl, half) = (cx.flow.is_rtl(), area.height() / 2.0);
        let length = (area.width() - area.height()).max(1.0);
        let (marks, look) = (&self.marks, cx.style.track.clone());
        let snap = |position: f64| marks.snapped(&scale, &look, length, position);
        // Thumbs follow the pointer's position along the track, in positions 0 to 1; the span follows its travel.
        let drag = ValueDrag::absolute((area.left() + half)..(area.right() - half), 0.0..=1.0);
        let travel = ValueDrag::relative(1.0 / f64::from(length), -1.0..=1.0);
        let start_edge = cx.flow.start_x();
        let offset = |x: f32| (x - start_edge).abs();
        let centre = |value: f64| half + scale.position(value) as f32 * length;
        for source in [Source::Lower, Source::Upper, Source::Track] {
            let press = cx.part(source.key()).press;
            let running = cx.memory::<Drag>().clone();
            let input = |moving: Moving, from: (f64, f64)| match moving {
                Moving::Span => travel.value(0.0, &press, rtl),
                Moving::Upper => drag.value(scale.position(from.1), &press, rtl),
                Moving::Lower | Moving::Either => drag.value(scale.position(from.0), &press, rtl),
            };
            match press.phase {
                PressPhase::DragStart { start, .. } => {
                    let from = Self::current(&scale, cx);
                    let moving = pressed(source, from, offset(start.origin.x), (centre(from.0), centre(from.1)));
                    let last = cx.memory::<Drag>();
                    last.from = from;
                    last.moving = moving;
                    match moving {
                        Moving::Lower => last.upper_last = false,
                        Moving::Upper => last.upper_last = true,
                        Moving::Span | Moving::Either => {}
                    }
                    cx.begin();
                }
                PressPhase::Drag { .. } => {
                    if let Some(next) = input(running.moving, running.from).map(|input| moved(&scale, &snap, running.moving, running.from, input)) {
                        cx.propose(next);
                    }
                }
                PressPhase::Release { start, click, .. } => {
                    if cx.gesture().is_some() {
                        if let Some(next) = input(running.moving, running.from).map(|input| moved(&scale, &snap, running.moving, running.from, input)) {
                            cx.propose(next);
                        }
                        cx.commit();
                    } else if click {
                        let from = Self::current(&scale, cx);
                        let moving = pressed(source, from, offset(start.origin.x), (centre(from.0), centre(from.1)));
                        if moving != Moving::Span
                            && let Some(next) = input(moving, from).map(|input| moved(&scale, &snap, moving, from, input))
                        {
                            cx.set(next);
                        }
                    }
                }
                _ => {}
            }
        }
        let focused = cx.active_part().filter(|part| part.key == LOWER || part.key == UPPER).map(|part| part.key == UPPER);
        while let Some((mv, mods)) = cx.intents().take_move() {
            if let Some(upper) = focused {
                let pair = Self::current(&scale, cx);
                let value = if upper { pair.1 } else { pair.0 };
                if let Some(next) = scale.stepped(value, Self::bounds(&scale, pair, upper), mv, mods) {
                    cx.set(with(pair, upper, next));
                }
            }
        }
        let upper = focused.unwrap_or(cx.memory::<Drag>().upper_last);
        if let Some(wheel) = cx.take_wheel() {
            let along = if wheel.scroll.y != 0.0 {
                wheel.scroll.y
            } else if rtl {
                -wheel.scroll.x
            } else {
                wheel.scroll.x
            };
            let pair = Self::current(&scale, cx);
            let value = if upper { pair.1 } else { pair.0 };
            cx.set(with(pair, upper, scale.nudged(value, f64::from(along.signum()), Self::bounds(&scale, pair, upper))));
        }
        if let Some(next) = cx.intents().take_set_value() {
            let pair = Self::current(&scale, cx);
            let (low, high) = Self::bounds(&scale, pair, upper);
            cx.set(with(pair, upper, scale.snap(next).clamp(low, high)));
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let scale = self.track_scale();
        let words = cx.words::<ValueDragWords>();
        let pair = shown(&scale, cx.value().copied());
        let label = cx.resolved.label;
        for (key, word, upper) in [(LOWER, &words.lower, false), (UPPER, &words.upper, true)] {
            let node = access.node(key, Role::SLIDER);
            node.name = if label.is_empty() { Name::Text(word.as_str().to_owned()) } else { Name::Joined(vec![label.to_owned(), word.as_str().to_owned()]) };
            node.numeric = Some(match pair {
                Some(pair) => {
                    let (low, high) = Self::bounds(&scale, pair, upper);
                    Numeric::new(if upper { pair.1 } else { pair.0 }, low..=high)
                }
                None => Numeric::empty(scale.min..=scale.max),
            }
            .step(scale.small()));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let style = cx.style;
        let flow = cx.flow;
        let scale = self.track_scale();
        let area = self.marks.area(cx.rect, &style.track);
        let pair = shown(&scale, cx.value().copied());
        let (lower, upper) = pair.unwrap_or((scale.min, scale.min));
        let faced = match &self.track_face {
            Some(face) => {
                face.paint(ui);
                true
            }
            None => false,
        };
        let draw = ui.draw();
        let thumbs = [(LOWER, lower), (UPPER, upper)].map(|(key, value)| (key, thumb_rect(&scale, value, area, flow)));
        if !faced {
            let reach = |thumb: Rect| (thumb.center().x - flow.start_x()).abs();
            let fill = pair.filter(|_| self.fill).map(|_| Fill { color: cx.resolve(&style.track.filled), from: reach(thumbs[0].1), to: reach(thumbs[1].1) });
            track::paint(&draw, &style.track, area, flow, cx.resolve(&style.track.groove), fill);
        }
        let ink = MarkInk {
            marks: cx.resolve(&style.track.mark_color),
            labels: cx.resolve(&style.mark_text.color),
            role: style.mark_text.role,
        };
        track::paint_marks(&draw, &style.track, &self.marks, &scale, Along { area, flow }, cx.rect, &ink);
        for (key, thumb) in thumbs {
            let drawn = Rect::from_center_size(thumb.center(), Vec2::new(style.thumb_width, style.thumb_height));
            let shape = PaintShape::Rect(drawn, CornerRadius::same(THUMB_CORNER));
            draw.fill(&shape, if pair.is_some() { cx.resolve_part(key, &style.thumb) } else { cx.resolve(&style.track.groove) });
            draw.stroke(&shape, StrokeWidth::Thin, cx.resolve_part(key, &style.thumb_edge));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<(f64, f64)> {
        cx.edit_response()
    }
}
