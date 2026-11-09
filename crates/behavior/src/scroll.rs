//! Scroll positions and the range a scroll part shows.

use crate::intent::NavMove;
use std::ops::Range;

/// Offsets and extents in content units. They are `f64` so a sheet of a million rows scrolls exactly.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ScrollPos {
    /// Offset along the inline axis, from the start edge.
    pub x: f64,
    /// Offset along the block axis.
    pub y: f64,
}

/// The range a scroll part shows. `content` is what the part's track stands for (it may start below zero
/// for overscroll), `visible` is the thumb. A range, not a `ScrollPos`: a view value can start below zero
/// and its visible length is the user's to change.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScrollSpan {
    /// The extent the track stands for.
    pub content: Range<f64>,
    /// The extent the thumb covers.
    pub visible: Range<f64>,
    /// Whether the thumb ends change the visible length (zoom).
    pub resize_ends: bool,
}

impl ScrollSpan {
    /// A span whose thumb ends do not resize.
    pub const fn new(content: Range<f64>, visible: Range<f64>) -> ScrollSpan {
        ScrollSpan { content, visible, resize_ends: false }
    }

    /// Lets the thumb ends change the visible length.
    pub const fn resize_ends(mut self, on: bool) -> ScrollSpan {
        self.resize_ends = on;
        self
    }
}

/// The thumb of `span` on a track `track` points long: its start along the track and its length, at least
/// `min_thumb`. The thumb stands for the visible range inside the content range.
pub fn thumb(span: &ScrollSpan, track: f32, min_thumb: f32) -> (f32, f32) {
    let content = span.content.end - span.content.start;
    let visible = span.visible.end - span.visible.start;
    let share = if content > 0.0 { (visible / content).clamp(0.0, 1.0) } else { 1.0 };
    let len = (f64::from(track) * share).max(f64::from(min_thumb.min(track))) as f32;
    let travel = f64::from((track - len).max(0.0));
    let room = (content - visible).max(f64::EPSILON);
    let at = ((span.visible.start - span.content.start) / room).clamp(0.0, 1.0);
    ((travel * at) as f32, len)
}

/// The visible range after the thumb moved `delta` points along the track, kept inside the content range.
pub fn drag(span: &ScrollSpan, track: f32, min_thumb: f32, delta: f32) -> Range<f64> {
    let (_, len) = thumb(span, track, min_thumb);
    let travel = f64::from((track - len).max(0.0));
    let visible = span.visible.end - span.visible.start;
    let room = (span.content.end - span.content.start - visible).max(0.0);
    let shift = if travel > 0.0 { f64::from(delta) * room / travel } else { 0.0 };
    shifted(span, shift)
}

/// The visible range one visible length further toward the end (or the start), kept inside the content range.
pub fn page(span: &ScrollSpan, toward_end: bool) -> Range<f64> {
    let visible = span.visible.end - span.visible.start;
    shifted(span, if toward_end { visible } else { -visible })
}

/// The visible range after a navigation move that scrolls: a page up or down, or to the start or the end of the
/// content. `None` for any other move.
pub fn jump(span: &ScrollSpan, mv: NavMove) -> Option<Range<f64>> {
    let visible = span.visible.end - span.visible.start;
    Some(match mv {
        NavMove::PageUp => page(span, false),
        NavMove::PageDown => page(span, true),
        NavMove::Start => span.content.start..span.content.start + visible,
        NavMove::End => shifted(span, span.content.end - span.content.start),
        _ => return None,
    })
}

fn shifted(span: &ScrollSpan, shift: f64) -> Range<f64> {
    let visible = span.visible.end - span.visible.start;
    let last = (span.content.end - visible).max(span.content.start);
    let start = (span.visible.start + shift).clamp(span.content.start, last);
    start..start + visible
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_the_thumb_to_the_end_of_the_track_shows_the_end_of_the_content() {
        let span = ScrollSpan::new(0.0..1000.0, 0.0..200.0);
        let (at, len) = thumb(&span, 100.0, 10.0);
        assert_eq!((at, len), (0.0, 20.0));
        let moved = drag(&ScrollSpan::new(0.0..1000.0, 0.0..200.0), 100.0, 10.0, 80.0);
        assert_eq!(moved, 800.0..1000.0);
        let past = drag(&span, 100.0, 10.0, 500.0);
        assert_eq!(past, 800.0..1000.0, "the range stops at the end of the content");
    }

    #[test]
    fn keyboard_and_page_moves_stop_at_the_ends() {
        let span = ScrollSpan::new(0.0..500.0, 100.0..300.0);
        let (content, view) = (0.0..500.0, 200.0);
        let start = jump(&span, NavMove::Start).unwrap();
        let end = jump(&span, NavMove::End).unwrap();
        assert_eq!((start.start, end.end), (content.start, content.end), "Home and End show the ends of the content");
        assert_eq!((start.end - start.start, end.end - end.start), (view, view));
        assert_eq!(jump(&span, NavMove::Down), None);
        // A page moves one visible length and stops at the ends.
        assert_eq!(page(&span, true), end);
        assert_eq!(page(&span, false), start);
    }
}
