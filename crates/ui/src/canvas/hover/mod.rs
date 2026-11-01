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

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

mod lookup;
mod style;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use egui::{AsIdSalt, Color32, IdSalt, Ui};
use kurbo::{BezPath, Rect};

use super::CanvasView;

pub use lookup::{HoverAnswer, HoverHint, HoverLookup, HoverQuery, HoverUpdate};
pub use style::HoverStyle;

/// What a click under the pointer would pick, in document units.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum HoverShape {
    /// A layer's or group's bounds: a `hover.outline` line just outside the rectangle (the line
    /// sits outside, so it never covers the layer's edge pixels).
    Rect(Rect),
    /// An object's outline: a `hover.outline` line along the path.
    Path(BezPath),
    /// An object's coverage: a tint of the outline colour at `hover.fill_alpha` × coverage, with
    /// a one-texel edge at `hover.edge_alpha`.
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
        // ceil(max(w, h) / TEX_MAX), samples at bbox.x0 + tx * step; TEX_MAX 768 at :28). The colour
        // and the edge test move to `HoverOutline::show`; a global counter gives `generation`.
        todo!()
    }

    /// The mask's extent in document units.
    pub fn bounds(&self) -> Rect {
        // new: the field.
        todo!()
    }

    /// Texels across and down.
    pub fn size(&self) -> [usize; 2] {
        // new: the field.
        todo!()
    }

    /// The coverage of texel `(x, y)`, 0..=1; 0 outside the mask.
    pub fn coverage(&self, x: usize, y: usize) -> f32 {
        // new: coverage[y * width + x] / 255.
        todo!()
    }

    /// Whether texel `(x, y)` is on the mask's edge: covered at 0.5 or more, and on the texture's
    /// border or beside a texel (left, right, above, below) covered less than 0.5.
    pub fn is_edge(&self, x: usize, y: usize) -> bool {
        // pc: crates/ui-egui/src/tools/object_finder.rs:180-190 (the four-neighbour edge test).
        todo!()
    }
}

/// The outline of the object under the pointer: decoration over the canvas, drawn after the
/// document and before the tool's own marks.
///
/// - No shape draws nothing; the host passes `None` while a button is down, another tool is up,
///   the pointer is off the canvas or the option is off (PhotoCraft's Auto-Select rollover, Object
///   Finder; VectorCraft's Object Highlighting).
/// - Colour: [`colour`](Self::colour) (VectorCraft and DesignCraft: the object's layer colour),
///   else `overlay.line` (the accent as a bare line, PhotoCraft's).
/// - It never takes the pointer or the keyboard and adds nothing to the AccessKit tree; the
///   lookup's hint is the host's to announce through its status line.
/// - Nothing mirrors in a right-to-left interface: the shape is the document's.
pub struct HoverOutline {
    id_salt: IdSalt,
    shape: Option<HoverShape>,
    colour: Option<Color32>,
}

impl HoverOutline {
    /// The outline of `shape`; `id_salt` keys a mask's texture.
    pub fn new(id_salt: impl AsIdSalt, shape: Option<HoverShape>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), shape, colour: None }
    }

    /// The outline's colour (a layer colour); `overlay.line` by default.
    pub fn colour(mut self, colour: Color32) -> Self {
        self.colour = Some(colour);
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
        // The sizes and alphas come from `HoverOutline::style`.
        todo!()
    }
}
