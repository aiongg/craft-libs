//! Image mode (Photoshop's default crop mode): the frame stays upright on screen and the gestures
//! move and turn the image behind it, changing the frame and the view together (PhotoCraft's
//! `tools/crop_mode.rs`). The drawing and the drag state are Classic mode's; only the mapping from
//! pointer to frame differs, and the view follows the frame.

use egui::{Response, Ui, vec2};
use kurbo::{Point, Rect, Vec2};

use super::geometry::{normalized_angle, resized, turn};
use super::{CanvasView, CropBox, CropFrame, CropResponse, Drag, Kind};

impl CropFrame<'_> {
    /// [`show`](Self::show) in Image mode. The frame stays upright on screen: a drag inside moves
    /// the image under it (the frame and the view centre move by the same document vector, away
    /// from the pointer), a drag outside turns the image about the frame's centre (⇧ in
    /// [`turn_step`](Self::turn_step)s), and with [`auto_center`](Self::auto_center) a handle drag
    /// keeps the frame's centre on screen. Arrows move the image under the frame.
    ///
    /// The frame's events carry the box only; this call changes `view` itself, and a cancelled drag
    /// puts back both the box and the view as they were at the press. Every pointer point of a
    /// drag is read in the view of the press. The host sets the view with [`view_for`](Self::view_for)
    /// when the tool is picked and after a commit.
    pub fn show_image(mut self, ui: &mut Ui, canvas: &Response, view: &mut CanvasView) -> CropResponse {
        self.image = true;
        self.run(ui, canvas, view)
    }

    /// `view` turned so the frame `b` shows upright (rotation `-b.angle`), the frame's centre kept
    /// where it is on screen (PhotoCraft's `sync`).
    pub fn view_for(b: &CropBox, view: &CanvasView) -> CanvasView {
        Self::view_for_about(b, view, b.center())
    }

    /// [`view_for`](Self::view_for) keeping document point `pivot` where it is on screen
    /// (PhotoCraft's `pivot_center`). `view` itself when anything is not finite.
    pub fn view_for_about(b: &CropBox, view: &CanvasView, pivot: Point) -> CanvasView {
        let rotation = normalized_angle(-b.angle);
        let at = view.affine() * pivot;
        let turned = CanvasView { rotation, center: pivot, ..*view };
        let back = turned.affine().inverse() * at;
        let center = Point::new(2.0 * pivot.x - back.x, 2.0 * pivot.y - back.y);
        if b.is_finite() && center.x.is_finite() && center.y.is_finite() { CanvasView { center, ..turned } } else { *view }
    }

    /// The view after a drag step that gave frame `next`: from the view of the press, so the view
    /// moving with the image never feeds back into the drag.
    pub(super) fn follow(&self, d: &Drag, view: &mut CanvasView, next: &CropBox) {
        let Some(press) = d.press_view else { return };
        let slid = || CanvasView { center: press.center + (next.center() - d.at_press.center()), ..press };
        *view = match d.kind {
            Kind::Move => slid(),
            Kind::Resize { .. } if self.auto_center => slid(),
            Kind::Turn { .. } => Self::view_for_about(next, &press, d.at_press.center()),
            Kind::Draw { .. } | Kind::Resize { .. } => press,
        };
    }

    /// A drawn frame is moved to the middle of the canvas on release (Auto Center Preview).
    pub(super) fn centre_drawn(&self, d: &Drag, view: &mut CanvasView, shown: &CropBox) {
        if self.auto_center && matches!(d.kind, Kind::Draw { .. }) && d.press_view.is_some() {
            *view = Self::view_for(shown, view);
            view.center = shown.center();
        }
    }

    /// The view as it was at the press, after a cancel.
    pub(super) fn restore_view(&self, d: &Drag, view: &mut CanvasView) {
        if let Some(press) = d.press_view {
            *view = press;
        }
    }
}

/// The frame of Auto Center Preview: a handle dragged by `d` (document, in the press view) grows
/// or shrinks the frame about its centre, so the handle stays under the pointer while the frame
/// stays centred on screen. On the image the opposite edge stays put (the centre with ⌥), so the
/// frame's centre moves by the difference and the view follows it.
pub(super) fn centred_resize(at: &CropBox, hx: i8, hy: i8, d: Vec2, ratio: Option<f64>, alt: bool) -> Option<CropBox> {
    let r = at.rect.abs();
    let (w0, h0) = (r.width(), r.height());
    let local = turn(Point::new(d.x, d.y), Point::ZERO, -at.angle);
    let n = resized(Rect::new(-w0 / 2.0, -h0 / 2.0, w0 / 2.0, h0 / 2.0), hx, hy, Vec2::new(local.x, local.y), ratio, true);
    let (w1, h1) = (n.width(), n.height());
    let shift = if alt { Point::ZERO } else { Point::new(f64::from(hx) * (w1 - w0) / 2.0, f64::from(hy) * (h1 - h0) / 2.0) };
    let s = turn(shift, Point::ZERO, at.angle);
    let c = at.center() + Vec2::new(s.x, s.y);
    let next = CropBox { rect: Rect::new(c.x - w1 / 2.0, c.y - h1 / 2.0, c.x + w1 / 2.0, c.y + h1 / 2.0), angle: at.angle };
    next.is_finite().then_some(next)
}

/// The frame's angle when the image is turned from the press by the pointer's sweep about `c`
/// (from direction `grab` to `p`): the frame turns against the sweep. `step` snaps it (⇧).
pub(super) fn image_turned_angle(c: Point, grab: f64, from: f64, p: Point, step: Option<f64>) -> f64 {
    let sweep = ((p.y - c.y).atan2(p.x - c.x) - grab).to_degrees();
    let a = from - sweep;
    normalized_angle(match step {
        Some(step) if step > 0.0 => (a / step).round() * step,
        _ => a,
    })
}

/// The image's turn the readout shows: clockwise positive, the frame's angle undone, never -0.
pub(super) fn image_turn(frame_angle: f64) -> f64 {
    let a = normalized_angle(-frame_angle);
    if a == 0.0 { 0.0 } else { a }
}

/// An arrow step `by` (screen points) as the document vector it is in `view`, at one document
/// unit per point.
pub(super) fn screen_step(view: &CanvasView, by: Vec2) -> Vec2 {
    CanvasView { zoom: 1.0, aspect: 1.0, ..*view }.unmap_vec(vec2(by.x as f32, by.y as f32))
}

#[cfg(test)]
#[path = "image_mode_tests.rs"]
mod tests;
