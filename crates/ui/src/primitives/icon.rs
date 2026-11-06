//! `IconSpec`: an icon to paint.

use crate::style::Ink;
use egui::{Color32, Painter, Rect};

/// Whether a directional icon flips under right-to-left.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Mirror {
    /// Flips a directional icon under right-to-left.
    #[default]
    Auto,
    /// Never flips.
    Never,
}

/// A value glyph: an icon drawn from a value.
pub trait Glyph {
    /// Whether the glyph has a reading direction (an arrow) and so flips under right-to-left.
    fn directional(&self) -> bool {
        false
    }

    /// Paints the glyph into `rect` in `color`.
    fn paint(&self, painter: &Painter, rect: Rect, color: Color32);
}

/// An icon to paint, from a value glyph.
#[non_exhaustive]
pub struct IconSpec<'a> {
    /// The glyph.
    pub glyph: &'a dyn Glyph,
    /// The ink the icon reads in.
    pub ink: Ink,
    /// Mirroring under right-to-left.
    pub mirror: Mirror,
}

impl<'a> IconSpec<'a> {
    /// An icon of `glyph` in the panel ink.
    pub fn glyph(glyph: &'a dyn Glyph) -> Self {
        IconSpec { glyph, ink: Ink::OnPanel, mirror: Mirror::Auto }
    }

    /// Sets the ink.
    pub fn ink(mut self, ink: Ink) -> Self {
        self.ink = ink;
        self
    }

    /// Sets the mirroring.
    pub fn mirror(mut self, mirror: Mirror) -> Self {
        self.mirror = mirror;
        self
    }
}
