//! The text boundary. Shaping and metrics come with the renderer's backend; the runtime keeps
//! each node's layouts and hands them back to the renderer by handle.

use std::any::Any;
use std::sync::Arc;

use craft_theme::{FontSlot, Rgba};
use emath::{Pos2, Rect, Vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontSpec {
    pub slot: FontSlot,
    /// Size in logical points.
    pub size: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextSpec {
    pub text: Arc<str>,
    pub font: FontSpec,
    /// Line height in logical points; the font's own when `None`.
    pub line_height: Option<f32>,
    pub color: Rgba,
    /// The line's room in logical points; with [`Overflow::Wrap`] the width it wraps at. No
    /// limit when `None`.
    pub max_width: Option<f32>,
    pub overflow: Overflow,
}

/// What a text does when it is wider than `max_width`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overflow {
    /// Wraps onto further lines.
    #[default]
    Wrap,
    /// One line, cut after the last character that fits.
    Clip,
    /// One line, cut so that the most characters that fit are followed by "…".
    Ellipsis,
}

const ELLIPSIS: &str = "\u{2026}";

/// The text a one-line `spec` shows: `spec.text` when it fits `max_width` or does not cut, else
/// the longest prefix (whole characters, trailing white space dropped, at most one character
/// short of the whole) that fits, followed by the ellipsis in [`Overflow::Ellipsis`]; the
/// ellipsis alone when no prefix fits. `width` measures a candidate in logical points.
pub fn overflow_text(spec: &TextSpec, mut width: impl FnMut(&str) -> f32) -> Arc<str> {
    let suffix = match spec.overflow {
        Overflow::Wrap => return spec.text.clone(),
        Overflow::Clip => "",
        Overflow::Ellipsis => ELLIPSIS,
    };
    let Some(max) = spec.max_width else {
        return spec.text.clone();
    };
    if width(&spec.text) <= max {
        return spec.text.clone();
    }
    let text: &str = &spec.text;
    let ends: Vec<usize> = text.char_indices().map(|(at, _)| at).chain([text.len()]).collect();
    let cut = |count: usize| format!("{}{suffix}", text[..ends[count]].trim_end());
    let (mut lo, mut hi) = (0, ends.len().saturating_sub(2));
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if width(&cut(mid)) <= max {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    cut(lo).into()
}

/// The backend's shaped text. The engine that made it is the only one that reads it.
#[derive(Clone)]
pub struct TextHandle(Arc<dyn Any + Send + Sync>);

impl TextHandle {
    pub fn new(value: Arc<dyn Any + Send + Sync>) -> TextHandle {
        TextHandle(value)
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }

    pub fn ptr_eq(&self, other: &TextHandle) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for TextHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TextHandle({:p})", Arc::as_ptr(&self.0))
    }
}

/// A laid-out text: the handle plus the metrics layout needs. Logical points.
#[derive(Clone, Debug)]
pub struct TextLayout {
    pub handle: TextHandle,
    pub size: Vec2,
    /// Distance from the top to the first baseline.
    pub baseline: f32,
    pub lines: usize,
}

/// Text shaping and metrics, supplied by the renderer's backend (epaint `Fonts`, skparagraph), so
/// that a layout measured in the layout phase is the one the renderer draws. One engine serves
/// every window; layouts are shaped per scale, so windows at different scales shape their own.
pub trait TextEngine {
    /// Shapes `spec` for drawing at `scale` (glyph positions snap to that scale's pixels).
    fn layout(&mut self, spec: &TextSpec, scale: f32) -> TextLayout;
    /// The byte index of the caret position nearest `pos` (layout coordinates).
    fn hit_index(&mut self, layout: &TextLayout, pos: Pos2) -> usize;
    /// The caret rect before byte `index`, in layout coordinates.
    fn caret_rect(&mut self, layout: &TextLayout, index: usize) -> Rect;
}
