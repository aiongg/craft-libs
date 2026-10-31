//! Zoom: Photoshop's zoom range and Zoom In / Zoom Out ladder, Fit on Screen and Fill Screen, a zoom
//! about a point, the eased zoom of Animated Zoom, the Zoom tool's hold, scrub and rectangle, and
//! the window size of Zoom Resizes Windows.
//!
//! Zoom levels here are **device zoom**: device pixels per document unit, so 1 is 100 % at any
//! interface scale. [`CanvasView::zoom`] is screen points per document unit; the two convert with
//! [`device_zoom`] and [`CanvasView::points_per_unit`]. Functions that change a view take it
//! `&mut`: the app copies `center` and `zoom` back into its own view type.
//!
//! Every way of setting the zoom (wheel, Zoom tool, menu, zoom field, navigator, automation)
//! clamps through [`ZoomLadder::clamp`], so no control can pull the zoom into a narrower range of
//! its own.
//!
//! ```no_run
//! use craft_ui::canvas::{CanvasView, ZoomLadder, kurbo::Size, zoom};
//! # fn demo(view: &mut CanvasView, ppp: f32) {
//! let doc = Size::new(2000.0, 1500.0);
//! zoom::step_view(view, 1, doc, ppp); // View › Zoom In: the next rung, about the view centre
//! let percent = zoom::device_zoom(view, ppp) * 100.0;
//! # let _ = (percent, ZoomLadder::MAX);
//! # }
//! ```

use std::ops::RangeInclusive;

use egui::{Context, Id, Painter, Pos2, Rect as ScreenRect, Vec2 as ScreenVec};
use kurbo::{Point, Rect, Size};

use super::marquee::{Marquee, MarqueeLook};
use super::view::CanvasView;
use crate::theme::UiTokens;

/// A zoom within 0.1 % of a rung counts as on it.
const RUNG_TOLERANCE_UP: f64 = 1.001;
const RUNG_TOLERANCE_DOWN: f64 = 0.999;

/// Points kept free on each side of the document by Fit on Screen.
const FIT_MARGIN: f64 = 20.0;

/// Document units an animation's anchor may differ from the target view before it is dropped.
const ANCHOR_TOLERANCE: f64 = 0.01;

/// Photoshop's zoom range and the Zoom In / Zoom Out ladder, measured on Photoshop 25.4: up to
/// 12800 %, down to where the document's shorter side is 2 px on screen (no lower than 0.1 % for
/// a ladder step). A ladder step goes from any zoom to the next rung (880 %: in to 1200 %, out to
/// 800 %).
pub struct ZoomLadder;

impl ZoomLadder {
    /// 12800 %.
    pub const MAX: f64 = 128.0;

    /// Zoom In / Zoom Out rungs (1 = 100 %).
    pub const STEPS: [f64; 33] = [
        0.001,
        0.002,
        0.003,
        0.004,
        0.005,
        0.007,
        0.01,
        0.015,
        0.02,
        0.03,
        0.04,
        0.05,
        0.0625,
        1.0 / 12.0,
        0.125,
        1.0 / 6.0,
        0.25,
        1.0 / 3.0,
        0.5,
        2.0 / 3.0,
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        12.0,
        16.0,
        32.0,
        64.0,
        128.0,
    ];

    /// The lowest zoom for a document of `size` units: its shorter side 2 device pixels. An empty
    /// size (a view before its first frame) gives the lowest rung.
    // pc: crates/ui-egui/src/canvas/zoom_levels.rs:56 `min`.
    pub fn min(size: Size) -> f64 {
        match size.width.min(size.height) {
            short if short.is_finite() && short > 0.0 => (2.0 / short).min(Self::MAX),
            _ => Self::STEPS[0],
        }
    }

    /// `zoom` within the range for a document of `size`; a zoom that is not a finite number
    /// becomes 100 %.
    // pc: crates/ui-egui/src/canvas/zoom_levels.rs:65 `clamp`.
    pub fn clamp(zoom: f64, size: Size) -> f64 {
        if zoom.is_finite() { zoom.min(Self::MAX).max(Self::min(size)) } else { 1.0 }
    }

    /// Zoom In (`dir` > 0) or Zoom Out (`dir` < 0) from `zoom`: the next rung past it (a zoom
    /// within 0.1 % of a rung counts as on it), within the document's range.
    // pc: crates/ui-egui/src/canvas/zoom_levels.rs:89 `step`.
    pub fn step(zoom: f64, dir: i32, size: Size) -> f64 {
        Self::step_in_range(zoom, dir, Self::min(size)..=Self::MAX)
    }

    /// [`step`](Self::step) within an explicit range (a navigator or a zoom field with bounds of
    /// its own).
    // pc: crates/ui-egui/src/canvas/zoom_levels.rs:78 `step_in_range`.
    pub fn step_in_range(zoom: f64, dir: i32, range: RangeInclusive<f64>) -> f64 {
        let (lo, hi) = range.into_inner();
        let z = if zoom.is_finite() { zoom.min(hi).max(lo) } else { 1.0 };
        let next = if dir > 0 {
            Self::STEPS.iter().copied().find(|s| *s > z * RUNG_TOLERANCE_UP).unwrap_or(hi)
        } else {
            Self::STEPS.iter().rev().copied().find(|s| *s < z * RUNG_TOLERANCE_DOWN).unwrap_or(z)
        };
        if next.is_finite() { next.min(hi).max(lo) } else { z }
    }

    /// A zoom field's range in percent: the document's range widened to include `zoom` itself.
    /// A field narrower than the zoom would clamp it and write it back every pass.
    // pc: crates/ui-egui/src/canvas/zoom_levels.rs:72 `percent_range`.
    pub fn percent_range(zoom: f64, size: Size) -> RangeInclusive<f64> {
        let z = if zoom.is_finite() { zoom } else { 1.0 };
        (Self::min(size).min(z) * 100.0)..=(Self::MAX.max(z) * 100.0)
    }
}

/// `view.zoom` as device zoom (1 = 100 %) at `pixels_per_point`.
pub fn device_zoom(view: &CanvasView, pixels_per_point: f32) -> f64 {
    view.zoom * f64::from(pixels_per_point)
}

/// `view.zoom` for `device_zoom` at `pixels_per_point` (an invalid scale counts as 1).
fn points_per_unit(device_zoom: f64, pixels_per_point: f32) -> f64 {
    let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.0 { pixels_per_point } else { 1.0 };
    CanvasView::points_per_unit(device_zoom, ppp)
}

/// The document's width (with the pixel aspect) and height as the view draws them, and the
/// absolute sine and cosine of its rotation.
fn shown_extent(view: &CanvasView, size: Size) -> (f64, f64, f64, f64) {
    let aspect = if view.aspect.is_finite() && view.aspect > 0.0 { view.aspect } else { 1.0 };
    let (sin, cos) = view.rotation.to_radians().sin_cos();
    let exact = |v: f64| if v.abs() < 1e-12 { 0.0 } else { v.abs() };
    (size.width * aspect, size.height, exact(sin), exact(cos))
}

/// Fit on Screen: centre the document and zoom so all of it shows inside `view.rect` less a
/// 20-point margin on each side, never past 100 % (one document pixel per device pixel, so a
/// 150 % display does not open documents magnified). `size` is in document units; the view's
/// [`aspect`](CanvasView::aspect) and rotation are taken into account.
// pc: crates/ui-egui/src/canvas/view_xform.rs:161 `fit_view`.
pub fn fit_view(view: &mut CanvasView, size: Size, pixels_per_point: f32) {
    let (shown_w, shown_h, sin, cos) = shown_extent(view, size);
    let (bw, bh) = (shown_w * cos + shown_h * sin, shown_w * sin + shown_h * cos);
    let fit = ((f64::from(view.rect.width()) - 2.0 * FIT_MARGIN) / bw).min((f64::from(view.rect.height()) - 2.0 * FIT_MARGIN) / bh);
    let device = ZoomLadder::clamp((fit * f64::from(pixels_per_point)).min(1.0), size);
    view.zoom = CanvasView::points_per_unit(device, pixels_per_point);
    view.center = Point::new(size.width / 2.0, size.height / 2.0);
}

/// Fill Screen: centre the document and zoom until it covers `view.rect`; one axis may extend
/// past the canvas.
// pc: crates/ui-egui/src/canvas/view_xform.rs:224 `fill_view`.
pub fn fill_view(view: &mut CanvasView, size: Size, pixels_per_point: f32) {
    let (shown_w, shown_h, sin, cos) = shown_extent(view, size);
    let (rw, rh) = (f64::from(view.rect.width()), f64::from(view.rect.height()));
    // The canvas rectangle's extent along the document's axes, which the document must cover.
    let (ex, ey) = (rw * cos + rh * sin, rw * sin + rh * cos);
    let device = ZoomLadder::clamp((ex / shown_w).max(ey / shown_h) * f64::from(pixels_per_point), size);
    view.zoom = CanvasView::points_per_unit(device, pixels_per_point);
    view.center = Point::new(size.width / 2.0, size.height / 2.0);
}

/// Set the zoom to `device_zoom` (clamped by the caller) keeping the document point under
/// `screen` where it is, or, with Zoom Clicked Point to Center (`center_on_point`), moving it to
/// the view's centre.
// pc: crates/ui-egui/src/canvas/view_xform.rs:251 `zoom_about`.
pub fn zoom_about(view: &mut CanvasView, screen: Pos2, device_zoom: f64, center_on_point: bool, pixels_per_point: f32) {
    let before = view.to_doc(screen);
    if center_on_point {
        view.zoom = points_per_unit(device_zoom, pixels_per_point);
        view.center = before;
    } else {
        anchor_view(view, screen, before, device_zoom, pixels_per_point);
    }
}

/// Set the zoom to `device_zoom` with the document point `doc` shown at `screen`.
// pc: crates/ui-egui/src/tools/zoom_tool.rs:44 `anchor_view`.
pub fn anchor_view(view: &mut CanvasView, screen: Pos2, doc: Point, device_zoom: f64, pixels_per_point: f32) {
    view.zoom = points_per_unit(device_zoom, pixels_per_point);
    view.center = doc - view.unmap_vec(screen - view.rect.center());
}

/// Where a wheel or pinch zoom anchors for a pointer at `screen`: the pointer over the document,
/// else the document's nearest point (an anchor out on the pasteboard slides the image out of
/// the window after a few notches; Photoshop keeps it in view). Clamped in document space, so a
/// turned or mirrored view behaves the same.
// pc: crates/ui-egui/src/canvas/view_xform.rs:239 `wheel_anchor`.
pub fn wheel_anchor(view: &CanvasView, screen: Pos2, size: Size) -> Pos2 {
    let d = view.to_doc(screen);
    let usable = |v: f64| v.is_finite() && v > 0.0;
    if !usable(size.width) || !usable(size.height) || !d.x.is_finite() || !d.y.is_finite() {
        return screen;
    }
    let nearest = Point::new(d.x.clamp(0.0, size.width), d.y.clamp(0.0, size.height));
    if nearest == d { screen } else { view.to_screen(nearest) }
}

/// View › Zoom In (`dir` > 0) or Zoom Out (`dir` < 0): the next ladder rung
/// ([`ZoomLadder::step`]), the view's centre unchanged. Returns whether the zoom changed (it does
/// not at a limit). The app starts a [`ZoomAnimation`] with no anchor when Animated Zoom is on.
// pc: crates/ui-egui/src/panels/actions.rs:224 `"view.zoomIn"` / `"view.zoomOut"`.
pub fn step_view(view: &mut CanvasView, dir: i32, size: Size, pixels_per_point: f32) -> bool {
    let current = device_zoom(view, pixels_per_point);
    let next = ZoomLadder::step(current, dir, size);
    let changed = next.is_finite() && (next / current - 1.0).abs() > 1e-9;
    if changed {
        view.zoom = points_per_unit(next, pixels_per_point);
    }
    changed
}

/// Animated Zoom: a zoom command eases from the view on screen to the app's new view over
/// [`DURATION`](Self::DURATION) (smoothstep), keeping an anchor point under the pointer
/// throughout. The app keeps its own view at the target; the animation only changes what is
/// drawn. With Animated Zoom off, [`shown`](Self::shown) returns the target at once.
///
/// The state lives in egui memory under the canvas's id.
pub struct ZoomAnimation;

/// A running ease.
#[derive(Clone, Copy)]
struct Easing {
    from: CanvasView,
    anchor: Option<(Point, Pos2)>,
    /// The app's zoom and centre the ease heads for; `None` until the first pass after `begin`.
    target: Option<(f64, Point)>,
    started: f64,
    /// The view drawn last pass: where an ease restarts from.
    shown: CanvasView,
}

fn animation_id(id: Id) -> Id {
    id.with("zoom-animation")
}

impl ZoomAnimation {
    /// Seconds a zoom eases over.
    pub const DURATION: f64 = 0.22;

    /// Start easing from `from` (the view drawn last pass) toward whatever the app's view is on
    /// the next pass, with `anchor` (a document point and the screen point it stays at) for a
    /// zoom about the pointer. A new zoom while one runs starts from the view in between.
    // pc: crates/ui-egui/src/canvas/view_xform.rs:175 `animated_view` (zoom_animating, zoom_anchor).
    pub fn begin(ctx: &Context, id: Id, from: &CanvasView, anchor: Option<(Point, Pos2)>) {
        ctx.data_mut(|d| d.insert_temp(animation_id(id), Easing { from: *from, anchor, target: None, started: 0.0, shown: *from }));
    }

    /// The view to draw this pass for the app's `target`, requesting a repaint while easing.
    /// `animated` is the Animated Zoom preference. A target that the anchor no longer fits (a
    /// clamp, a canvas resize, another camera command) drops the anchor: the target always wins.
    // pc: crates/ui-egui/src/canvas/view_xform.rs:175 `animated_view`.
    pub fn shown(ctx: &Context, id: Id, target: &CanvasView, animated: bool) -> CanvasView {
        let key = animation_id(id);
        let Some(mut easing) = ctx.data(|d| d.get_temp::<Easing>(key)) else { return *target };
        if !animated {
            ctx.data_mut(|d| d.remove::<Easing>(key));
            return *target;
        }
        let now = ctx.input(|i| i.time);
        let goal = (target.zoom, target.center);
        if easing.target != Some(goal) {
            // The app's view moved while easing (or this is the first pass): ease on from what was drawn.
            if easing.target.is_some() {
                easing.from = easing.shown;
            }
            easing.target = Some(goal);
            easing.started = now;
        }
        // A clamp, a canvas resize or another camera command can leave the anchor out of line
        // with the target view: the target wins.
        if let Some((doc, screen)) = easing.anchor {
            let at = target.to_doc(screen);
            if (at.x - doc.x).abs() > ANCHOR_TOLERANCE || (at.y - doc.y).abs() > ANCHOR_TOLERANCE {
                easing.anchor = None;
            }
        }
        let elapsed = (now - easing.started) / Self::DURATION;
        let t = if elapsed.is_finite() { elapsed.clamp(0.0, 1.0) } else { 1.0 };
        if t >= 1.0 {
            ctx.data_mut(|d| d.remove::<Easing>(key));
            return *target;
        }
        let ease = t * t * (3.0 - 2.0 * t);
        let mut view = *target;
        view.zoom = easing.from.zoom + (target.zoom - easing.from.zoom) * ease;
        view.center = easing.from.center.lerp(target.center, ease);
        if let Some((doc, screen)) = easing.anchor
            && view.zoom.is_finite()
            && view.zoom > 0.0
        {
            view.center = doc - view.unmap_vec(screen - view.rect.center());
        }
        easing.shown = view;
        ctx.data_mut(|d| d.insert_temp(key, easing));
        ctx.request_repaint();
        view
    }

    /// Whether a zoom is easing on the canvas `id`.
    pub fn running(ctx: &Context, id: Id) -> bool {
        ctx.data(|d| d.get_temp::<Easing>(animation_id(id))).is_some()
    }

    /// Stop easing: the next [`shown`](Self::shown) is the target (a pan or a scroll took over).
    pub fn stop(ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.remove::<Easing>(animation_id(id)));
    }
}

/// The Zoom tool's press held still: past [`DELAY`](Self::DELAY) it zooms in continuously about
/// the press point (out with Alt) until the button comes up, [`RATE`](Self::RATE) doublings a
/// second of wall time at any frame rate. The release is then no click step. A press that moves
/// more than [`SLOP`](Self::SLOP) points first is a drag and never becomes a hold. Only with
/// Animated Zoom on; off, a hold is a click.
///
/// The Zoom tool keeps one in its own state from `press` to `release`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoldZoom {
    anchor: Pos2,
    doc: Point,
    pressed_at: f64,
    last: Option<f64>,
    dragged: bool,
}

impl HoldZoom {
    /// Seconds a press is held still before it zooms.
    pub const DELAY: f64 = 0.3;
    /// Doublings (halvings zooming out) per second.
    pub const RATE: f64 = 2.0;
    /// Screen points the pointer may wander before the press is a drag.
    pub const SLOP: f32 = 6.0;
    /// The longest frame gap a hold zooms over, so a stalled frame never jumps the zoom.
    pub const MAX_DT: f64 = 0.25;

    /// A press at `screen` over document point `doc` at `time`.
    pub fn press(screen: Pos2, doc: Point, time: f64) -> Self {
        Self { anchor: screen, doc, pressed_at: time, last: None, dragged: false }
    }

    /// Advance the hold for this pass with the pointer at `pointer`: once past the delay, zoom
    /// `view` about the press point (out when `out`). Returns `true` while the hold zooms (and
    /// asks for a repaint), so the tool skips its scrub and its click step.
    // pc: crates/ui-egui/src/canvas/zoom_hold.rs:65 `update`.
    pub fn advance(&mut self, view: &mut CanvasView, pointer: Pos2, time: f64, out: bool, size: Size, pixels_per_point: f32) -> bool {
        if self.dragged {
            return false;
        }
        if self.last.is_none() {
            // Moved before the hold began: a drag (scrubby zoom or rectangle), not a hold.
            if pointer.distance(self.anchor) > Self::SLOP {
                self.dragged = true;
                return false;
            }
            let begin = self.pressed_at + Self::DELAY;
            if time < begin {
                return false;
            }
            self.last = Some(begin);
        }
        let dt = time - self.last.unwrap_or(time);
        let current = device_zoom(view, pixels_per_point);
        let zoom = Self::zoom_after(current, dt, out, size);
        // At a limit the view stays as it is (a window wrapped around the image centred it).
        if (zoom / current - 1.0).abs() > 1e-12 {
            anchor_view(view, self.anchor, self.doc, zoom, pixels_per_point);
        }
        self.last = Some(time);
        true
    }

    /// Whether the hold has started zooming (the release is then no click step).
    pub fn zooming(&self) -> bool {
        self.last.is_some()
    }

    /// The zoom after holding for `dt` seconds at `zoom`, within the document's range. A
    /// non-finite or negative `dt` changes nothing; `dt` is capped at [`MAX_DT`](Self::MAX_DT).
    // pc: crates/ui-egui/src/canvas/zoom_hold.rs:48 `hold_zoom`.
    pub fn zoom_after(zoom: f64, dt: f64, out: bool, size: Size) -> f64 {
        let dt = if dt.is_finite() { dt.clamp(0.0, Self::MAX_DT) } else { 0.0 };
        let doublings = if out { -Self::RATE * dt } else { Self::RATE * dt };
        ZoomLadder::clamp(zoom * doublings.exp2(), size)
    }
}

/// A Zoom tool drag. With Scrubby Zoom on (Photoshop's default) the drag zooms continuously,
/// right in and left out, [`DOUBLING`](Self::DOUBLING) points per doubling, about the press
/// point. Off, it draws a rectangle and the release zooms to fit it (Alt: out, so the view
/// shrinks to fit the rectangle's content in its place).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomDrag {
    anchor: Pos2,
    doc: Point,
    zoom0: f64,
    corner: Option<Pos2>,
}

impl ZoomDrag {
    /// Horizontal drag in screen points that doubles (or halves) the zoom.
    pub const DOUBLING: f32 = 100.0;
    /// A rectangle narrower or shorter than this many points is a click.
    pub const CLICK: f32 = 3.0;

    /// A drag pressed at `screen` on `view`. `scrubby` is the Zoom tool's Scrubby Zoom option.
    // pc: crates/ui-egui/src/tools/zoom_tool.rs:55 `drag` (b.started).
    pub fn begin(view: &CanvasView, screen: Pos2, scrubby: bool, pixels_per_point: f32) -> Self {
        Self { anchor: screen, doc: view.to_doc(screen), zoom0: device_zoom(view, pixels_per_point), corner: (!scrubby).then_some(screen) }
    }

    /// The pointer moved to `pointer`: a scrub zooms `view` about the press point; a rectangle
    /// moves its corner (the view is unchanged).
    // pc: crates/ui-egui/src/tools/zoom_tool.rs:55 `drag`.
    pub fn drag(&mut self, view: &mut CanvasView, pointer: Pos2, size: Size, pixels_per_point: f32) {
        match self.corner.as_mut() {
            Some(corner) => *corner = pointer,
            None => {
                let zoom = Self::scrub_zoom(self.zoom0, pointer.x - self.anchor.x, size);
                // Only a new zoom re-anchors: a window that Zoom Resizes Windows grew around the
                // image has centred it since, and the release keeps that.
                if (zoom / device_zoom(view, pixels_per_point) - 1.0).abs() > 1e-12 {
                    anchor_view(view, self.anchor, self.doc, zoom, pixels_per_point);
                }
            }
        }
    }

    /// The button came up: a rectangle zooms `view` to fit it, or with `out` steps the zoom out
    /// about the press point; a scrub has nothing left to do. A rectangle smaller than
    /// [`CLICK`](Self::CLICK) points is a click (no zoom here: the tool steps).
    // pc: crates/ui-egui/src/tools/zoom_tool.rs:91 `zoom_to_rect`.
    pub fn release(self, view: &mut CanvasView, out: bool, size: Size, pixels_per_point: f32) {
        let Some(corner) = self.corner else { return };
        let rect = ScreenRect::from_two_pos(self.anchor, corner);
        if rect.width() < Self::CLICK || rect.height() < Self::CLICK {
            return;
        }
        let current = device_zoom(view, pixels_per_point);
        if out {
            let zoom = ZoomLadder::step(current, -1, size);
            anchor_view(view, self.anchor, self.doc, zoom, pixels_per_point);
            return;
        }
        // The ratio of two screen lengths scales the zoom in any unit.
        let k = f64::from((view.rect.width() / rect.width()).min(view.rect.height() / rect.height()));
        let zoom = ZoomLadder::clamp(current * k, size);
        let target = view.to_doc(rect.center());
        anchor_view(view, view.rect.center(), target, zoom, pixels_per_point);
    }

    /// The zoom rectangle on screen while it is drawn (Scrubby Zoom off).
    pub fn rect(&self) -> Option<ScreenRect> {
        self.corner.map(|c| ScreenRect::from_two_pos(self.anchor, c))
    }

    /// Draw the zoom rectangle: the marquee's ants look (`marquee.*` tokens).
    // pc: crates/ui-egui/src/tools/zoom_tool.rs:105 `draw`.
    pub fn paint(&self, painter: &Painter, _tokens: &UiTokens) {
        let Some(rect) = self.rect() else { return };
        let rect = ScreenRect::from_min_max(rect.min.round() + egui::vec2(0.5, 0.5), rect.max.round() + egui::vec2(0.5, 0.5));
        // A view that draws document points where they are, so the ants run on the screen rectangle.
        let identity = CanvasView::new(
            painter.clip_rect(),
            Point::new(f64::from(painter.clip_rect().center().x), f64::from(painter.clip_rect().center().y)),
            1.0,
        );
        let outline = Rect::new(f64::from(rect.min.x), f64::from(rect.min.y), f64::from(rect.max.x), f64::from(rect.max.y));
        let time = painter.ctx().input(|i| i.time);
        Marquee::rect(outline, "").paint(painter, &identity, MarqueeLook::Ants, time);
    }

    /// The scrubby zoom for a horizontal drag of `dx` points from a press at `zoom0`, within the
    /// document's range.
    // pc: crates/ui-egui/src/tools/zoom_tool.rs:36 `scrub_zoom`.
    pub fn scrub_zoom(zoom0: f64, dx: f32, size: Size) -> f64 {
        // ±40 doublings already span far more than the zoom range; the bound keeps the power finite.
        let doublings = if dx.is_finite() { f64::from(dx / Self::DOUBLING).clamp(-40.0, 40.0) } else { 0.0 };
        ZoomLadder::clamp(zoom0 * doublings.exp2(), size)
    }
}

/// Zoom Resizes Windows (Preferences › General, and the Zoom tool's Resize Windows to Fit: one
/// setting). With it on, zooming a document that floats in its own window resizes the window to
/// wrap the image at the new zoom, no larger than its screen less [`SCREEN_MARGIN`](Self::SCREEN_MARGIN)
/// and no smaller than [`MIN_SIZE`](Self::MIN_SIZE). Tabbed documents, maximised and full-screen
/// windows keep their size. Only a zoom change resizes: Fit on Screen derives the zoom from the
/// window, so the two never chase each other.
///
/// The app sends the size through `craft-window` (`ViewportCommand::InnerSize`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomWindowFit {
    /// The window's size around the canvas (rulers, scrollbars, title, status bar), in points.
    pub chrome: ScreenVec,
    /// The work area of the window's screen, in points.
    pub screen: ScreenRect,
}

impl ZoomWindowFit {
    /// The window size the image and chrome need, and the size [`wrapped_size`](Self::wrapped_size)
    /// asks for (at least [`MIN_SIZE`](Self::MIN_SIZE), at most the screen less the margin).
    fn wrap(&self, view: &CanvasView, size: Size) -> Option<(ScreenVec, ScreenVec)> {
        let usable = |x: f64| x.is_finite() && x > 0.0;
        let (w, h, sin, cos) = shown_extent(view, size);
        let (w, h) = (w * view.zoom, h * view.zoom);
        if !usable(view.zoom) || !usable(w) || !usable(h) {
            return None;
        }
        let image = egui::vec2((w * cos + h * sin) as f32, (w * sin + h * cos) as f32);
        let chrome = if self.chrome.is_finite() { self.chrome.max(ScreenVec::ZERO) } else { ScreenVec::ZERO };
        let needed = image + chrome;
        // The screen is the hard cap: applied last, it wins over the minimum on a tiny screen.
        let cap = (self.screen.size() - ScreenVec::splat(Self::SCREEN_MARGIN)).max(ScreenVec::splat(1.0));
        Some((needed, needed.max(Self::MIN_SIZE).min(cap)))
    }

    /// The least window inner size.
    pub const MIN_SIZE: ScreenVec = egui::vec2(240.0, 160.0);
    /// Points kept free between a wrapped window and the screen's edges.
    pub const SCREEN_MARGIN: f32 = 48.0;

    /// The window inner size that wraps a document of `size` shown by `view` (its bounding box
    /// when turned), or `None` when there is nothing to wrap.
    // pc: crates/ui-egui/src/canvas/zoom_window_fit.rs:48 `wrapped_size`.
    pub fn wrapped_size(&self, view: &CanvasView, size: Size) -> Option<ScreenVec> {
        self.wrap(view, size).map(|(_, wrapped)| wrapped)
    }

    /// The new size for a zoom from `before` to `after` with the preference `on`, or `None`
    /// when the window keeps its size (preference off, zoom unchanged, nothing to wrap). Every
    /// call records the zoom, so the first call for a window `id` asks for nothing. The app
    /// passes `on = false` on a frame that fits or fills, which is how a fit never resizes.
    // pc: crates/ui-egui/src/canvas/zoom_window_fit.rs:109 `resize_for`.
    pub fn resize_for(&self, ctx: &Context, id: Id, on: bool, before: &CanvasView, after: &CanvasView, size: Size) -> Option<ScreenVec> {
        let key = id.with("zoom-window-fit");
        let last = ctx.data_mut(|d| {
            let last = d.get_temp::<f64>(key);
            d.insert_temp(key, after.zoom);
            last
        });
        // Bits: any change of zoom counts, and a NaN never matches itself into a resize loop.
        let changed = last?.to_bits() != after.zoom.to_bits() && before.zoom.to_bits() != after.zoom.to_bits();
        if !on || !changed {
            return None;
        }
        self.wrapped_size(after, size)
    }

    /// The view centre that shows the whole image centred in a window grown to wrap it (a zoom
    /// about the pointer left it off centre); a window the screen caps keeps the zoomed spot.
    // pc: crates/ui-egui/src/canvas/zoom_window_fit.rs:79 `wrapped_center`.
    pub fn wrapped_center(&self, view: &CanvasView, size: Size) -> Option<Point> {
        let (needed, wrapped) = self.wrap(view, size)?;
        // Capped by the screen and not fitting: the zoomed spot stays.
        if needed.x > wrapped.x + 0.5 || needed.y > wrapped.y + 0.5 {
            return None;
        }
        let center = Point::new(size.width / 2.0, size.height / 2.0);
        (center.x.is_finite() && center.y.is_finite()).then_some(center)
    }
}
