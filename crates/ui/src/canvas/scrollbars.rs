//! [`CanvasScrollbars`]: the scroll bars of a canvas, as Photoshop's document window has them: a
//! horizontal bar along the bottom of the view and a vertical one along the right, always shown.
//! The thumb is the visible fraction of the scrollable extent (the document, plus wherever the
//! view has been scrolled past it); drag it to scroll, or click the track to page by one screenful.
//! With overscroll on, a document smaller than the window can also be scrolled from one edge of
//! the window to the other.
//!
//! With overscroll off the host clamps the view before it draws ([`CanvasScrollbars::clamp`]): the
//! document can't be scrolled past its edges, and one smaller than the window stays inside it.
//!
//! The bars measure the view along the screen axes, so on a rotated view they show the extent of
//! the unrotated document, as PhotoCraft's do.

use egui::accesskit::{self, Role};
use egui::{AsIdSalt, Color32, CornerRadius, CursorIcon, IdSalt, Pos2, Rect as ScreenRect, Sense, Ui, WidgetInfo, pos2, vec2};
use kurbo::{Point, Rect};

use super::CanvasView;
use crate::primitives::Divider;
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// One axis of the view, in document units counted from the document's start on that axis.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Span {
    /// Document length on this axis.
    len: f32,
    /// First and last visible coordinate.
    v0: f32,
    v1: f32,
}

impl Span {
    /// The axis of a view: `size` screen points at `zoom` around `center`.
    fn of(len: f32, center: f32, size: f32, zoom: f32) -> Option<Span> {
        let zoom = if zoom.is_finite() && zoom > 0.0 { zoom } else { return None };
        let half = size / zoom / 2.0;
        let s = Span { len, v0: center - half, v1: center + half };
        (s.len.is_finite() && s.len > 0.0 && s.v0.is_finite() && s.v1.is_finite() && s.v1 > s.v0).then_some(s)
    }

    /// The scrollable extent: the document and the visible span. With `overscroll`, a document
    /// shorter than the span may slide edge to edge within it.
    fn extent(&self, overscroll: bool) -> (f32, f32) {
        let (lo, hi) = (self.v0.min(0.0), self.v1.max(self.len));
        if !overscroll {
            return (lo, hi);
        }
        let visible = self.v1 - self.v0;
        (lo.min(self.len - visible), hi.max(visible))
    }

    /// The thumb's start and length along a track of `track` points, for extent `e`; never shorter
    /// than `min`.
    fn thumb(&self, e: (f32, f32), track: f32, min: f32) -> (f32, f32) {
        let total = (e.1 - e.0).max(1e-6);
        let visible = (self.v1 - self.v0).min(total);
        let len = (visible / total * track).clamp(min.min(track), track.max(0.0));
        let travel = (track - len).max(0.0);
        let room = total - visible;
        let at = if room > 1e-6 { ((self.v0 - e.0) / room).clamp(0.0, 1.0) * travel } else { 0.0 };
        (at, len)
    }

    /// Document units per point of thumb movement along `track`, for extent `e`.
    fn doc_per_point(&self, e: (f32, f32), track: f32, min: f32) -> f32 {
        let (_, len) = self.thumb(e, track, min);
        let travel = track - len;
        let room = (e.1 - e.0) - (self.v1 - self.v0);
        if travel > 1e-3 && room > 0.0 { room / travel } else { 0.0 }
    }
}

/// Clamp one axis of the view centre with overscroll off, as Photoshop 25.4 does: a document
/// larger than the view can't leave its edges, and one smaller than the view stays inside it,
/// where zooming put it (zooming out around the pointer doesn't re-centre it; Fit on Screen
/// does).
fn clamp_axis(center: f32, len: f32, size: f32, zoom: f32) -> f32 {
    if !(zoom.is_finite() && zoom > 0.0 && len.is_finite() && len > 0.0 && size.is_finite() && size > 0.0) {
        return center;
    }
    if !center.is_finite() {
        return len / 2.0;
    }
    let half = size / zoom / 2.0;
    // Larger: the view stays within the document. Smaller: the document stays within the view.
    let (a, b) = (half, len - half);
    center.clamp(a.min(b), a.max(b))
}

/// A thumb drag in progress: the extent and centre when it began (the extent grows as the view
/// moves past the document, so it is frozen for the drag).
#[derive(Clone, Copy, Debug)]
struct Drag {
    extent: (f32, f32),
    center: f32,
    origin: f32,
}

/// The two bars' rectangles inside `rect` (bottom, right), given which are shown.
fn bar_rects(rect: ScreenRect, width: f32, horizontal: bool, vertical: bool) -> (Option<ScreenRect>, Option<ScreenRect>) {
    let right = if vertical { rect.right() - width } else { rect.right() };
    let bottom = if horizontal { rect.bottom() - width } else { rect.bottom() };
    let h = horizontal.then(|| ScreenRect::from_min_max(pos2(rect.left(), bottom), pos2(right, rect.bottom())));
    let v = vertical.then(|| ScreenRect::from_min_max(pos2(right, rect.top()), pos2(rect.right(), bottom)));
    (h, v)
}

/// The AccessKit names of the bars, the host's translated text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScrollbarNames {
    /// The bar along the bottom ("Horizontal scroll bar").
    pub horizontal: String,
    /// The bar along the right ("Vertical scroll bar").
    pub vertical: String,
}

impl ScrollbarNames {
    pub fn new(horizontal: impl Into<String>, vertical: impl Into<String>) -> Self {
        Self { horizontal: horizontal.into(), vertical: vertical.into() }
    }
}

/// The bars' look, from [`CanvasScrollbars::style`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScrollbarStyle {
    /// Thickness of a bar and side of the corner square: `scroll.canvas_width`.
    pub width: f32,
    /// Between a bar's edge and its thumb: `scroll.thumb_inset`.
    pub inset: f32,
    /// Shortest thumb: `scroll.thumb_min`.
    pub thumb_min: f32,
    /// The tracks and the corner: `surface.chrome`.
    pub track: Color32,
    /// The thumb at rest: `text.faint`.
    pub thumb: Color32,
    /// The thumb while the pointer is within `inset` of it or it is dragged: `text.muted`.
    pub thumb_hot: Color32,
}

/// What happened to the bars in a pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use = "repaint when the view moved, and leave the pointer alone while it is over a bar"]
pub struct ScrollbarsResponse {
    /// The pointer is over a bar or dragging one: the canvas leaves it alone.
    pub over: bool,
    /// The view's centre moved.
    pub changed: bool,
}

/// The scroll bars of a canvas. They sit inside the canvas rect, over its bottom and right
/// edges, and sense a press before the canvas below does; the host reads
/// [`ScrollbarsResponse::over`] to keep its tool from reacting to the same press. The bars are
/// AccessKit scroll bars named by [`ScrollbarNames`] and no Tab stops: the host scrolls from the
/// keyboard as it does without bars.
pub struct CanvasScrollbars {
    id_salt: IdSalt,
    names: ScrollbarNames,
    overscroll: bool,
}

impl CanvasScrollbars {
    /// Scroll bars named `names` in the AccessKit tree.
    pub fn new(id_salt: impl AsIdSalt, names: ScrollbarNames) -> Self {
        Self { id_salt: IdSalt::new(id_salt), names, overscroll: false }
    }

    /// Whether a document that fits in the window can still be scrolled edge to edge within it
    /// (Preferences › Tools › Overscroll). Off by default.
    pub fn overscroll(mut self, overscroll: bool) -> Self {
        self.overscroll = overscroll;
        self
    }

    /// The bars' look in `tokens`.
    pub fn style(tokens: &UiTokens) -> ScrollbarStyle {
        let (c, s) = (&tokens.color, &tokens.component.scroll);
        ScrollbarStyle {
            width: s.canvas_width,
            inset: s.thumb_inset,
            thumb_min: s.thumb_min,
            track: c.surface.chrome.to_egui(),
            thumb: c.text.faint.to_egui(),
            thumb_hot: c.text.muted.to_egui(),
        }
    }

    /// Applies overscroll off to `view` over the document `doc`: the view can't leave the
    /// document's edges, and a document smaller than the view stays inside it. True when the view
    /// moved (then repaint).
    pub fn clamp(view: &mut CanvasView, doc: Rect) -> bool {
        if !(doc.width() > 0.0 && doc.height() > 0.0) || !view.zoom.is_finite() || view.zoom <= 0.0 {
            return false;
        }
        let size = view.rect.size();
        let aspect = if view.aspect.is_finite() && view.aspect > 0.0 { view.aspect } else { 1.0 };
        let (w, h) = (doc.width() as f32, doc.height() as f32);
        let zoom = view.zoom as f32;
        let from = Point::new(doc.x0, doc.y0);
        let local = if view.rotation.abs() < 1e-8 {
            [
                clamp_axis(view.center.x as f32 - from.x as f32, w, size.x / aspect as f32, zoom),
                clamp_axis(view.center.y as f32 - from.y as f32, h, size.y, zoom),
            ]
        } else {
            if !size.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
                return false;
            }
            // The displayed document's bounds against the view's: the offset that puts the document
            // inside the view (or the view inside it) is the same clamp, on screen.
            let level = CanvasView { flip_x: false, ..*view };
            let corners = doc_corners(doc).map(|p| level.to_screen(p));
            let image =
                corners.iter().skip(1).fold(ScreenRect::from_min_max(corners[0], corners[0]), |r, &p| r.union(ScreenRect::from_min_max(p, p)));
            if !image.is_finite() {
                return false;
            }
            let offset = view.rect.center() - image.min;
            let wanted = vec2(clamp_axis(offset.x, image.width(), size.x, 1.0), clamp_axis(offset.y, image.height(), size.y, 1.0)) - offset;
            let d = level.unmap_vec(wanted);
            [(view.center.x + d.x - from.x) as f32, (view.center.y + d.y - from.y) as f32]
        };
        let center = Point::new(f64::from(local[0]) + doc.x0, f64::from(local[1]) + doc.y0);
        let moved = !view.center.x.is_finite()
            || !view.center.y.is_finite()
            || (center.x - view.center.x).abs() > 1e-3
            || (center.y - view.center.y).abs() > 1e-3;
        if moved {
            view.center = center;
        }
        moved
    }

    /// Draws the bars over the bottom and right edges of `view.rect` and applies their drags and
    /// page clicks to `view.center`. `doc` is the document's bounds in document units.
    pub fn show(self, ui: &Ui, view: &mut CanvasView, doc: Rect) -> ScrollbarsResponse {
        let mut out = ScrollbarsResponse { over: false, changed: false };
        let rect = view.rect;
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let zoom = view.zoom as f32;
        let aspect = if view.aspect.is_finite() && view.aspect > 0.0 { view.aspect as f32 } else { 1.0 };
        let (w, h) = (doc.width() as f32, doc.height() as f32);
        let (x0, y0) = (doc.x0 as f32, doc.y0 as f32);
        let flip = view.flip_x;
        // The horizontal axis in screen order: with the view flipped, run the document backwards.
        let hx = |c: f32| if flip { w - c } else { c };
        let with_extent = |s: Span| (s, s.extent(self.overscroll));
        let ex = Span::of(w, hx(view.center.x as f32 - x0), rect.width() / aspect, zoom).map(with_extent);
        let ey = Span::of(h, view.center.y as f32 - y0, rect.height(), zoom).map(with_extent);
        if ex.is_none() && ey.is_none() {
            return out;
        }
        let (hr, vr) = bar_rects(rect, style.width, ex.is_some(), ey.is_some());
        let painter = ui.painter_at(rect);
        let id = ui.make_persistent_id(self.id_salt);
        let enabled = ui.is_enabled();
        let ppp = ui.pixels_per_point();
        let half = 0.5 / ppp;
        if let (Some(r), Some(_)) = (hr, vr) {
            painter.rect_filled(ScreenRect::from_min_max(pos2(r.right(), r.top()), rect.max), 0.0, style.track);
        }
        for (axis, bar, span) in [(0usize, hr, ex), (1usize, vr, ey)] {
            let (Some(bar), Some((s, e))) = (bar, span) else { continue };
            let bar_id = id.with(("scrollbar", axis));
            let resp = ui.interact(bar, bar_id, Sense::click_and_drag() - Sense::FOCUSABLE);
            out.over |= resp.hovered() || resp.dragged();
            let along = |p: Pos2| if axis == 0 { p.x - bar.left() } else { p.y - bar.top() };
            let track = if axis == 0 { bar.width() } else { bar.height() };
            let dragging = ui.data(|d| d.get_temp::<Drag>(bar_id));
            // A drag keeps the extent it began with.
            let e = dragging.map_or(e, |d| d.extent);
            let (t0, tl) = s.thumb(e, track, style.thumb_min);
            let set = |view: &mut CanvasView, c: f32, changed: &mut bool| {
                if !c.is_finite() {
                    return;
                }
                let before = view.center;
                if axis == 0 {
                    view.center.x = f64::from(hx(c) + x0);
                } else {
                    view.center.y = f64::from(c + y0);
                }
                *changed |= view.center != before;
            };
            let center = (s.v0 + s.v1) / 2.0;
            // The drag starts where the button went down, not where it passed egui's drag threshold.
            if resp.drag_started()
                && let Some(p) = ui.input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos())
            {
                let a = along(p);
                if a >= t0 && a <= t0 + tl {
                    ui.data_mut(|d| d.insert_temp(bar_id, Drag { extent: e, center, origin: a }));
                }
            }
            if let Some(d) = ui.data(|m| m.get_temp::<Drag>(bar_id)) {
                if let Some(p) = resp.interact_pointer_pos().filter(|_| resp.dragged()) {
                    let c = d.center + (along(p) - d.origin) * s.doc_per_point(d.extent, track, style.thumb_min);
                    set(view, c, &mut out.changed);
                }
                if !resp.dragged() || resp.drag_stopped() {
                    ui.data_mut(|m| m.remove::<Drag>(bar_id));
                }
            } else if resp.clicked()
                && let Some(p) = resp.interact_pointer_pos()
            {
                // Page by one screenful toward the click, stopping at the end of the extent.
                let a = along(p);
                let page = s.v1 - s.v0;
                let c = if a < t0 {
                    (center - page).max(e.0 + page / 2.0)
                } else if a > t0 + tl {
                    (center + page).min(e.1 - page / 2.0)
                } else {
                    center
                };
                set(view, c, &mut out.changed);
            }
            // Track and thumb.
            painter.rect_filled(bar, 0.0, style.track);
            let edge = if axis == 0 { [bar.left_top(), bar.right_top()] } else { [bar.left_top(), bar.left_bottom()] };
            let inward = if axis == 0 { vec2(0.0, half) } else { vec2(half, 0.0) };
            Divider::paint(&painter, edge[0] + inward, edge[1] + inward, &tokens);
            let thumb = if axis == 0 {
                ScreenRect::from_min_size(pos2(bar.left() + t0, bar.top()), vec2(tl, bar.height()))
            } else {
                ScreenRect::from_min_size(pos2(bar.left(), bar.top() + t0), vec2(bar.width(), tl))
            }
            .shrink(style.inset);
            let hot = resp.dragged() || resp.hover_pos().is_some_and(|p| thumb.expand(style.inset).contains(p));
            let fill = if hot { style.thumb_hot } else { style.thumb };
            // Fully round: the radius is half the thumb's thickness.
            let radius = (thumb.width().min(thumb.height()) / 2.0).clamp(0.0, 255.0) as u8;
            painter.rect_filled(thumb, CornerRadius::same(radius), fill);
            if resp.hovered() || resp.dragged() {
                ui.set_cursor_icon(CursorIcon::Default);
            }
            let (name, orientation) = if axis == 0 {
                (&self.names.horizontal, accesskit::Orientation::Horizontal)
            } else {
                (&self.names.vertical, accesskit::Orientation::Vertical)
            };
            resp.widget_info(|| WidgetInfo::labeled(Role::ScrollBar, enabled, name));
            let room = f64::from(((e.1 - e.0) - (s.v1 - s.v0)).max(0.0));
            let position = f64::from((s.v0 - e.0).max(0.0)).min(room);
            ui.ctx().accesskit_node_builder(resp.id, |node| {
                node.set_orientation(orientation);
                node.set_min_numeric_value(0.0);
                node.set_max_numeric_value(room);
                node.set_numeric_value(position);
            });
        }
        out
    }
}

/// The four corners of a document rectangle.
fn doc_corners(doc: Rect) -> [Point; 4] {
    [Point::new(doc.x0, doc.y0), Point::new(doc.x1, doc.y0), Point::new(doc.x1, doc.y1), Point::new(doc.x0, doc.y1)]
}

#[cfg(test)]
#[path = "scrollbars_tests.rs"]
mod tests;
