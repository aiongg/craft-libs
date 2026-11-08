//! Where native floating windows go on the desktop. Rects are in desktop logical points.

use emath::{Pos2, Rect, Vec2, pos2, vec2};

use crate::monitor::Monitor;
use crate::placement::{MAX_POSITION, monitor_of, usable};

/// Largest native window.
const MAX_SIZE: f32 = 16_384.0;
const MIN_WIDTH: f32 = 160.0;
const MIN_HEIGHT: f32 = 100.0;
/// The extent a non-finite width or height falls back to.
const FALLBACK_WIDTH: f32 = 420.0;
const FALLBACK_HEIGHT: f32 = 320.0;
/// Width of a window's bar that must lie on one monitor for the window to be reachable: wide
/// enough to grab. Half the bar's height must lie there too.
const REACHABLE_WIDTH: f32 = 40.0;

/// A rect safe to hand to the windowing system: finite, its size and position bounded.
/// Hostile workspace files and bad monitor reports included.
pub fn sanitized(rect: Rect) -> Rect {
    let finite = |v: f32, fallback: f32| if v.is_finite() { v } else { fallback };
    let size = rect.size();
    Rect::from_min_size(
        pos2(finite(rect.min.x, 0.0).clamp(-MAX_POSITION, MAX_POSITION), finite(rect.min.y, 0.0).clamp(-MAX_POSITION, MAX_POSITION)),
        vec2(finite(size.x, FALLBACK_WIDTH).clamp(MIN_WIDTH, MAX_SIZE), finite(size.y, FALLBACK_HEIGHT).clamp(MIN_HEIGHT, MAX_SIZE)),
    )
}

/// Whether enough of the `bar`-high bar along the top of `rect` lies on one of `monitors` to
/// grab it.
fn reachable(rect: Rect, monitors: &[Monitor], bar: f32) -> bool {
    let bar_rect = Rect::from_min_size(rect.min, vec2(rect.width(), bar));
    monitors.iter().filter(|m| usable(m)).any(|m| {
        let on = bar_rect.intersect(m.rect);
        on.width() >= REACHABLE_WIDTH.min(rect.width()) && on.height() >= bar / 2.0
    })
}

/// `rect` made safe (its bar `bar` high), and moved wholly onto the nearest of `monitors` when too little of its
/// bar lies on any to grab it. Without a monitor list, only made safe.
pub fn kept_reachable(rect: Rect, monitors: &[Monitor], bar: f32) -> Rect {
    let rect = sanitized(rect);
    if !monitors.iter().any(usable) || reachable(rect, monitors, bar) {
        return rect;
    }
    monitor_of(monitors, rect).map_or(rect, |m| clamp_onto(rect, m.rect))
}

/// `rect` moved, and shrunk if it must be, to lie wholly on `area`.
fn clamp_onto(rect: Rect, area: Rect) -> Rect {
    let size = vec2(rect.width().min(area.width()), rect.height().min(area.height()));
    // `area.max - size` can round below `area.min` when the sizes are equal, so no `clamp`, which panics then.
    let at = pos2(rect.min.x.min(area.max.x - size.x).max(area.min.x), rect.min.y.min(area.max.y - size.y).max(area.min.y));
    sanitized(Rect::from_min_size(at, size))
}

/// What a saved native window is placed against.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct NativeSpace<'a> {
    /// The connected monitors; empty when the platform lists none.
    pub monitors: &'a [Monitor],
    /// The main window's desktop rect.
    pub main: Option<Rect>,
    /// Without a monitor list: the size of the monitor the main window is on.
    pub monitor_size: Option<Vec2>,
    /// Height of a window's bar, which must stay reachable.
    pub bar: f32,
}

impl<'a> NativeSpace<'a> {
    /// A space of `monitors` whose windows have a `bar`-high bar, with no main window known.
    pub fn new(monitors: &'a [Monitor], bar: f32) -> Self {
        Self { monitors, main: None, monitor_size: None, bar }
    }
}

/// Where a native window from a saved layout opens.
///
/// - Its saved monitor (`monitor`), found in the monitor list by name or else by rect, is connected:
///   the saved rect, moved in proportion if that monitor's rect changed, and onto it if too
///   little of its bar lies on a monitor to grab it.
/// - It is not: the main window's monitor, at the same offset from its origin.
/// - No monitor list: the main window's monitor as the platform reports it, by its size alone. Its
///   origin is taken as the multiple of that size nearest below the main window's center,
///   which holds for a row or grid of monitors of one size.
///
/// A window that moves to another monitor ends up wholly on it.
pub fn restore_rect(saved: Rect, monitor: Option<&Monitor>, space: NativeSpace<'_>) -> Rect {
    let NativeSpace { monitors, main, monitor_size, bar } = space;
    let saved = sanitized(saved);
    let main = main.filter(|r| r.is_finite()).map(|r| Rect::from_min_size(r.min, vec2(r.width().max(1.0), r.height().max(1.0))));
    if !monitors.iter().any(usable) {
        let (Some(main), Some(size)) = (main, monitor_size.filter(|s| s.is_finite() && s.x >= 1.0 && s.y >= 1.0)) else { return saved };
        let center = main.center();
        let origin = Pos2::new((center.x / size.x).floor() * size.x, (center.y / size.y).floor() * size.y);
        return clamp_onto(saved, Rect::from_min_size(origin, size));
    }
    let main_monitor = main.and_then(|main| monitor_of(monitors, main)).or_else(|| monitors.iter().find(|m| usable(m)));
    let connected = monitor.and_then(|old| {
        let by_name = old.name.as_ref().and_then(|name| monitors.iter().filter(|m| usable(m)).find(|m| m.name.as_ref() == Some(name)));
        by_name.or_else(|| monitors.iter().filter(|m| usable(m)).find(|m| m.rect == old.rect)).map(|now| (old, now))
    });
    match (connected, monitor) {
        (Some((old, now)), _) if old.rect == now.rect => {
            if reachable(saved, monitors, bar) {
                saved
            } else {
                clamp_onto(saved, now.rect)
            }
        }
        (Some((old, now)), _) => {
            let (scale_x, scale_y) =
                if usable(old) { (now.rect.width() / old.rect.width(), now.rect.height() / old.rect.height()) } else { (1.0, 1.0) };
            let offset = saved.min - old.rect.min;
            clamp_onto(Rect::from_min_size(now.rect.min + vec2(offset.x * scale_x, offset.y * scale_y), saved.size()), now.rect)
        }
        (None, Some(old)) => {
            let Some(target) = main_monitor else { return saved };
            let offset = if usable(old) { saved.min - old.rect.min } else { Vec2::ZERO };
            clamp_onto(Rect::from_min_size(target.rect.min + offset, saved.size()), target.rect)
        }
        // No saved monitor: the window stays where its bar can be grabbed.
        (None, None) => {
            if reachable(saved, monitors, bar) {
                return saved;
            }
            main_monitor.map_or(saved, |target| clamp_onto(saved, target.rect))
        }
    }
}
