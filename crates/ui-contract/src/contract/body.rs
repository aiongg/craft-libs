//! The body's shared names: the parts a container's own regions declare (move band, reorder band), the padding and
//! grip measures, the keep rule for a layer's bounds, and the header slots `Container::header_parts` reserves. The
//! body itself is laid out on the `Ui` cursor (`contract/rt/container.rs`).

use crate::contract::Pad;
use crate::style::UiTokens;
use craft_behavior::PartKey;
use craft_ui_rt::{Rect, Vec2};

/// The move band of a layer's or window's header: the part a drag moves the container by.
pub(crate) const MOVE_BAND: PartKey = PartKey::new("move_band");

/// The header band that drags a section to a new place among its siblings (`DragBand::Reorder`).
#[doc(hidden)]
pub const REORDER_BAND: PartKey = PartKey::new("reorder_band");

/// The slots of a container's own header parts at the edges of the header's line (`Container::header_parts`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct HeaderSlots {
    pub start: Option<Rect>,
    pub end: Option<Rect>,
}

/// The padding of tier `pad` in points.
pub fn pad_of(tokens: &UiTokens, pad: Pad) -> f32 {
    pad.points(tokens.tokens())
}

/// The width of a window's edge grips, and of the band a layer keeps clear above its header for its top grip:
/// `size.handle.sm`. A corner grip reaches twice as far along each of its edges.
pub fn grip_band(tokens: &UiTokens) -> f32 {
    tokens.tokens().size.handle.sm
}

/// `rect` moved, at its size, the least distance that puts it inside `bounds`: the one keep rule for a layer's
/// bounds, whether the user moves the layer or the host places it. On an axis where it is longer than the bounds, its
/// low edge sits on theirs.
pub fn keep_inside(rect: Rect, bounds: Rect) -> Rect {
    let keep = |low: f32, high: f32, bounds_low: f32, bounds_high: f32| low.min(bounds_high - (high - low)).max(bounds_low) - low;
    rect.translate(Vec2::new(keep(rect.min.x, rect.max.x, bounds.min.x, bounds.max.x), keep(rect.min.y, rect.max.y, bounds.min.y, bounds.max.y)))
}
