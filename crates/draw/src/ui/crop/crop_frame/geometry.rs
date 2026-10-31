//! The crop frame's pure geometry: the box, where a press lands on it, and what each gesture
//! makes of it (PhotoCraft's `tools/crop_ui.rs`).

use egui::Modifiers;
use kurbo::{Point, Rect, Vec2};

use craft_ui::canvas::Constraint;
use craft_ui::canvas::quad::BoxPart;

/// Smallest side (document units) a gesture may leave behind.
pub(super) const MIN_FRAME: f64 = 1.0;

/// The crop frame: a document rect and the angle it is turned by about its centre.
///
/// The rect is the frame before its turn, so ratios, whole-pixel readouts and the crop itself are
/// defined on it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropBox {
    /// The frame before its turn, in document units.
    pub rect: Rect,
    /// Degrees, clockwise on screen, -180..=180.
    pub angle: f64,
}

impl CropBox {
    /// An unturned frame.
    pub fn upright(rect: Rect) -> Self {
        Self { rect, angle: 0.0 }
    }

    /// The rect with `x0 <= x1` and `y0 <= y1`.
    fn rect_abs(&self) -> Rect {
        self.rect.abs()
    }

    /// Whether the rect and the angle are finite numbers.
    pub fn is_finite(&self) -> bool {
        let r = self.rect;
        [r.x0, r.y0, r.x1, r.y1, self.angle].iter().all(|v| v.is_finite())
    }

    /// The centre the frame turns about.
    pub fn center(&self) -> Point {
        self.rect.center()
    }

    /// The corners on the page, turned: top left, top right, bottom right, bottom left of the
    /// upright frame.
    pub fn corners(&self) -> [Point; 4] {
        let r = self.rect_abs();
        let c = r.center();
        [Point::new(r.x0, r.y0), Point::new(r.x1, r.y0), Point::new(r.x1, r.y1), Point::new(r.x0, r.y1)].map(|p| turn(p, c, self.angle))
    }

    /// The document point of `part` on the turned frame: a corner or an edge handle, or the
    /// centre for [`BoxPart::Inside`]; none for the other parts.
    pub fn part_point(&self, part: BoxPart) -> Option<Point> {
        let c = self.center();
        if part == BoxPart::Inside {
            return Some(c);
        }
        let (hx, hy) = part_handle(part)?;
        let r = self.rect_abs();
        Some(turn(Point::new(c.x + f64::from(hx) * r.width() / 2.0, c.y + f64::from(hy) * r.height() / 2.0), c, self.angle))
    }

    /// The axis-aligned bounds of the turned frame.
    pub fn turned_bounds(&self) -> Rect {
        let [first, rest @ ..] = self.corners();
        rest.iter().fold(Rect::from_points(first, first), |acc, &p| acc.union_pt(p))
    }

    /// The frame moved by `by` on the page, its angle kept.
    pub fn moved(&self, by: Vec2) -> Self {
        Self { rect: self.rect + by, ..*self }
    }

    /// Width and height swapped about the centre, the angle kept (X).
    pub fn swapped(&self) -> Self {
        let c = self.center();
        let (hw, hh) = (self.rect.width() / 2.0, self.rect.height() / 2.0);
        Self { rect: Rect::new(c.x - hh, c.y - hw, c.x + hh, c.y + hw), ..*self }
    }

    /// The size Enter would crop to: the frame's own width and height in whole document units, at
    /// least 1; `None` when they are not finite.
    pub fn crop_size(&self) -> Option<(f64, f64)> {
        let (w, h) = (self.rect.width(), self.rect.height());
        (w.is_finite() && h.is_finite()).then(|| (w.round().max(1.0), h.round().max(1.0)))
    }

    /// Whether both sides are at least one document unit.
    pub(super) fn size_ok(&self) -> bool {
        self.rect.width() >= MIN_FRAME && self.rect.height() >= MIN_FRAME
    }
}

/// `p` turned `deg` degrees clockwise on screen (y down) about `c`.
pub(super) fn turn(p: Point, c: Point, deg: f64) -> Point {
    if deg == 0.0 {
        return p;
    }
    let (s, k) = deg.to_radians().sin_cos();
    let (dx, dy) = (p.x - c.x, p.y - c.y);
    Point::new(c.x + k * dx - s * dy, c.y + s * dx + k * dy)
}

/// An angle in degrees brought into (-180, 180]; 0 for anything not finite.
pub(super) fn normalized_angle(deg: f64) -> f64 {
    if !deg.is_finite() {
        return 0.0;
    }
    let a = deg.rem_euclid(360.0);
    if a > 180.0 { a - 360.0 } else { a }
}

/// What a press lands on, in the frame's own axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Hit {
    /// An edge or corner: `hx`, `hy` are -1 (left, top), 1 (right, bottom) or 0.
    Handle(i8, i8),
    Inside,
    Outside,
}

/// The part for handle (`hx`, `hy`): corners clockwise from the top left, edges clockwise from the
/// top.
pub(super) fn handle_part(hx: i8, hy: i8) -> BoxPart {
    match (hx.signum(), hy.signum()) {
        (-1, -1) => BoxPart::Corner(0),
        (1, -1) => BoxPart::Corner(1),
        (1, 1) => BoxPart::Corner(2),
        (-1, 1) => BoxPart::Corner(3),
        (0, -1) => BoxPart::Edge(0),
        (1, 0) => BoxPart::Edge(1),
        (0, 1) => BoxPart::Edge(2),
        (-1, 0) => BoxPart::Edge(3),
        _ => BoxPart::Inside,
    }
}

/// The handle (`hx`, `hy`) of a corner or edge part.
pub(super) fn part_handle(part: BoxPart) -> Option<(i8, i8)> {
    match part {
        BoxPart::Corner(i) => Some([(-1, -1), (1, -1), (1, 1), (-1, 1)][usize::from(i % 4)]),
        BoxPart::Edge(i) => Some([(0, -1), (1, 0), (0, 1), (-1, 0)][usize::from(i % 4)]),
        _ => None,
    }
}

/// The part under `p` for the frame `b`: `tol` document units of reach. Outside the frame the
/// full reach applies, inside it at most a quarter of the side, so a small frame stays movable.
pub(super) fn hit(b: &CropBox, p: Point, tol: f64) -> Hit {
    let r = b.rect_abs();
    let p = turn(p, r.center(), -b.angle);
    let axis = |lo: f64, hi: f64, v: f64, other_in: bool| -> i8 {
        let t = tol.min((hi - lo) / 4.0).max(0.0);
        let near = |e: f64, outward: f64| (v - e) * outward <= tol && (e - v) * outward <= t;
        match (other_in, near(lo, -1.0), near(hi, 1.0)) {
            (false, ..) => 0,
            (_, true, true) => {
                if (v - lo).abs() <= (v - hi).abs() {
                    -1
                } else {
                    1
                }
            }
            (_, true, false) => -1,
            (_, false, true) => 1,
            _ => 0,
        }
    };
    let in_x = p.x >= r.x0 - tol && p.x <= r.x1 + tol;
    let in_y = p.y >= r.y0 - tol && p.y <= r.y1 + tol;
    let (hx, hy) = (axis(r.x0, r.x1, p.x, in_y), axis(r.y0, r.y1, p.y, in_x));
    if hx != 0 || hy != 0 {
        Hit::Handle(hx, hy)
    } else if p.x > r.x0 && p.x < r.x1 && p.y > r.y0 && p.y < r.y1 {
        Hit::Inside
    } else {
        Hit::Outside
    }
}

/// The frame angle after turning it from `from` degrees by the pointer's sweep about `c`, from
/// direction `grab` (radians) to `p`; `step` snaps it (⇧).
pub(super) fn turned_angle(c: Point, grab: f64, from: f64, p: Point, step: Option<f64>) -> f64 {
    let now = (p.y - c.y).atan2(p.x - c.x);
    let a = from + (now - grab).to_degrees();
    normalized_angle(match step {
        Some(step) if step > 0.0 => (a / step).round() * step,
        _ => a,
    })
}

/// The direction of `p` from `c`, in radians.
pub(super) fn direction(c: Point, p: Point) -> f64 {
    (p.y - c.y).atan2(p.x - c.x)
}

/// Frame `r` with handle (`hx`, `hy`) dragged by `d`. `alt` resizes about the centre; `ratio`
/// (width / height) keeps the proportions, the larger relative extent winning at a corner.
pub(super) fn resized(r: Rect, hx: i8, hy: i8, d: Vec2, ratio: Option<f64>, alt: bool) -> Rect {
    let r = r.abs();
    // Per axis: the fixed point, the signed extent from it, and whether it is centred (the extent
    // is then a half-extent).
    let axis = |lo: f64, hi: f64, h: i8, d: f64| -> (f64, f64, bool) {
        let c = (lo + hi) / 2.0;
        match h {
            1 if alt => (c, hi + d - c, true),
            1 => (lo, hi + d - lo, false),
            -1 if alt => (c, lo + d - c, true),
            -1 => (hi, lo + d - hi, false),
            _ => (c, (hi - lo) / 2.0, true),
        }
    };
    let (fx, mut ex, cx) = axis(r.x0, r.x1, hx, d.x);
    let (fy, mut ey, cy) = axis(r.y0, r.y1, hy, d.y);
    if let Some(k) = ratio.filter(|k| k.is_finite() && *k > 0.0) {
        let full = |e: f64, c: bool| e.abs() * if c { 2.0 } else { 1.0 };
        let (w, h) = (full(ex, cx), full(ey, cy));
        let (w, h) = match (hx != 0, hy != 0) {
            (true, true) => {
                let s = (w / k).max(h);
                (s * k, s)
            }
            (true, false) => (w, w / k),
            (false, true) => (h * k, h),
            (false, false) => (w, h),
        };
        let back = |e: f64, len: f64, c: bool| if e < 0.0 { -1.0 } else { 1.0 } * if c { len / 2.0 } else { len };
        ex = back(ex, w, cx);
        ey = back(ey, h, cy);
    }
    let span = |f: f64, e: f64, c: bool| if c { (f - e, f + e) } else { (f, f + e) };
    let ((ax, bx), (ay, by)) = (span(fx, ex, cx), span(fy, ey, cy));
    Rect::new(ax, ay, bx, by).abs()
}

/// [`resized`] for a frame turned by its angle: the handle moves along the frame's own axes by
/// the page drag `d`, and the edges opposite stay where they are on the page.
pub(super) fn resized_turned(b: &CropBox, hx: i8, hy: i8, d: Vec2, ratio: Option<f64>, alt: bool) -> CropBox {
    if b.angle == 0.0 {
        return CropBox { rect: resized(b.rect, hx, hy, d, ratio, alt), angle: 0.0 };
    }
    let c = b.center();
    let local = turn(Point::new(d.x, d.y), Point::ZERO, -b.angle);
    let n = resized(b.rect, hx, hy, Vec2::new(local.x, local.y), ratio, alt);
    let nc = turn(n.center(), c, b.angle);
    let (hw, hh) = (n.width() / 2.0, n.height() / 2.0);
    CropBox { rect: Rect::new(nc.x - hw, nc.y - hh, nc.x + hw, nc.y + hh), angle: b.angle }
}

/// A new frame drawn from `anchor` to `cur`: ⇧ square, ⌥ from the centre, `ratio` held.
pub(super) fn drawn(anchor: Point, cur: Point, ratio: Option<f64>, mods: Modifiers) -> Rect {
    let constraint = match ratio.filter(|k| k.is_finite() && *k > 0.0) {
        Some(k) => Constraint::Ratio(k, 1.0),
        None => Constraint::Free,
    };
    let (a, b) = constraint.corners(anchor, cur, mods.shift, mods.alt);
    Rect::from_points(a, b).abs()
}

/// `b` kept inside `bounds`: an unturned frame is cut to it; a turned one is moved by the least
/// that brings its bounds in, and left as it is when it cannot fit. `None` when a frame that
/// changes size cannot be kept inside (the caller keeps its last frame). With `held` (a ratio is
/// held), a cut that would change the frame's ratio is refused the same way.
pub(super) fn within(b: CropBox, bounds: Rect, resizing: bool, held: bool) -> Option<CropBox> {
    let bounds = bounds.abs();
    if b.angle == 0.0 {
        let r = b.rect.abs();
        let cut = Rect::new(r.x0.max(bounds.x0), r.y0.max(bounds.y0), r.x1.min(bounds.x1), r.y1.min(bounds.y1));
        if resizing {
            let kept = !held || (cut.width() * r.height() - cut.height() * r.width()).abs() <= 1e-9 * (r.width() * r.height()).max(1.0);
            return (kept && cut.width() >= 0.0 && cut.height() >= 0.0).then_some(CropBox { rect: cut, angle: 0.0 });
        }
        return Some(CropBox { rect: shifted_in(r, bounds), angle: 0.0 });
    }
    let t = b.turned_bounds();
    if resizing {
        let inside = t.x0 >= bounds.x0 && t.y0 >= bounds.y0 && t.x1 <= bounds.x1 && t.y1 <= bounds.y1;
        return inside.then_some(b);
    }
    let by = shifted_in(t, bounds).origin() - t.origin();
    Some(b.moved(by))
}

/// `r` moved by the least that brings it inside `bounds` (an edge of `bounds` first when `r` is
/// larger).
fn shifted_in(r: Rect, bounds: Rect) -> Rect {
    let fit = |lo: f64, hi: f64, b_lo: f64, b_hi: f64| {
        if hi - lo >= b_hi - b_lo || lo < b_lo {
            b_lo - lo
        } else if hi > b_hi {
            b_hi - hi
        } else {
            0.0
        }
    };
    r + Vec2::new(fit(r.x0, r.x1, bounds.x0, bounds.x1), fit(r.y0, r.y1, bounds.y0, bounds.y1))
}
