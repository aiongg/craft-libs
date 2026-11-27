//! `Scroll`: a clipped viewport over tall content, a wheel taker, and the floating scroll bar
//! (look sheet 3.13, 5.6). The offset is a cell the owner shares; scrolling moves the content
//! node and repaints the bar only.

use std::hash::Hash;

use craft_ui_rt::{BuildCx, Constraints, Element, Event, EventCx, Gesture, Handled, LayoutCx, PaintCx, Rect, Sense, State, Vec2, WheelUnit, hash_key};

use crate::look::Look;

/// What the viewport shows of what: both lengths along the scroll axis, in points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Extent {
    pub content: f32,
    pub view: f32,
}

impl Extent {
    pub fn max_offset(&self) -> f32 {
        (self.content - self.view).max(0.0)
    }
}

/// A vertical viewport. `f` creates the one content child, `content` points tall, and is given
/// the cell holding what the viewport shows (the content's virtual rows need the view's height);
/// the bar is the second child.
pub struct Scroll<P, F> {
    offset: State<f32>,
    content: f32,
    props: P,
    f: F,
}

impl<P: Hash + 'static, F: Fn(&P, State<Extent>, &mut BuildCx<'_>) + 'static> Scroll<P, F> {
    pub fn new(offset: State<f32>, content: f32, props: P, f: F) -> Self {
        Scroll { offset, content, props, f }
    }
}

impl<P: Hash + 'static, F: Fn(&P, State<Extent>, &mut BuildCx<'_>) + 'static> Element for Scroll<P, F> {
    type Output = ();
    type Local = State<Extent>;

    fn props_key(&self) -> u64 {
        hash_key(&(hash_key(&self.props), self.content.to_bits()))
    }

    fn init(&self, cx: &mut BuildCx<'_>) -> State<Extent> {
        cx.state(Extent::default())
    }

    fn build(&self, extent: &mut State<Extent>, cx: &mut BuildCx<'_>) {
        (self.f)(&self.props, *extent, cx);
        cx.child(EdgeFades { offset: self.offset, extent: *extent });
        cx.child(ScrollBar { offset: self.offset, extent: *extent });
    }

    fn layout(&self, extent: &mut State<Extent>, cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let size = Vec2::new(if c.max.x.is_finite() { c.max.x } else { c.min.x }, if c.max.y.is_finite() { c.max.y } else { c.min.y });
        let now = Extent { content: self.content, view: size.y };
        extent.set_if_changed(cx, now);
        let offset = self.offset.get(cx);
        let clamped = offset.clamp(0.0, now.max_offset());
        if clamped != offset {
            self.offset.set(cx, clamped);
        }
        let tokens = cx.tokens();
        let bar = Look::of(&tokens).bar;
        let content = cx.child_at(0);
        cx.layout_child(content, Constraints::tight(Vec2::new(size.x, self.content)));
        cx.place_child(content, Vec2::new(0.0, -clamped));
        let fades = cx.child_at(1);
        cx.layout_child(fades, Constraints::tight(size));
        cx.place_child(fades, Vec2::ZERO);
        let track = cx.child_at(2);
        cx.layout_child(track, Constraints::tight(Vec2::new(bar, size.y)));
        cx.place_child(track, Vec2::new(size.x - bar, 0.0));
        size
    }

    fn relayout_boundary(&self) -> bool {
        true
    }

    fn paint(&self, _: &State<Extent>, cx: &mut PaintCx<'_>) {
        cx.clip(cx.bounds());
        cx.hit(cx.bounds(), Sense::WHEEL, 0);
    }

    fn event(&self, extent: &mut State<Extent>, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        let Event::Wheel(w) = ev else { return Handled::No };
        let e = extent.get(cx);
        let max = e.max_offset();
        let points = match w.unit {
            WheelUnit::Line => w.delta.y * cx.runtime().wheel_options().line_points,
            WheelUnit::Page => w.delta.y * e.view,
            _ => w.delta.y,
        };
        let offset = self.offset.get(cx);
        self.offset.set_if_changed(cx, (offset - points).clamp(0.0, max));
        Handled::Yes
    }
}

fn look_of(cx: &mut EventCx<'_>) -> Look {
    let tokens = cx.env().tokens;
    Look::of(&tokens.get(cx))
}

/// The fades over the view's top and bottom edges for the content hidden past them (look sheet
/// 3.13): `FADE_LENGTH` long, from the window background colour at `FADE_STRENGTH` times the hidden share of
/// that length to clear. Between the content and the bar, and sensing nothing.
struct EdgeFades {
    offset: State<f32>,
    extent: State<Extent>,
}

const FADE_LENGTH: f32 = 20.0;
const FADE_STRENGTH: f32 = 0.5;

impl Element for EdgeFades {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let ground = Look::of(&cx.tokens()).background;
        let e = self.extent.get(cx);
        let offset = self.offset.get(cx);
        let view = cx.bounds();
        let length = FADE_LENGTH.min(view.height());
        let strength = |hidden: f32| ground.faded((hidden / FADE_LENGTH).clamp(0.0, 1.0) as f32 * FADE_STRENGTH);
        let clear = ground.with_alpha(0);
        let above = offset.max(0.0);
        let below = (e.content - e.view - offset).max(0.0);
        if above > 0.0 && length > 0.0 {
            let rect = crate::geo::rect(view.min.x, view.min.y, view.max.x, view.min.y + length);
            cx.vgradient(rect, strength(above), clear);
        }
        if below > 0.0 && length > 0.0 {
            let rect = crate::geo::rect(view.min.x, view.max.y - length, view.max.x, view.max.y);
            cx.vgradient(rect, clear, strength(below));
        }
    }
}

/// The track and its thumb, over the content's right edge.
struct ScrollBar {
    offset: State<f32>,
    extent: State<Extent>,
}

/// A thumb drag: where the pointer and the offset were when it began.
#[derive(Clone, Copy)]
struct ThumbDrag {
    pointer_y: f32,
    offset: f32,
}

impl ScrollBar {
    /// The thumb's rect in the track, `None` while everything fits.
    fn thumb(look: &Look, e: Extent, offset: f32, width: f32) -> Option<Rect> {
        if e.content <= e.view || e.view <= 0.0 {
            return None;
        }
        // craft's thumb rule, in its f32 track units.
        let span = craft_behavior::scroll::ScrollSpan::new(0.0..f64::from(e.content), f64::from(offset)..f64::from(offset + e.view));
        let (y, len) = craft_behavior::scroll::thumb(&span, e.view, look.bar_min_thumb);
        Some(crate::geo::rect(0.0, y, width, y + len))
    }
}

impl Element for ScrollBar {
    type Output = ();
    type Local = Option<ThumbDrag>;

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) -> Option<ThumbDrag> {
        None
    }

    fn paint(&self, _: &Option<ThumbDrag>, cx: &mut PaintCx<'_>) {
        let look = Look::of(&cx.tokens());
        let e = self.extent.get(cx);
        let offset = self.offset.get(cx);
        let bounds = cx.bounds();
        let Some(thumb) = Self::thumb(&look, e, offset, bounds.width()) else { return };
        cx.hit(bounds, Sense::HOVER | Sense::CLICK | Sense::DRAG, 0);
        let state = cx.interaction();
        let active = state.hovered || state.pressed;
        if active {
            cx.rect(bounds, bounds.width() / 2.0, look.hover, None);
        }
        let (rect, ink) = if state.pressed {
            (thumb, look.text)
        } else if state.hovered {
            (thumb, look.text_muted)
        } else {
            (crate::geo::rect(thumb.max.x - look.thick, thumb.min.y, thumb.max.x, thumb.max.y), look.text_faint)
        };
        cx.rect(rect, rect.width().min(rect.height()) / 2.0, ink, None);
    }

    fn event(&self, drag: &mut Option<ThumbDrag>, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        let Event::Pointer(p) = ev else { return Handled::No };
        let at = cx.to_local(p.pos);
        let e = self.extent.get(cx);
        let offset = self.offset.get(cx);
        match p.gesture {
            Gesture::Down { .. } => {
                let width = cx.bounds().width();
                let look = look_of(cx);
                match Self::thumb(&look, e, offset, width) {
                    Some(thumb) if thumb.contains(at) => {
                        *drag = Some(ThumbDrag { pointer_y: at.y, offset });
                    }
                    Some(thumb) => {
                        let to = if at.y < thumb.min.y { offset - e.view } else { offset + e.view };
                        self.offset.set_if_changed(cx, to.clamp(0.0, e.max_offset()));
                    }
                    None => {}
                }
            }
            Gesture::DragStart { .. } | Gesture::Drag { .. } => {
                if let Some(d) = *drag {
                    let width = cx.bounds().width();
                    let look = look_of(cx);
                    if let Some(thumb) = Self::thumb(&look, e, offset, width) {
                        let travel = (e.view - thumb.height()).max(1.0);
                        let to = d.offset + (at.y - d.pointer_y) * e.max_offset() / travel;
                        self.offset.set_if_changed(cx, to.clamp(0.0, e.max_offset()));
                    }
                }
            }
            Gesture::Up { .. } | Gesture::Cancel(_) => *drag = None,
            Gesture::LongPress => {}
        }
        Handled::Yes
    }
}
