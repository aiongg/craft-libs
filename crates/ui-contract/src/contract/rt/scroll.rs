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
//! - Anchors: `ScrollAnchor::End` follows the end while the view is at it; otherwise the first child
//!   in view keeps its place when content before it changes extent.
//! - Kinetic: line steps (mouse notches) gather in a `WheelEase` that layout pays out frame by
//!   frame; point steps (touchpads, which bring their own momentum) apply at once.

use craft_behavior::scroll::{drag, thumb};
use craft_behavior::{ScrollSpan, WheelEase};
use craft_theme::Tokens;
use craft_ui_rt::{
    AccessCx, BuildCx, Constraints, Element, Event, EventCx, Gesture, Handled, HitShape, LayoutCx, NodeId, PaintCx, Pos2, Rect, Rgba, Sense, State,
    Vec2, WheelUnit,
};

use super::node::{Kept, Placed};
use super::ui::Flow;
use crate::contract::{Notch, Scroll, ScrollAnchor};

/// The share of the view's extent built beyond each of its edges: a step within it moves the kept
/// content and builds nothing.
const OVERSCAN_SHARE: f32 = 0.5;

/// The part a bar thumb's hit shape names, per axis.
const BAR_PARTS: [u64; 2] = [1, 2];

/// A glide's step per frame: the share of the way left it covers.
const GLIDE_SHARE: f32 = 0.35;

/// A glide this close to its end, in points, ends there.
const GLIDE_SETTLE: f32 = 0.5;

/// The frame interval a glide or eased step asks layout for, in seconds.
const FRAME: f64 = 1.0 / 60.0;

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
    ease: WheelEase,
    glide: Option<Vec2>,
    /// A bar drag: the axis, the pointer position along it at the press and the offset then.
    grab: Option<(usize, f32, f32)>,
    /// The view is at the content's end (`ScrollAnchor::End` follows it).
    at_end: bool,
    /// The first child in view and its content position, kept in place across builds.
    anchor: Option<(NodeId, f32)>,
    look: Look,
}

/// The bar and fade look, resolved from the tokens at the last build.
#[derive(Clone, Copy)]
struct Look {
    thickness: f32,
    min_thumb: f32,
    thumb: Rgba,
    fade: Rgba,
    fade_extent: f32,
    line: f32,
}

impl Look {
    fn of(tokens: &Tokens) -> Look {
        Look {
            thickness: tokens.size.handle.sm,
            min_thumb: tokens.size.button,
            thumb: tokens.color.text.faint,
            fade: tokens.color.surface.chrome,
            fade_extent: tokens.space.gap.section,
            line: tokens.size.button,
        }
    }
}

impl ScrollKept {
    fn new() -> ScrollKept {
        ScrollKept {
            kept: Kept::new(),
            offset: Vec2::ZERO,
            extent: Vec2::ZERO,
            view: Vec2::ZERO,
            band: Rect::EVERYTHING,
            misses: None,
            ease: WheelEase::default(),
            glide: None,
            grab: None,
            at_end: true,
            anchor: None,
            look: Look { thickness: 0.0, min_thumb: 0.0, thumb: Rgba([0; 4]), fade: Rgba([0; 4]), fade_extent: 0.0, line: 0.0 },
        }
    }

    /// The build reads the band-miss cell, so a layout that leaves the band builds the body again.
    pub(crate) fn subscribe(&mut self, cx: &mut BuildCx<'_>) {
        let misses = *self.misses.get_or_insert_with(|| cx.state(0));
        misses.get(cx);
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
            self.glide = Some(self.clamp(at - content_origin));
        }
        self.offset = self.clamp(self.offset);
        self.at_end = self.offset.y >= self.max_offset().y;
        self.anchor = placed.iter().find(|p| p.offset.y + p.size.y > self.offset.y).map(|p| (p.node, p.offset.y));
        self.band = if flow.limited { overscan(view).translate(-content_origin.to_vec2()) } else { Rect::EVERYTHING };
        let placed = vec![Placed { node: content, offset: Vec2::ZERO, size: extent }];
        if self.kept.placed != placed || before != (self.offset, self.extent, self.view) || self.glide.is_some() {
            self.kept.placed = placed;
            self.kept.size = self.view;
            cx.mark_changed();
        }
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

    fn init(&self, _: &mut BuildCx<'_>) -> ScrollKept {
        ScrollKept::new()
    }

    /// Pays out an eased step and a glide, then places the children moved by the offset. Leaving
    /// the band writes the miss cell, so the body builds again in this frame.
    fn layout(&self, sk: &mut ScrollKept, cx: &mut LayoutCx<'_>, _: Constraints) -> Vec2 {
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
        if sk.leaves_band()
            && let Some(misses) = sk.misses
        {
            misses.update(cx, |n| *n = n.wrapping_add(1));
        }
        sk.kept.place_children(cx, sk.offset);
        sk.kept.size
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
            for axis in [0, 1] {
                let on = if axis == 1 { self.scroll.vertical_on() } else { self.scroll.horizontal_on() };
                if let Some((_, thumb)) = sk.bar(axis, bounds).filter(|_| on) {
                    cx.rect(thumb, look.thickness / 2.0, look.thumb, None);
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
                let along = |axis: usize| cx.to_local(p.pos)[axis];
                match p.gesture {
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

/// The points one wheel line moves a body with `notch` whose view is `view` high.
fn notch_points(notch: Notch, line: f32, view: f32) -> f32 {
    notch.wheel(line).points(view)
}
