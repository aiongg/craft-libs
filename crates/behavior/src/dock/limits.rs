//! Size limits and value checks for a dock layout.

use super::split::SplitSize;
use crate::monitor::Monitor;

/// Limits are checked before cloning or recursively modifying an application layout.
pub const MAX_DEPTH: usize = 64;
/// Most panels a layout may hold. Larger layouts fail with [`DockError::LimitExceeded`](super::DockError::LimitExceeded).
pub const MAX_PANELS: usize = 4096;
pub(super) const MAX_NODES: usize = MAX_PANELS * 2;
const MAX_EXTENT: f32 = 1_000_000.0;

pub(super) fn valid_rect(rect: [f32; 4]) -> bool {
    rect.iter().all(|v| v.is_finite() && v.abs() <= MAX_EXTENT) && rect.get(2).is_some_and(|v| *v > 0.0) && rect.get(3).is_some_and(|v| *v > 0.0)
}

/// A floating window's saved monitor and monitor size, each where given.
pub(super) fn valid_monitor(monitor: Option<&Monitor>, monitor_size: Option<[f32; 2]>) -> bool {
    monitor.is_none_or(|m| valid_rect(m.rect) && m.scale.is_finite() && m.scale > 0.0)
        && monitor_size.is_none_or(|[w, h]| valid_rect([0.0, 0.0, w, h]))
}

pub(super) fn valid_width(width: f32) -> bool {
    width.is_finite() && width > 0.0 && width <= MAX_EXTENT
}

pub(super) fn valid_size(size: SplitSize) -> bool {
    match size {
        SplitSize::Ratio(r) => r.is_finite() && (0.0..=1.0).contains(&r),
        SplitSize::FixedFirst(v) | SplitSize::FixedSecond(v) => v.is_finite() && (0.0..=MAX_EXTENT).contains(&v),
    }
}

pub(super) fn valid_height(height: Option<f32>) -> bool {
    height.is_none_or(|v| v.is_finite() && (0.0..=MAX_EXTENT).contains(&v))
}

pub(super) fn valid_active(len: usize, active: usize) -> bool {
    active < len || (len == 0 && active == 0)
}
