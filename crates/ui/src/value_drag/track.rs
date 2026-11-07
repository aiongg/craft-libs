//! The track and fill look shared by `Slider`, `RangeSlider` and `ThumbRow`: a rounded groove across the control, with
//! the part between two points along it filled.

use crate::primitives::{Draw, PaintShape};
use crate::style::{StyleColor, StyleCx};
use craft_behavior::Flow;
use craft_theme::{Rgba, Stateful, States};
use egui::{CornerRadius, Rect, Vec2};

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
        }
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
