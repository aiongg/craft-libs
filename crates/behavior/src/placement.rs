//! Where windows go on the desktop: which monitor a window is on, and where the next window of a cascade opens.
//! Rects are in desktop logical points.

use emath::{Rect, Vec2, pos2};

use crate::monitor::Monitor;

/// The farthest a window or monitor may sit from the desktop origin.
pub(crate) const MAX_POSITION: f32 = 100_000.0;

/// The monitor `window` is on: the one showing most of it or, when none shows it, the nearest. By area, not by a
/// corner or the centre: on Windows a maximized window overhangs its monitor on every side. Monitors with a
/// non-finite or empty rect are skipped. `None` only when there is no usable monitor.
pub fn monitor_of(monitors: &[Monitor], window: Rect) -> Option<&Monitor> {
    let shown = |monitor: &Monitor| {
        let overlap = window.intersect(monitor.rect);
        overlap.width().max(0.0) * overlap.height().max(0.0)
    };
    let gap = |monitor: &Monitor| {
        let across = (monitor.rect.min.x - window.max.x).max(window.min.x - monitor.rect.max.x).max(0.0);
        let down = (monitor.rect.min.y - window.max.y).max(window.min.y - monitor.rect.max.y).max(0.0);
        across * across + down * down
    };
    monitors.iter().filter(|monitor| usable(monitor)).max_by(|a, b| shown(a).total_cmp(&shown(b)).then_with(|| gap(b).total_cmp(&gap(a))))
}

/// Whether a monitor report can place a window: finite, nonempty and of sane size.
pub(crate) fn usable(monitor: &Monitor) -> bool {
    let (min, size) = (monitor.rect.min, monitor.rect.size());
    [min.x, min.y, size.x, size.y].iter().all(|v| v.is_finite() && v.abs() <= MAX_POSITION) && size.x > 0.0 && size.y > 0.0
}

/// Where a window of `size` opens after the one at `previous`: one `step` right and down. An axis on which that
/// would leave `bounds` starts again at the edge of `bounds`, so a long run of windows walks down the screen and
/// wraps back to the top left. A window larger than `bounds` goes to the corner of `bounds`; its size never
/// changes.
pub fn cascade(previous: Rect, size: Vec2, bounds: Rect, step: f32) -> Rect {
    let axis = |previous: f32, length: f32, low: f32, high: f32| {
        let at = previous + step;
        if at < low || at + length > high { low } else { at }
    };
    let at = pos2(axis(previous.min.x, size.x, bounds.min.x, bounds.max.x), axis(previous.min.y, size.y, bounds.min.y, bounds.max.y));
    Rect::from_min_size(at, size)
}

/// `n` windows of `size` in a cascade inside `area`: the first at its top left, each next one [`cascade`]d from the
/// one before. The host passes its cascade step (`size.window.cascade`).
pub fn cascade_rects(area: Rect, size: Vec2, n: usize, step: f32) -> Vec<Rect> {
    let mut rects: Vec<Rect> = Vec::with_capacity(n);
    for _ in 0..n {
        let next = rects.last().map_or_else(|| Rect::from_min_size(area.min, size), |last| cascade(*last, size, area, step));
        rects.push(next);
    }
    rects
}

#[cfg(test)]
mod tests {
    use super::*;
    use emath::vec2;

    fn monitor(x: f32, y: f32, w: f32, h: f32) -> Monitor {
        Monitor::new(None, Rect::from_min_size(pos2(x, y), vec2(w, h)), 1.0)
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(pos2(x, y), vec2(w, h))
    }

    #[test]
    fn a_window_is_on_the_monitor_showing_most_of_it() {
        let left = monitor(-1920.0, 200.0, 1920.0, 1080.0);
        let main = monitor(0.0, 0.0, 3840.0, 1600.0);
        let below = monitor(718.0, 1600.0, 1280.0, 720.0);
        let desktop = [left.clone(), main.clone(), below.clone()];
        assert_eq!(monitor_of(&desktop, rect(-8.0, -8.0, 3856.0, 1576.0)), Some(&main), "maximized, its corner outside");
        assert_eq!(monitor_of(&desktop, rect(-1928.0, 192.0, 1936.0, 1056.0)), Some(&left), "maximized, on the left");
        assert_eq!(monitor_of(&desktop, rect(800.0, 1700.0, 400.0, 300.0)), Some(&below), "offset and smaller");
        assert_eq!(monitor_of(&desktop, rect(-100.0, 300.0, 1000.0, 500.0)), Some(&main), "across two, mostly on one");
        assert_eq!(monitor_of(std::slice::from_ref(&main), rect(3400.0, 100.0, 800.0, 600.0)), Some(&main), "past the right edge");
    }
}
