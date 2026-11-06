//! The outline of the object under the pointer: what a click there would pick.
//!
//! - [`HoverOutline`]: draws a [`HoverShape`] over the canvas: a layer's bounds (PhotoCraft's Move
//!   tool rollover), an object's path in its layer colour (VectorCraft's Object Highlighting, a
//!   pen segment), or an object's coverage mask as a tint (PhotoCraft's Object Finder).
//! - [`HoverLookup`]: the debounced, cancellable background lookup that finds the object under a
//!   resting pointer, with a cache of misses and the empty-layer hint.
//!
//! Neither changes the document nor takes the keyboard: the outline is decoration, and the hint
//! is the app's to show through its status line.
//!
//! ```no_run
//! use craft_ui::canvas::CanvasView;
//! use craft_ui::canvas::hover::{HoverOutline, HoverShape};
//! use craft_ui::canvas::kurbo::Rect;
//! # fn demo(ui: &egui::Ui, view: &CanvasView, layer: Option<Rect>) {
//! // The Move tool's rollover: the bounds of the layer a click would pick.
//! HoverOutline::new("rollover", layer.map(HoverShape::Rect)).show(ui, view);
//! # }
//! ```

mod lookup;
mod style;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use std::sync::atomic::{AtomicU64, Ordering};

use egui::epaint::{PathShape, PathStroke, Vertex};
use egui::{AsIdSalt, Color32, ColorImage, IdSalt, Mesh, Shape, Stroke, TextureHandle, TextureOptions, Ui, pos2};
use kurbo::{BezPath, PathEl, Rect};

use super::CanvasView;
use crate::theme::current;

pub use lookup::{HoverAnswer, HoverHint, HoverLookup, HoverQuery, HoverUpdate};
pub use style::HoverStyle;

/// What a click under the pointer would pick, in document units.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum HoverShape {
    /// A layer's or group's bounds: the outline line just outside the rectangle (the line
    /// sits outside, so it never covers the layer's edge pixels).
    Rect(Rect),
    /// An object's outline: the outline line along the path.
    Path(BezPath),
    /// An object's coverage: a tint of the outline colour at the fill alpha × coverage, with
    /// a one-texel edge at the edge alpha.
    Mask(HoverMask),
}

/// An object's coverage reduced for the hover tint: at most [`HoverMask::MAX_TEXELS`] texels on
/// its long side, sampled once when the mask is made. Cheap to clone (the texels are shared), and
/// `Send + Sync`, so a lookup job can make it off the UI thread and the app can keep it with its
/// result. [`HoverOutline`] uploads its texture once per mask and colour.
#[derive(Clone, Debug)]
pub struct HoverMask {
    /// The mask's extent in document units: the texture's quad.
    bounds: Rect,
    /// Texels across and down.
    size: [usize; 2],
    /// Coverage 0..=255 per texel, row by row.
    coverage: Arc<[u8]>,
    /// Unique per mask made, for the texture cache.
    generation: u64,
}

const _: () = {
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<HoverMask>();
};

impl PartialEq for HoverMask {
    /// Two masks are equal when they are the same sampling (clones of one mask).
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation
    }
}

impl HoverMask {
    /// The longest side of the tint texture: the outline is a preview, not the selection
    /// (PhotoCraft's `TEX_MAX`).
    pub const MAX_TEXELS: usize = 768;

    /// The mask over `bounds` (document units): `coverage(x, y)` (0..=1, at a document point) is
    /// read at the top-left corner of each texel's cell, with one texel per document unit until
    /// the long side passes [`MAX_TEXELS`](Self::MAX_TEXELS), then `ceil(long side / 768)` units
    /// per texel. Coverage outside 0..=1 is clamped; a non-finite one reads as 0. Empty or
    /// non-finite `bounds` give a 1 × 1 mask with no coverage.
    pub fn sampled(bounds: Rect, coverage: impl Fn(f64, f64) -> f32) -> HoverMask {
        // pc: crates/ui-egui/src/tools/object_finder.rs:172-195 `overlay_image` (step =
        // ceil(max(w, h) / TEX_MAX), samples at bbox.x0 + tx * step).
        static GENERATIONS: AtomicU64 = AtomicU64::new(0);
        let generation = GENERATIONS.fetch_add(1, Ordering::Relaxed);
        let finite = bounds.x0.is_finite() && bounds.y0.is_finite() && bounds.x1.is_finite() && bounds.y1.is_finite();
        if !finite || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return HoverMask { bounds, size: [1, 1], coverage: Arc::from(vec![0u8]), generation };
        }
        let (w, h) = (bounds.width().ceil().max(1.0), bounds.height().ceil().max(1.0));
        let step = (w.max(h) / Self::MAX_TEXELS as f64).ceil().max(1.0);
        let (tw, th) = ((w / step).ceil() as usize, (h / step).ceil() as usize);
        let mut texels = Vec::with_capacity(tw * th);
        for ty in 0..th {
            for tx in 0..tw {
                let c = coverage(bounds.x0 + tx as f64 * step, bounds.y0 + ty as f64 * step);
                let c = if c.is_finite() { c.clamp(0.0, 1.0) } else { 0.0 };
                texels.push((c * 255.0).round() as u8);
            }
        }
        HoverMask { bounds, size: [tw, th], coverage: Arc::from(texels), generation }
    }

    /// The mask's extent in document units.
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Texels across and down.
    pub fn size(&self) -> [usize; 2] {
        self.size
    }

    /// The coverage of texel `(x, y)`, 0..=1; 0 outside the mask.
    pub fn coverage(&self, x: usize, y: usize) -> f32 {
        let [w, h] = self.size;
        if x >= w || y >= h {
            return 0.0;
        }
        f32::from(self.coverage[y * w + x]) / 255.0
    }

    /// Whether texel `(x, y)` is on the mask's edge: covered at 0.5 or more, and on the texture's
    /// border or beside a texel (left, right, above, below) covered less than 0.5.
    pub fn is_edge(&self, x: usize, y: usize) -> bool {
        // pc: crates/ui-egui/src/tools/object_finder.rs:180-190 (the four-neighbour edge test).
        let [w, h] = self.size;
        let on = |x: usize, y: usize| self.coverage(x, y) >= 0.5;
        on(x, y) && (x == 0 || y == 0 || x + 1 == w || y + 1 == h || !on(x - 1, y) || !on(x + 1, y) || !on(x, y - 1) || !on(x, y + 1))
    }
}

/// The outline of the object under the pointer: decoration over the canvas, drawn after the
/// document and before the tool's own marks.
///
/// - No shape draws nothing; the host passes `None` while a button is down, another tool is up,
///   the pointer is off the canvas or the option is off (PhotoCraft's Auto-Select rollover, Object
///   Finder; VectorCraft's Object Highlighting).
/// - Colour: [`color`](Self::color) (VectorCraft and DesignCraft: the object's layer colour),
///   else `overlay.line` (the accent as a bare line, PhotoCraft's).
/// - It never takes the pointer or the keyboard and adds nothing to the AccessKit tree; the
///   lookup's hint is the host's to announce through its status line.
/// - Nothing mirrors in a right-to-left interface: the shape is the document's.
pub struct HoverOutline {
    id_salt: IdSalt,
    shape: Option<HoverShape>,
    color: Option<Color32>,
}

impl HoverOutline {
    /// The outline of `shape`; `id_salt` keys a mask's texture.
    pub fn new(id_salt: impl AsIdSalt, shape: Option<HoverShape>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), shape, color: None }
    }

    /// The outline's colour (a layer colour); `overlay.line` by default.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    /// Paints the shape over `view`, clipped to the canvas rect. A [`HoverShape::Mask`] is one
    /// textured quad over the mask's bounds (linear filtering), its texture made from the mask and
    /// the colour and kept in the context under the outline's id until another mask or colour
    /// comes.
    pub fn show(self, ui: &Ui, view: &CanvasView) {
        // pc: crates/ui-egui/src/tools/move_hover.rs:128-130 (1.5 pt accent, `StrokeKind::Outside`).
        // pc: crates/ui-egui/src/tools/object_finder.rs:164-168 `overlay` (0.42 × coverage, edge
        // 0.95), :218-233 (texture keyed by the region, one quad mesh through the view).
        // vectorcraft: crates/ui-egui/src/canvas/overlays.rs:20-24 (the path in its layer colour, 1.5).
        let Some(shape) = &self.shape else { return };
        let style = Self::style(&current(ui.ctx()));
        let color = self.color.unwrap_or(style.color);
        let painter = ui.painter().with_clip_rect(view.rect);
        let stroke = Stroke::new(style.outline, color);
        match shape {
            HoverShape::Rect(rect) => {
                // The line sits outside the rectangle: grow it by half the width.
                let half = view.doc_len(style.outline / 2.0);
                let half_x = f64::from(style.outline / 2.0) / view.zoom_x();
                let grown = Rect::new(rect.x0 - half_x, rect.y0 - half, rect.x1 + half_x, rect.y1 + half);
                painter.add(Shape::closed_line(view.quad(grown).to_vec(), stroke));
            }
            HoverShape::Path(path) => {
                let screen = view.affine() * path.clone();
                let mut points = Vec::new();
                let emit = |points: &mut Vec<egui::Pos2>, closed: bool| {
                    if points.len() > 1 {
                        let line = std::mem::take(points);
                        painter.add(if closed {
                            Shape::Path(PathShape { points: line, closed: true, fill: Color32::TRANSPARENT, stroke: PathStroke::from(stroke) })
                        } else {
                            Shape::line(line, stroke)
                        });
                    }
                    points.clear();
                };
                kurbo::flatten(screen.elements().iter().copied(), 0.25, |el| match el {
                    PathEl::MoveTo(p) => {
                        emit(&mut points, false);
                        points.push(pos2(p.x as f32, p.y as f32));
                    }
                    PathEl::LineTo(p) => points.push(pos2(p.x as f32, p.y as f32)),
                    PathEl::ClosePath => emit(&mut points, true),
                    _ => {}
                });
                emit(&mut points, false);
            }
            HoverShape::Mask(mask) => {
                let id = ui.make_persistent_id(self.id_salt);
                let cached = ui.ctx().data(|d| d.get_temp::<(u64, Color32, TextureHandle)>(id));
                let texture = match cached {
                    Some((generation, c, texture)) if generation == mask.generation && c == color => texture,
                    _ => {
                        let texture = ui.ctx().load_texture("craft-hover-mask", mask_image(mask, color, &style), TextureOptions::LINEAR);
                        ui.ctx().data_mut(|d| d.insert_temp(id, (mask.generation, color, texture.clone())));
                        texture
                    }
                };
                let corners = view.quad(mask.bounds);
                let uvs = [pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(1.0, 1.0), pos2(0.0, 1.0)];
                let mut mesh = Mesh::with_texture(texture.id());
                for (pos, uv) in corners.into_iter().zip(uvs) {
                    mesh.vertices.push(Vertex { pos, uv, color: Color32::WHITE });
                }
                mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
                painter.add(Shape::mesh(mesh));
            }
        }
    }
}

/// The tint image of `mask` in `color`: coverage × `fill_alpha`, edge texels at `edge_alpha`.
fn mask_image(mask: &HoverMask, color: Color32, style: &HoverStyle) -> ColorImage {
    let [w, h] = mask.size;
    let mut pixels = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let c = mask.coverage(x, y);
            let alpha = if c <= 0.0 {
                0.0
            } else if mask.is_edge(x, y) {
                style.edge_alpha
            } else {
                style.fill_alpha * c
            };
            pixels.push(Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (alpha.clamp(0.0, 1.0) * 255.0).round() as u8));
        }
    }
    ColorImage::new([w, h], pixels)
}
