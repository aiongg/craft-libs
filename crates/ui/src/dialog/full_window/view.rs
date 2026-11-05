//! [`FullWindowView`]: the view of a full-window canvas as plain data, and
//! [`FullWindowZoomKeys`], the zoom keys that act on it.

use egui::{Key, KeyboardShortcut, Modifiers, Rect, Ui};
use kurbo::{Point, Size};

use crate::canvas::{CanvasView, ZoomLadder};

/// The most a fitted view magnifies: 400 %.
const FIT_MAX: f64 = 4.0;
/// Zooms and centres closer than this are the same view (the round trip through screen points
/// per unit and back differs in the last bits).
const SAME: f64 = 1e-9;

/// How a full-window canvas shows its image: fitted, or a zoom and a centre the user chose.
///
/// Plain data the workspace keeps beside its session, never in its
/// [`LocalHistory`](crate::dialog::LocalHistory): Undo never moves the view. A new view is fitted.
/// [`FullWindowCanvas`](super::FullWindowCanvas) refits it every pass while fitted (so a window
/// resize refits), and writes back the zoom it fitted to and the image's size, so
/// [`zoom_percent`](Self::zoom_percent) is the zoom on screen either way and the
/// [`FullWindowZoomBar`](super::FullWindowZoomBar) steps from it.
///
/// - Fitted: the whole image shows centred, with `full_window.canvas.margin` on each side, at no
///   more than 400 %.
/// - A zoom (wheel, pinch, keys, the Zoom tool, the zoom bar) or a pan (Space, the middle button,
///   the Hand tool) by the user ends the fit; [`reset_fit`](Self::reset_fit) (⌘0, the zoom bar's
///   Fit in View) returns to it.
/// - Zooms are device zooms, 1 = 100 % (one image pixel per device pixel), kept inside
///   [`ZoomLadder`](crate::canvas::ZoomLadder)'s range for the image's size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FullWindowView {
    /// The view fits the image until the user zooms or pans.
    fit: bool,
    /// Device zoom; while fitted, the zoom the last pass fitted to.
    zoom: f64,
    /// The image point at the canvas centre, in image pixels.
    center: Point,
    /// The image's size in pixels as the last pass saw it; `[0, 0]` before the first.
    size: [u32; 2],
}

impl Default for FullWindowView {
    fn default() -> Self {
        Self::new()
    }
}

impl FullWindowView {
    /// A fitted view of an image not yet shown.
    pub fn new() -> Self {
        Self { fit: true, zoom: 1.0, center: Point::ZERO, size: [0, 0] }
    }

    /// Whether the view fits the image: until the user zooms or pans, and after
    /// [`reset_fit`](Self::reset_fit).
    pub fn is_fit(&self) -> bool {
        self.fit
    }

    /// The zoom on screen as a percentage (66.7 for 66.7 %), the fitted zoom while fitted: the
    /// title strip's and the zoom bar's value.
    pub fn zoom_percent(&self) -> f64 {
        self.zoom * 100.0
    }

    /// The device zoom on screen, 1 = 100 %.
    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    /// The image point at the canvas centre, in image pixels.
    pub fn center(&self) -> Point {
        self.center
    }

    /// Sets the device zoom (clamped to [`ZoomLadder`](crate::canvas::ZoomLadder)'s range for the
    /// image's size, once it is known), keeping the centre, and ends the fit.
    pub fn set_zoom(&mut self, zoom: f64) {
        // new: PhotoCraft keeps `zoom <= 0.0` as "fit on the next frame" and clamps to 0.01..32
        // (pc: crates/ui-egui/src/dialogs/liquify_ui.rs:1232-1233); the library's ladder range
        // replaces the clamp (step7-workspaces.md lead call 16).
        self.zoom = ZoomLadder::clamp(zoom, self.doc_size());
        self.fit = false;
    }

    /// Steps the zoom one rung of [`ZoomLadder::STEPS`](crate::canvas::ZoomLadder::STEPS) in
    /// (`dir > 0`) or out, from the zoom on screen, and ends the fit. False at the end of the
    /// range (nothing changes).
    pub fn step(&mut self, dir: i32) -> bool {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:747-748 (`step_in_range` from the zoom on
        // screen), with `ZoomLadder::step` for the image's size in place of the dialog's range.
        let next = ZoomLadder::step(self.zoom, dir, self.doc_size());
        let changed = next.is_finite() && (next / self.zoom - 1.0).abs() > SAME;
        if changed {
            self.zoom = next;
            self.fit = false;
        }
        changed
    }

    /// The image's size as the ladder takes it.
    fn doc_size(&self) -> Size {
        Size::new(f64::from(self.size[0]), f64::from(self.size[1]))
    }

    /// Fits the image again (⌘0, Fit in View).
    pub fn reset_fit(&mut self) {
        self.fit = true;
    }

    /// The [`CanvasView`] of this pass on `rect` for an image of `size` pixels: fitted while
    /// [`is_fit`](Self::is_fit) (`margin` on each side, centred, at most 4 = 400 %, at least the
    /// ladder's minimum), else the zoom and centre. Records `size` and, while fitted, the zoom it
    /// fitted to. The view's zoom is in screen points per image pixel
    /// ([`CanvasView::points_per_unit`] of the device zoom at `pixels_per_point`).
    pub fn canvas_view(&mut self, rect: Rect, size: [u32; 2], margin: f32, pixels_per_point: f32) -> CanvasView {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1914-1915 (fit: `(area - 40) / size`,
        // clamped to 0.01..4), liquify_ui.rs:1085-1086 (the same).
        self.size = size;
        if self.fit && size[0] > 0 && size[1] > 0 {
            let doc = self.doc_size();
            let room = |side: f32| f64::from((side - 2.0 * margin).max(1.0));
            let points = (room(rect.width()) / doc.width).min(room(rect.height()) / doc.height);
            self.zoom = (points * f64::from(pixels_per_point)).min(FIT_MAX).max(ZoomLadder::min(doc));
            self.center = Point::new(doc.width / 2.0, doc.height / 2.0);
        }
        CanvasView::new(rect, self.center, CanvasView::points_per_unit(self.zoom, pixels_per_point))
    }

    /// Takes the zoom and centre of `view` (after a tool, a pan or a wheel zoom changed it on the
    /// canvas this pass); a change of either ends the fit.
    pub fn take(&mut self, view: &CanvasView, pixels_per_point: f32) {
        // new: the canvas hands the host's changed `CanvasView` back to the plain data.
        let zoom = view.zoom * f64::from(pixels_per_point);
        let moved = (zoom / self.zoom - 1.0).abs() > SAME || (view.center - self.center).hypot() > SAME;
        if moved {
            self.zoom = zoom;
            self.center = view.center;
            self.fit = false;
        }
    }

    /// Applies a zoom command: [`In`](FullWindowZoom::In) and [`Out`](FullWindowZoom::Out) step
    /// the ladder, [`Fit`](FullWindowZoom::Fit) fits, [`ActualPixels`](FullWindowZoom::ActualPixels)
    /// sets 100 % about the centre. False when nothing changed.
    pub fn apply(&mut self, zoom: FullWindowZoom) -> bool {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:746-752 (zoom in, out, fit as `0.0`, else
        // 100 %).
        match zoom {
            FullWindowZoom::In => self.step(1),
            FullWindowZoom::Out => self.step(-1),
            FullWindowZoom::Fit => {
                let changed = !self.fit;
                self.reset_fit();
                changed
            }
            FullWindowZoom::ActualPixels => {
                let changed = self.fit || (self.zoom - 1.0).abs() > SAME;
                self.set_zoom(1.0);
                changed
            }
        }
    }
}

/// A zoom command of a full-window canvas: its keys and the zoom bar's buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FullWindowZoom {
    /// Zoom in.
    In,
    /// Zoom out.
    Out,
    /// Fit in View.
    Fit,
    /// 100 %.
    ActualPixels,
}

/// The zoom keys of a full-window canvas. They act on the workspace's view and are consumed,
/// so they never reach the canvas behind the dialog: ⌘+ (and ⌘=), ⌘−, ⌘0 (Fit), ⌘1 (100 %) by
/// default, as the main window's View menu. An app whose user rebound those commands passes its
/// own chords (PhotoCraft's `nav_shortcut`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullWindowZoomKeys {
    /// Chords that zoom in.
    pub zoom_in: Vec<KeyboardShortcut>,
    /// Chords that zoom out.
    pub zoom_out: Vec<KeyboardShortcut>,
    /// Chords for Fit in View.
    pub fit: Vec<KeyboardShortcut>,
    /// Chords for 100 %.
    pub actual_pixels: Vec<KeyboardShortcut>,
}

impl Default for FullWindowZoomKeys {
    fn default() -> Self {
        let command = |key| KeyboardShortcut::new(Modifiers::COMMAND, key);
        Self {
            zoom_in: vec![command(Key::Plus), command(Key::Equals)],
            zoom_out: vec![command(Key::Minus)],
            fit: vec![command(Key::Num0)],
            actual_pixels: vec![command(Key::Num1)],
        }
    }
}

impl FullWindowZoomKeys {
    /// The first of these chords pressed this pass, consumed. `None` unless the keys are free for
    /// `ui`'s layer ([`keys_free`](crate::focus::key_owner::keys_free)): a text field keeps ⌘+ and
    /// ⌘− for its own text, a dialog the workspace opened above it has them, and so does an open
    /// popup, menu or popover.
    pub fn take(&self, ui: &Ui) -> Option<FullWindowZoom> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:742-743 ("⌘+ / ⌘− / ⌘0 / ⌘1 (or their
        // overrides) zoom the preview, not the canvas behind it").
        let ctx = ui.ctx();
        if !crate::focus::key_owner::keys_free(ctx, ui.layer_id()) {
            return None;
        }
        let groups = [
            (&self.zoom_in, FullWindowZoom::In),
            (&self.zoom_out, FullWindowZoom::Out),
            (&self.fit, FullWindowZoom::Fit),
            (&self.actual_pixels, FullWindowZoom::ActualPixels),
        ];
        groups.into_iter().find_map(|(chords, zoom)| chords.iter().any(|chord| ctx.input_mut(|i| i.consume_shortcut(chord))).then_some(zoom))
    }
}
