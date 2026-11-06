//! `ScrollBar`: the track and thumb of a scroll part, with state.

use crate::style::{ScrollBarLook, StateSet};
use egui::{Rect, Shape, pos2, vec2};

/// Which way a bar runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BarAxis {
    /// Along x.
    Horizontal,
    /// Along y.
    Vertical,
}

/// The scroll bar painter.
pub struct ScrollBar;

impl ScrollBar {
    /// The thumb inside `track` for `viewport` points of `content` scrolled by `offset`: its length is the shown
    /// share of the track, at least `look.min_thumb`, and it sits at the offset's share of the travel.
    pub fn thumb(look: &ScrollBarLook, axis: BarAxis, track: Rect, content: f32, viewport: f32, offset: f32) -> Rect {
        let length = match axis {
            BarAxis::Horizontal => track.width(),
            BarAxis::Vertical => track.height(),
        };
        let share = if content > 0.0 { (viewport / content).clamp(0.0, 1.0) } else { 1.0 };
        let thumb = (length * share).max(look.min_thumb.min(length));
        let travel = (length - thumb).max(0.0);
        let range = (content - viewport).max(f32::EPSILON);
        let at = travel * (offset / range).clamp(0.0, 1.0);
        match axis {
            BarAxis::Horizontal => Rect::from_min_size(pos2(track.min.x + at, track.min.y), vec2(thumb, track.height())),
            BarAxis::Vertical => Rect::from_min_size(pos2(track.min.x, track.min.y + at), vec2(track.width(), thumb)),
        }
    }

    /// The shapes of the track and the thumb in `state`.
    pub fn shapes(look: &ScrollBarLook, track: Rect, thumb: Rect, state: &StateSet) -> Vec<Shape> {
        let radius = thumb.width().min(thumb.height()) / 2.0;
        vec![
            Shape::rect_filled(track, radius, state.resolve(&look.track).egui()),
            Shape::rect_filled(thumb, radius, state.resolve(&look.thumb).egui()),
        ]
    }
}
