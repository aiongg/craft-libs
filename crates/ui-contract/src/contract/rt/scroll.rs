//! The scroll body (docs/contract-rt.md section 4): the node that holds a scrolling container's
//! body. One content node under it holds the body's children, laid out in content coordinates by the
//! build; the scroll offset lives on the scroll node, and its layout moves the content node by it. A wheel step, a bar drag or a glide changes the offset
//! and lays out this one node: the children keep their display lists and the renderer translates
//! them, so nothing runs. Only when the view leaves the band the last build covered (content shown
//! through `Lazy` children stayed out of view) does the body build again, through the path to its
//! window root.
//!
//! - Bars: a thumb per scrolling axis over the content, from the offset and the extent; a press on
//!   it drags the offset.
//! - Fades: an edge fade over the content at each end the view does not reach.
//! - Reveal: a `Lazy` that asks to be revealed sets a glide to its start; any wheel or drag ends it.
//! - Glide at a rate: [`glide`] sets a velocity (a drag's edge scroll, middle-button auto-scroll); layout
//!   pays it out frame by frame and nothing runs until the view leaves the built band or the rate changes.
//! - Anchors: `ScrollAnchor::End` follows the end while the view is at it; otherwise the first child
//!   in view keeps its place when content before it changes extent.
//! - Kinetic: line steps (mouse notches) gather in a `WheelEase` that layout pays out frame by
//!   frame; point steps (touchpads, which bring their own momentum) apply at once.
//! - Indicators: `ScrollIndicator::Bar` draws the bars always, `Overlay` only while the pointer is
//!   over the view (or always with `at_rest`), `Strips` a strip at each end the view does not reach,
//!   which scrolls while pressed. A bar takes the floating scroll bar's look: a thin thumb at rest, the full
//!   thickness and a stronger fill while the pointer is on the thumb or the thumb is dragged.
//! - ScrollIntoView: an assistive technology's request on a node inside the body bubbles to the
//!   nearest scroll body, which glides until the node is in view, and goes on to the bodies around
//!   it.

use craft_behavior::scroll::{drag, thumb};
use craft_behavior::{ScrollSpan, WheelEase};
use craft_theme::{State as LookState, States, Tokens};
use craft_ui_rt::{
    AccessCx, BuildCx, Constraints, Element, Event, EventCx, Gesture, Handled, HitShape, InteractFact, LayoutCx, NodeId, PaintCx, Pos2, Rect, Rgba,
    Sense, State, Vec2, WheelUnit,
};

use super::ids::node_of;
use super::node::{Kept, NodeInput, Placed};
use super::ui::{Flow, Ui};
use crate::contract::{Id, Notch, Scroll, ScrollAnchor};
use crate::style::{StyleColor, scroll_bar_look};

/// The share of the view's extent built beyond each of its edges: a step within it moves the kept
/// content and builds nothing.
const OVERSCAN_SHARE: f32 = 0.5;

/// The part a bar thumb's hit shape names, per axis.
const BAR_PARTS: [u64; 2] = [1, 2];

/// The parts of the strips before and after the view (`ScrollIndicator::Strips`).
const STRIP_PARTS: [u64; 2] = [3, 4];

/// Rows per second a body scrolls at while one of its strips is pressed, in rows a control high.
const STRIP_ROWS_PER_SECOND: f32 = 24.0;

/// A glide's step per frame: the share of the way left it covers.
const GLIDE_SHARE: f32 = 0.35;

/// A glide this close to its end, in points, ends there.
const GLIDE_SETTLE: f32 = 0.5;

/// The frame interval a glide or eased step asks layout for, in seconds.
const FRAME: f64 = 1.0 / 60.0;

/// The smallest move of `view` that shows the latest reveal queued on the run input of a container that owns its
/// body's scroll without a scroll region (`NodeInput::Reveal`, `Focus::take_reveal`): positive toward larger
/// coordinates. `None` when none is queued or the view shows it already. A reveal larger than the view brings its start
/// into view.
pub(crate) fn reveal_move(input: &[NodeInput], view: Rect) -> Option<Vec2> {
    let rect = input.iter().rev().find_map(|input| match input {
        NodeInput::Reveal(rect) => Some(*rect),
        _ => None,
    })?;
    let axis = |low: f32, high: f32, view_low: f32, view_high: f32| {
        if low < view_low {
            low - view_low
        } else if high > view_high {
            (high - view_high).min(low - view_low)
        } else {
            0.0
        }
    };
    let by = Vec2::new(axis(rect.min.x, rect.max.x, view.min.x, view.max.x), axis(rect.min.y, rect.max.y, view.min.y, view.max.y));
    (by != Vec2::ZERO).then_some(by)
}

/// The rect content is built for: the view and `OVERSCAN_SHARE` of it beyond each scrolling edge.
pub(crate) fn overscan(view: Rect) -> Rect {
    view.expand2(view.size() * OVERSCAN_SHARE)
}

/// A scroll body's element; its `scroll` props are the container's.
pub(crate) struct ScrollNode {
    pub(crate) scroll: Scroll,
}

/// What a scroll body keeps: its children's placement (content coordinates) and size (the view),
/// and the scroll state the node owns.
pub(crate) struct ScrollKept {
    pub(crate) kept: Kept,
    pub(crate) offset: Vec2,
    extent: Vec2,
    view: Vec2,
    /// The content rect the last build covered; everything when no `Lazy` stayed out of view.
    band: Rect,
    /// Written by layout when the view left the band; the build reads it, so it runs again.
    misses: Option<State<u64>>,
    /// The offset layout placed the content at, for a build that follows it (`read_offset`); written by layout only.
    placed_at: Option<State<Vec2>>,
    ease: WheelEase,
    glide: Option<Vec2>,
    /// The velocity [`glide`] sets, in points per second, positive toward the content's end. Only this node's layout
    /// reads it, so a write lays this node out and runs nothing.
    rate: State<Vec2>,
    /// When layout last paid the rate out; `None` while it is zero or the offset stopped at an end.
    rate_at: Option<f64>,
    /// A bar drag: the axis, the pointer position along it at the press and the offset then.
    grab: Option<(usize, f32, f32)>,
    /// The view is at the content's end (`ScrollAnchor::End` follows it).
    at_end: bool,
    /// The first child in view and its content position, kept in place across builds.
    anchor: Option<(NodeId, f32)>,
    /// A pressed strip: the direction it scrolls (-1 back, 1 forward) and when layout last paid it out.
    strip: Option<(f32, f64)>,
    /// `jump_to` moved the offset since the last `finish_build`.
    jumped: bool,
    look: Look,
}

/// The bar and fade look, resolved from the tokens at the last build. The bar takes the floating scroll bar's look
/// (`scroll_bar_look`), the one every scroll bar shares.
#[derive(Clone, Copy)]
struct Look {
    /// The bar's thickness: its track, and its thumb at its widest.
    thickness: f32,
    /// The thumb's thickness per state: thin at rest along the track's outer edge, the bar's thickness under the
    /// pointer or while dragged.
    thumb_width: States<f32>,
    min_thumb: f32,
    /// The thumb's fill per state.
    thumb: States<Rgba>,
    /// The track's fill per state.
    track: States<Rgba>,
    fade: Rgba,
    fade_extent: f32,
    line: f32,
    /// The height of a scroll strip.
    strip: f32,
}

impl Look {
    fn of(tokens: &Tokens) -> Look {
        let bar = scroll_bar_look(tokens);
        let fill = |states: States<StyleColor>| {
            let rgba = |color: StyleColor| crate::primitives::items::rgba(color.egui());
            States::new(rgba(states.rest), rgba(states.hover), rgba(states.pressed), rgba(states.disabled))
        };
        Look {
            thickness: bar.thickness,
            thumb_width: bar.thumb_width.off,
            min_thumb: bar.min_thumb,
            thumb: fill(bar.thumb.off),
            track: fill(bar.track.off),
            fade: tokens.color.surface.chrome,
            fade_extent: tokens.space.gap.section,
            line: tokens.size.button,
            strip: tokens.size.control.sm,
        }
    }

    /// The look before the first build.
    const NONE: Look = {
        let clear = States::new(Rgba([0; 4]), Rgba([0; 4]), Rgba([0; 4]), Rgba([0; 4]));
        Look {
            thickness: 0.0,
            thumb_width: States::new(0.0, 0.0, 0.0, 0.0),
            min_thumb: 0.0,
            thumb: clear,
            track: clear,
            fade: Rgba([0; 4]),
            fade_extent: 0.0,
            line: 0.0,
            strip: 0.0,
        }
    };
}

impl ScrollKept {
    fn new(rate: State<Vec2>) -> ScrollKept {
        ScrollKept {
            kept: Kept::new(),
            offset: Vec2::ZERO,
            extent: Vec2::ZERO,
            view: Vec2::ZERO,
            band: Rect::EVERYTHING,
            misses: None,
            placed_at: None,
            ease: WheelEase::default(),
            glide: None,
            rate,
            rate_at: None,
            jumped: false,
            grab: None,
            at_end: true,
            anchor: None,
            strip: None,
            look: Look::NONE,
        }
    }

    /// The build reads the band-miss cell, so a layout that leaves the band builds the body again.
    pub(crate) fn subscribe(&mut self, cx: &mut BuildCx<'_>) {
        let misses = *self.misses.get_or_insert_with(|| cx.state(0));
        misses.get(cx);
    }

    /// The cell layout writes the offset it placed the content at into. A node that reads it builds again each time
    /// layout moves the offset (a glide, a wheel, a bar drag); the owner whose run follows the offset reads it.
    pub(crate) fn offset_cell(&mut self, cx: &mut BuildCx<'_>) -> State<Vec2> {
        let offset = self.offset;
        *self.placed_at.get_or_insert_with(|| cx.state(offset))
    }

    fn max_offset(&self) -> Vec2 {
        (self.extent - self.view).max(Vec2::ZERO)
    }

    fn clamp(&self, offset: Vec2) -> Vec2 {
        offset.clamp(Vec2::ZERO, self.max_offset())
    }

    /// The view at the current offset leaves the band the last build covered.
    fn leaves_band(&self) -> bool {
        !self.band.contains_rect(Rect::from_min_size(self.offset.to_pos2(), self.view))
    }

    /// Takes over what the body's build found: the content's extent and placement, the band it
    /// covered, a reveal; keeps the anchors and clamps the offset.
    /// `content` is the node holding the body's children, which the offset moves as one transform.
    pub(crate) fn finish_build(&mut self, cx: &mut BuildCx<'_>, view: Rect, content: NodeId, flow: &mut Flow, scroll: Scroll, tokens: &Tokens) {
        let extent = flow.content_size();
        let content_origin = flow.origin;
        let placed = &flow.placed;
        let before = (self.offset, self.extent, self.view);
        self.look = Look::of(tokens);
        self.view = view.size();
        // The first child in view keeps its place when content before it changed extent.
        if let Some((node, y)) = self.anchor
            && let Some(p) = placed.iter().find(|p| p.node == node)
            && self.glide.is_none()
        {
            self.offset.y += p.offset.y - y;
        }
        self.extent = extent;
        if scroll.anchor == ScrollAnchor::End && self.at_end {
            self.offset = self.max_offset();
        }
        if let Some(at) = flow.reveal.take() {
            let target = self.clamp(at - content_origin);
            self.glide = Some(target);
            // The reveal goes on to the scroll bodies around this one: where the point lands in this view once the
            // glide ends (`flow` is absorbed into the flow around).
            flow.reveal = Some(view.min + (at - content_origin - target));
        }
        self.offset = self.clamp(self.offset);
        self.at_end = self.offset.y >= self.max_offset().y;
        self.anchor = placed.iter().find(|p| p.offset.y + p.size.y > self.offset.y).map(|p| (p.node, p.offset.y));
        self.band = if flow.limited { overscan(view).translate(-content_origin.to_vec2()) } else { Rect::EVERYTHING };
        let placed = vec![Placed { node: content, offset: Vec2::ZERO, size: extent }];
        let jumped = std::mem::take(&mut self.jumped);
        if self.kept.placed != placed || before != (self.offset, self.extent, self.view) || self.glide.is_some() || jumped {
            self.kept.placed = placed;
            self.kept.size = self.view;
            cx.mark_changed();
        }
    }

    /// Sets the offset to `offset` at once (clamped once the build knows the extent): a bound offset that its owner
    /// moved. It ends a glide, an eased step and a bar drag, and drops the anchor, so the first child in view does not
    /// pull the offset back. The build that calls it lays the content out again when it finishes (`finish_build`).
    pub(crate) fn jump_to(&mut self, offset: Vec2) {
        if offset == self.offset && self.glide.is_none() {
            return;
        }
        self.offset = offset.max(Vec2::ZERO);
        self.glide = None;
        self.ease = WheelEase::default();
        self.grab = None;
        self.anchor = None;
        self.jumped = true;
    }

    /// Moves the offset to `to` (clamped); returns whether it moved.
    fn move_to(&mut self, to: Vec2) -> bool {
        let to = self.clamp(to);
        if to == self.offset {
            return false;
        }
        self.offset = to;
        self.at_end = self.offset.y >= self.max_offset().y;
        true
    }

    /// The bar of `axis` in local coordinates and its thumb, when the content overflows there.
    fn bar(&self, axis: usize, bounds: Rect) -> Option<(Rect, Rect)> {
        let (extent, view, offset) = (self.extent[axis], self.view[axis], self.offset[axis]);
        if extent <= view {
            return None;
        }
        let t = self.look.thickness;
        let track = if axis == 1 {
            Rect::from_min_max(Pos2::new(bounds.max.x - t, bounds.min.y), bounds.max)
        } else {
            Rect::from_min_max(Pos2::new(bounds.min.x, bounds.max.y - t), bounds.max)
        };
        let span = ScrollSpan::new(0.0..f64::from(extent), f64::from(offset)..f64::from(offset + view));
        let (at, len) = thumb(&span, track.size()[axis], self.look.min_thumb);
        let mut min = track.min;
        min[axis] += at;
        let mut size = track.size();
        size[axis] = len;
        Some((track, Rect::from_min_size(min, size)))
    }
}

impl Element for ScrollNode {
    type Output = ();
    type Local = ScrollKept;

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, cx: &mut BuildCx<'_>) -> ScrollKept {
        let owner = cx.runtime().parent(cx.node());
        let rate = owner.and_then(|owner| glide_rate(cx, owner));
        ScrollKept::new(rate.unwrap_or_else(|| cx.state(Vec2::ZERO)))
    }

    /// Pays out an eased step and a glide, then places the children moved by the offset. Leaving
    /// the band writes the miss cell, so the body builds again in this frame.
    fn layout(&self, sk: &mut ScrollKept, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Vec2 {
        if let Some((dir, last)) = sk.strip {
            let now = cx.time_until(|now| Some(now + FRAME));
            let step = (now - last) as f32 * STRIP_ROWS_PER_SECOND * sk.look.line * dir;
            sk.strip = Some((dir, now));
            if !sk.move_to(sk.offset + Vec2::new(0.0, step)) && step != 0.0 {
                sk.strip = None;
            }
        }
        if sk.ease.owes() || sk.glide.is_some() {
            let now = cx.time_until(|now| Some(now + FRAME));
            let paid = sk.ease.step(now);
            let mut to = sk.offset - paid;
            if let Some(target) = sk.glide {
                let left = target - to;
                to = if left.length() <= GLIDE_SETTLE { target } else { to + left * GLIDE_SHARE };
                if to == target {
                    sk.glide = None;
                }
            }
            sk.move_to(to);
        }
        let rate = sk.rate.get(cx);
        if rate == Vec2::ZERO {
            sk.rate_at = None;
        } else {
            let now = cx.time_until(|_| None);
            let step = sk.rate_at.map_or(Vec2::ZERO, |last| rate * (now - last) as f32);
            // At the end the rate points to, the glide rests: no frames until the extent or the rate changes. A second
            // layout in the same frame pays nothing and is no rest.
            let rests = step != Vec2::ZERO && !sk.move_to(sk.offset + step);
            sk.rate_at = (!rests).then_some(now);
            if !rests {
                cx.time_until(|now| Some(now + FRAME));
            }
        }
        if sk.leaves_band()
            && let Some(misses) = sk.misses
        {
            misses.update(cx, |n| *n = n.wrapping_add(1));
        }
        if let Some(placed_at) = sk.placed_at {
            placed_at.set_if_changed(cx, sk.offset);
        }
        sk.kept.place_children(cx, sk.offset);
        // A collapsing container places its scroll body at the share of the view it shows; the clip in paint follows.
        constraints.constrain(sk.kept.size)
    }

    fn relayout_boundary(&self) -> bool {
        true
    }

    /// Clips the content to the view, fades the ends the view does not reach, draws the bars over
    /// it and records the hit shapes: the view takes wheels, a thumb takes presses.
    fn paint(&self, sk: &ScrollKept, cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        cx.clip(bounds);
        cx.hit(HitShape::Rect(bounds), Sense::WHEEL | Sense::WHEEL_X | Sense::WHEEL_Y, 0);
        let look = sk.look;
        let max = sk.max_offset();
        // The pointer over the view shows overlay bars and the stronger thumb: a flip repaints this node only.
        let hovered = cx.interaction_fact(InteractFact::Hovered);
        let bars = !self.scroll.overlay_hides_at_rest() || hovered || sk.grab.is_some();
        // A bar is in its hover look while the pointer is on its thumb and in its pressed look while dragged.
        let on_bar = cx.hovered_part();
        cx.over(|cx| {
            if self.scroll.vertical_on() {
                let clear = look.fade.faded(0.0);
                if sk.offset.y > 0.0 {
                    cx.vgradient(Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), look.fade_extent)), look.fade, clear);
                }
                if sk.offset.y < max.y {
                    let top = bounds.max.y - look.fade_extent;
                    cx.vgradient(Rect::from_min_max(Pos2::new(bounds.min.x, top), bounds.max), clear, look.fade);
                }
            }
            if self.scroll.has_strips() {
                let back = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), look.strip));
                let forward = Rect::from_min_max(Pos2::new(bounds.min.x, bounds.max.y - look.strip), bounds.max);
                for (i, (rect, shows)) in [(back, sk.offset.y > 0.0), (forward, sk.offset.y < max.y)].into_iter().enumerate() {
                    if shows {
                        cx.fill_rect(rect, look.fade);
                        cx.hit(HitShape::Rect(rect), Sense::DRAG | Sense::HOVER, STRIP_PARTS[i]);
                    }
                }
                return;
            }
            for axis in [0, 1] {
                let on = if axis == 1 { self.scroll.vertical_on() } else { self.scroll.horizontal_on() };
                if let Some((track, thumb)) = sk.bar(axis, bounds).filter(|_| on && bars) {
                    let state = match (sk.grab.is_some_and(|(at, ..)| at == axis), on_bar == Some(BAR_PARTS[axis])) {
                        (true, _) => LookState::Pressed,
                        (false, true) => LookState::Hover,
                        _ => LookState::Rest,
                    };
                    let radius = look.thickness / 2.0;
                    let track_fill = look.track.get(state);
                    if track_fill.0[3] > 0 {
                        cx.rect(track, radius, track_fill, None);
                    }
                    // The thumb lies along the track's outer edge: its end side, or its bottom for a sideways bar.
                    let width = look.thumb_width.get(state).min(look.thickness);
                    let mut edge = thumb;
                    if axis == 1 {
                        edge.min.x = edge.max.x - width;
                    } else {
                        edge.min.y = edge.max.y - width;
                    }
                    cx.rect(edge, width / 2.0, look.thumb.get(state), None);
                    cx.hit(HitShape::Rect(thumb), Sense::DRAG | Sense::HOVER, BAR_PARTS[axis]);
                }
            }
        });
    }

    fn event(&self, sk: &mut ScrollKept, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        let moved = match ev {
            Event::Wheel(w) if !w.pinches() => {
                sk.glide = None;
                let mut delta = w.delta;
                if !self.scroll.horizontal_on() {
                    delta.x = 0.0;
                }
                if !self.scroll.vertical_on() {
                    delta.y = 0.0;
                }
                match w.unit {
                    WheelUnit::Line => {
                        let notch = notch_points(self.scroll.notch, sk.look.line, sk.view.y);
                        sk.ease.feed(delta * notch, cx.time());
                        cx.request_layout();
                        false
                    }
                    WheelUnit::Page => sk.move_to(sk.offset - delta * sk.view),
                    _ => sk.move_to(sk.offset - delta),
                }
            }
            Event::Pointer(p) => {
                let axis = BAR_PARTS.iter().position(|part| p.hit.is_some_and(|h| h.part == *part));
                let strip = STRIP_PARTS.iter().position(|part| p.hit.is_some_and(|h| h.part == *part));
                let along = |axis: usize| cx.to_local(p.pos)[axis];
                match p.gesture {
                    Gesture::Down { .. } if strip.is_some() => {
                        sk.glide = None;
                        sk.strip = strip.map(|i| (if i == 0 { -1.0 } else { 1.0 }, cx.time()));
                        cx.request_layout();
                        return Handled::Yes;
                    }
                    Gesture::Up { .. } | Gesture::Cancel(_) if sk.strip.is_some() => {
                        sk.strip = None;
                        return Handled::Yes;
                    }
                    Gesture::Down { .. } => {
                        if let Some(axis) = axis {
                            sk.glide = None;
                            sk.grab = Some((axis, along(axis), sk.offset[axis]));
                        }
                        return if axis.is_some() { Handled::Yes } else { Handled::No };
                    }
                    Gesture::Up { .. } => {
                        sk.grab = None;
                        false
                    }
                    _ => match sk.grab {
                        Some((axis, from, offset)) => {
                            let bounds = cx.bounds();
                            let Some((track, _)) = sk.bar(axis, bounds) else { return Handled::Yes };
                            let span = ScrollSpan::new(0.0..f64::from(sk.extent[axis]), f64::from(offset)..f64::from(offset + sk.view[axis]));
                            let to = drag(&span, track.size()[axis], sk.look.min_thumb, along(axis) - from);
                            let mut target = sk.offset;
                            target[axis] = to.start as f32;
                            sk.move_to(target)
                        }
                        None => return Handled::No,
                    },
                }
            }
            Event::Access(a) if a.action == accesskit::Action::ScrollIntoView => {
                reveal(sk, cx, a.target);
                // The scroll bodies around this one reveal the node too.
                return Handled::No;
            }
            Event::Access(a) => {
                let step = sk.view.y * OVERSCAN_SHARE;
                match a.action {
                    accesskit::Action::ScrollDown => sk.move_to(sk.offset + Vec2::new(0.0, step)),
                    accesskit::Action::ScrollUp => sk.move_to(sk.offset - Vec2::new(0.0, step)),
                    _ => return Handled::No,
                }
            }
            _ => return Handled::No,
        };
        if moved {
            cx.request_layout();
            cx.request_paint();
            cx.request_access();
            if sk.leaves_band() {
                cx.request_build();
            }
        }
        Handled::Yes
    }

    fn access(&self, sk: &ScrollKept, _: &mut AccessCx<'_>) -> Option<accesskit::Node> {
        let mut node = accesskit::Node::new(accesskit::Role::ScrollView);
        if self.scroll.vertical_on() {
            node.set_scroll_y(f64::from(sk.offset.y));
            node.set_scroll_y_min(0.0);
            node.set_scroll_y_max(f64::from(sk.max_offset().y));
            node.add_action(accesskit::Action::ScrollDown);
            node.add_action(accesskit::Action::ScrollUp);
        }
        if self.scroll.horizontal_on() {
            node.set_scroll_x(f64::from(sk.offset.x));
            node.set_scroll_x_min(0.0);
            node.set_scroll_x_max(f64::from(sk.max_offset().x));
        }
        Some(node)
    }
}

/// Sets the velocity at which the scroll body of `owner` (a container that scrolls its body, or a collection's rows
/// view) glides, in points per second along each axis, positive toward the content's end: a drag's edge scroll or
/// middle-button auto-scroll. The scroll node's layout pays it out every frame; nothing runs until the view leaves the
/// band the last build covered (which builds the owner) or the rate changes. A zero velocity stops it; a wheel, a bar
/// drag or `jump_to` leaves it running. Asked before the owner's scroll body exists (its first build), it does nothing.
pub(crate) fn glide(cx: &mut BuildCx<'_>, owner: Id, velocity: Vec2) {
    let Some(node) = node_of(cx, owner) else { return };
    // The cell is on the owner's node: its run asks for the glide while its build has not reached the scroll node yet.
    if let Some(rate) = glide_rate(cx, node) {
        rate.set_if_changed(cx, velocity);
    }
}

/// The glide rate cell of the scroll body directly under `owner`, on `owner`'s node; `None` when `owner` is not in
/// the tree.
fn glide_rate(cx: &mut BuildCx<'_>, owner: NodeId) -> Option<State<Vec2>> {
    cx.slot_of(owner, "craft-glide-rate", || Vec2::ZERO)
}

impl Ui<'_, '_> {
    /// Sets the velocity at which the scroll body of `owner` glides ([`glide`]): points per second, positive toward the
    /// content's end; zero stops it. For a drag's edge scroll or an auto-scroll outside a collection.
    pub fn glide(&mut self, owner: Id, velocity: Vec2) {
        glide(self.cx, owner, velocity);
    }
}

/// Starts a glide that brings `target` (a node inside the body) into the view: the least move
/// along each axis that shows its whole rect, or its start when it is larger than the view.
fn reveal(sk: &mut ScrollKept, cx: &mut EventCx<'_>, target: NodeId) {
    let Some(rect) = cx.window_rect(target) else { return };
    let local = Rect::from_min_max(cx.to_local(rect.min), cx.to_local(rect.max));
    let mut to = sk.glide.unwrap_or(sk.offset);
    for axis in [0, 1] {
        let (start, end, view) = (local.min[axis], local.max[axis], sk.view[axis]);
        if start < 0.0 || end - start > view {
            to[axis] += start;
        } else if end > view {
            to[axis] += end - view;
        }
    }
    let to = sk.clamp(to);
    if to != sk.offset {
        // Layout pays the glide out; leaving the built band on the way builds the body again.
        sk.glide = Some(to);
        cx.request_layout();
        cx.request_paint();
    }
}

/// The points one wheel line moves a body with `notch` whose view is `view` high.
fn notch_points(notch: Notch, line: f32, view: f32) -> f32 {
    notch.wheel(line).points(view)
}
