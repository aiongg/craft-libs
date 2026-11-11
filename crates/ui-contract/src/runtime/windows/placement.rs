//! Saved window placement: reading, writing and recording it in the placement store.

use super::host::scale_of;
use super::{Geometry, Record, shared};
use crate::runtime::egui::viewports;
use crate::runtime::frame::PerViewport;
use craft_window::restore::Placement;
use egui::{Context, Pos2, ViewportId};

/// The placement saved under `key`.
pub(crate) fn saved_placement(ctx: &Context, key: &str) -> Option<Placement> {
    let store = shared(ctx, |shared| shared.store.clone())?;
    let store = store.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    store.get(key)
}

/// Saves `placement` under `key`; the end hook writes the file.
pub(crate) fn save_placement(ctx: &Context, key: &str, placement: &Placement) {
    let Some(store) = shared(ctx, |shared| shared.store.clone()) else { return };
    store.lock().unwrap_or_else(std::sync::PoisonError::into_inner).set(key, placement);
}

/// The outer position of a saved placement in desktop points.
pub(super) fn saved_position(placement: &Placement) -> Option<Pos2> {
    let scale = placement.scale.max(f32::EPSILON);
    placement.position_px.map(|[x, y]| Pos2::new(x as f32 / scale, y as f32 / scale))
}

/// Keeps the window's placement in the store: its size and position unless it is maximized or full screen, which
/// keep the last normal size.
pub(super) fn record_placement(ctx: &Context, viewport: ViewportId, key: &str, facts: &viewports::Facts) {
    let Some(inner) = facts.inner else { return };
    let geometry = Geometry { inner, outer: facts.outer, scale: facts.native_scale, maximized: facts.maximized, fullscreen: facts.fullscreen };
    let unchanged = PerViewport::<Record>::with(ctx, viewport, |record| record.placed.replace(geometry) == Some(geometry));
    if unchanged {
        return;
    }
    let Some(scale) = scale_of(ctx, facts) else { return };
    let mut next = saved_placement(ctx, key).unwrap_or_else(|| Placement::new(inner.size().into(), scale));
    next.scale = scale;
    next.maximized = facts.maximized;
    next.fullscreen = facts.fullscreen;
    if !facts.maximized && !facts.fullscreen {
        next.normal.size = inner.size().into();
        if let Some(outer) = facts.outer {
            next.normal.pos = [outer.min.x, outer.min.y];
            next.position_px = Some([(outer.min.x * scale).round() as i32, (outer.min.y * scale).round() as i32]);
        }
    }
    save_placement(ctx, key, &next);
}
