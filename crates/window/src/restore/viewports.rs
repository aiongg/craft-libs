//! Further windows of a multi-window app (`craft_window::Viewports`): the first window saves under
//! [`MAIN`](super::MAIN), and each new document window opens cascaded from it.

use super::{NormalRect, Placement};
use crate::place::{Rect, cascade};

/// Where the `n`th window after the first opens (1 for the second window): `main`'s normal size,
/// each window `step` points right of and below the one before (see [`crate::cascade`]), within
/// `bounds`.
///
/// `step` and `bounds` are in points at zoom 1 in the desktop's coordinates (pixels divided by
/// `main.scale`), for example the monitor's work area; `None` wraps nowhere. Without a saved
/// position (Wayland) only the size carries over and the compositor places the window. The result
/// is never maximized or full screen. Build its viewport with [`super::apply`]; once windows are
/// open, `Viewports` cascades further ones from the most recent window itself.
pub fn cascade_window(main: &Placement, n: usize, step: f32, bounds: Option<Rect>) -> Placement {
    let mut p = main.clone();
    p.maximized = false;
    p.fullscreen = false;
    let Some(pos_px) = main.position_px else {
        p.normal.pos = [0.0; 2];
        return p;
    };
    let scale = if main.scale.is_finite() && main.scale > 0.0 { main.scale } else { 1.0 };
    let size = main.normal.size;
    let bounds = bounds.unwrap_or(Rect::new(f32::MIN / 4.0, f32::MIN / 4.0, f32::MAX / 2.0, f32::MAX / 2.0));
    let mut rect = Rect::new(pos_px[0] as f32 / scale, pos_px[1] as f32 / scale, size[0], size[1]);
    for _ in 0..n {
        rect = cascade(rect, size, bounds, step);
    }
    let px = [(rect.x * scale).round() as i32, (rect.y * scale).round() as i32];
    p.position_px = Some(px);
    p.normal = NormalRect { pos: [rect.x, rect.y], size };
    p.scale = scale;
    p
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn main_window() -> Placement {
        let mut p = Placement::new([800.0, 600.0], 2.0);
        p.position_px = Some([200, 100]);
        p.normal.pos = [100.0, 50.0];
        p.maximized = true;
        p
    }

    #[test]
    fn the_second_and_third_windows_step_from_main() {
        let main = main_window();
        let second = cascade_window(&main, 1, 28.0, None);
        assert_eq!((second.position_px, second.normal.size), (Some([256, 156]), [800.0, 600.0]));
        assert_eq!(second.normal.pos, [128.0, 78.0]);
        assert!(!second.maximized);
        let third = cascade_window(&main, 2, 28.0, None);
        assert_eq!(third.position_px, Some([312, 212]));
    }

    #[test]
    fn cascading_wraps_within_the_bounds() {
        let main = main_window();
        // 1000 × 700 pt: the third window's bottom (106 + 600) runs past 700 and starts over at the top.
        let third = cascade_window(&main, 2, 28.0, Some(Rect::new(0.0, 0.0, 1000.0, 700.0)));
        assert_eq!(third.position_px, Some([312, 0]));
    }

    #[test]
    fn without_a_position_only_the_size_carries_over() {
        let mut main = main_window();
        main.position_px = None;
        let second = cascade_window(&main, 1, 28.0, None);
        assert_eq!((second.position_px, second.normal.size), (None, [800.0, 600.0]));
    }
}
