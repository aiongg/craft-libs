//! Step 5, measure, and the size guard.

use crate::contract::cx::MeasureCx;
use crate::contract::paint_ui::UiHost;
use crate::contract::{Component, Resolved, Sizing, ValueOf};
use crate::runtime::env::Env;
use crate::style::{ComponentStyle, StyleView};
use craft_behavior::{Insets, Proposal};
use craft_ui_rt::Vec2;
use std::cell::RefCell;

/// What step 5 reads besides the component.
pub(crate) struct MeasureIn<'a> {
    pub env: &'a Env,
    pub proposal: Proposal,
    pub room: Option<f32>,
    /// The run's `Ui`, lent per call: text shapes through its window's engine, and children measure
    /// through their size cache on it.
    pub host: &'a RefCell<&'a mut (dyn UiHost + 'a)>,
}

/// Step 5, measure: the `content` size plus padding, then `max(min_size)`, and for any proposal but `Min` at least the
/// ideal width, up to the room, unless the component is sized to its content (`sizing`, a leaf's `Sizing::Content`).
/// At `Min` a least width the style sets takes the place of `min_size`'s width. `extent` is what the shape adds to the
/// content: a body's remembered size or a collection's content size. The padding is the style's and the props' (`padding`).
pub(crate) fn measure(style: &dyn StyleView, input: &MeasureIn<'_>, content: Vec2, extent: Vec2, resolved: &Resolved<'_>) -> Vec2 {
    let (metrics, sizing) = (style.metrics(), resolved.sizing);
    let floor = match (input.proposal, metrics.least_width) {
        (Proposal::Min, Some(least)) => Vec2::new(least, metrics.min_size.y),
        _ => metrics.min_size,
    };
    let mut size = padded(style, content + extent, resolved).max(floor);
    if input.proposal != Proposal::Min && sizing != Some(Sizing::Content) {
        size.x = size.x.max(input.room.map_or(metrics.ideal_width, |room| metrics.ideal_width.min(room)));
    }
    size
}

/// The least width at which the `content` an `Ideal` measure gave shows whole: step 5's width without the ideal
/// width, which is room the style asks for beyond the content. A row's shrink step takes that room first.
pub(crate) fn whole(style: &dyn StyleView, content: Vec2, extent: Vec2, resolved: &Resolved<'_>) -> f32 {
    padded(style, content + extent, resolved).max(style.metrics().min_size).x
}

/// `content` plus the padding of the style and the props.
fn padded(style: &dyn StyleView, content: Vec2, resolved: &Resolved<'_>) -> Vec2 {
    let padding = padding(style.metrics().padding, resolved.pad);
    content + Vec2::new(padding.start + padding.end, padding.top + padding.bottom)
}

/// The padding between the component's edge and its content: `padding` (the style's, or `Component::padding` where
/// the component is shown), and `pad` on every side that the shape's props add (`Resolved::pad`).
pub(crate) fn padding(padding: Insets, pad: f32) -> Insets {
    Insets::new(padding.start + pad, padding.end + pad, padding.top + pad, padding.bottom + pad)
}

/// The read view `measure` and `parts` get. Its room is the content's: the parent's room less the padding
/// `measure` adds.
pub(crate) fn measure_cx<'a, C: Component>(
    input: &MeasureIn<'a>,
    style: &'a C::Style,
    resolved: &'a Resolved<'a>,
    value: Option<&'a ValueOf<C>>,
) -> MeasureCx<'a, C> {
    let padding = padding(ComponentStyle::metrics(style).padding, resolved.pad);
    MeasureCx {
        env: input.env.view(),
        scope: input.env,
        style,
        resolved,
        proposal: input.proposal,
        room: input.room.map(|room| (room - padding.start - padding.end).max(0.0)),
        value,
        host: input.host,
        tokens: &input.env.tokens,
        ink: input.env.ink,
        id: resolved.id,
        enabled_size: (resolved.enabled, resolved.size),
    }
}
