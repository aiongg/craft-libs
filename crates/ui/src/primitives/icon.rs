//! `IconSpec`: an icon to paint, and the SVG set it can name.

use super::svg_set;
use super::textures::TextureCache;
use crate::style::{Ink, UiTokens, ink_color};
use craft_behavior::IconId;
use craft_i18n::Direction;
use egui::emath::GuiRounding as _;
use egui::{Color32, Painter, Rect, pos2, vec2};

pub use super::svg_set::IconSet;

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

    /// Paints the glyph into `rect` in `color`. `mirrored` is set when a directional glyph is painted under
    /// right-to-left: the glyph draws itself flipped.
    fn paint(&self, painter: &Painter, rect: Rect, color: Color32, mirrored: bool);
}

/// Where an icon's drawing comes from.
#[non_exhaustive]
pub enum IconSource<'a> {
    /// An SVG-set or tool icon, by its egui-free key.
    Svg(IconId),
    /// A value glyph.
    Glyph(&'a dyn Glyph),
}

/// An icon to paint.
#[non_exhaustive]
pub struct IconSpec<'a> {
    /// The drawing.
    pub source: IconSource<'a>,
    /// The ink the icon reads in.
    pub ink: Ink,
    /// Mirroring under right-to-left.
    pub mirror: Mirror,
    /// The set an SVG icon is drawn from.
    pub set: IconSet,
}

impl<'a> IconSpec<'a> {
    /// An icon of the SVG set or a tool icon, in the panel ink.
    pub fn svg(id: IconId) -> Self {
        IconSpec { source: IconSource::Svg(id), ink: Ink::OnPanel, mirror: Mirror::Auto, set: IconSet::default() }
    }

    /// An icon of `glyph` in the panel ink.
    pub fn glyph(glyph: &'a dyn Glyph) -> Self {
        IconSpec { source: IconSource::Glyph(glyph), ink: Ink::OnPanel, mirror: Mirror::Auto, set: IconSet::default() }
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

    /// Sets the set an SVG icon is drawn from.
    pub fn set(mut self, set: IconSet) -> Self {
        self.set = set;
        self
    }
}

/// Whether the catalog has an icon named `id`.
pub fn has_icon(id: &IconId) -> bool {
    svg_set::find(id.name()).is_some()
}

/// The names of the catalog's icons, sorted.
pub fn icon_names() -> &'static [&'static str] {
    svg_set::names()
}

/// Paints `spec` centred in the square of `rect`'s shorter side.
pub(crate) fn paint(painter: &Painter, tokens: &UiTokens, direction: Direction, rect: Rect, spec: &IconSpec<'_>) {
    let color = ink_color(tokens, spec.ink);
    let rtl = direction == Direction::Rtl;
    let side = rect.width().min(rect.height());
    let square = Rect::from_center_size(rect.center(), vec2(side, side));
    match &spec.source {
        IconSource::Glyph(glyph) => glyph.paint(painter, square, color, rtl && glyph.directional() && spec.mirror == Mirror::Auto),
        IconSource::Svg(id) => {
            let Some(index) = svg_set::find(id.name()) else {
                log::error!("icon {:?} is not in the catalog", id.name());
                return;
            };
            let mirrored = rtl && svg_set::directional(index) && spec.mirror == Mirror::Auto;
            svg(painter, tokens, square, index, spec.set, mirrored, color);
        }
    }
}

fn svg(painter: &Painter, tokens: &UiTokens, square: Rect, index: usize, set: IconSet, mirrored: bool, color: Color32) {
    let ctx = painter.ctx();
    let ppp = ctx.pixels_per_point();
    let px = (square.width() * ppp).round().max(1.0) as u32;
    // Fill sets ignore the stroke, so all their requests share one entry.
    let stroke_px = if set.stroke_width().is_some() { (tokens.stroke.medium * ppp * 64.0).round() as u32 } else { 0 };
    let key = (index, set, px, stroke_px, mirrored);
    let texture = TextureCache::texture(ctx, key, || svg_set::rasterise(index, set, px, stroke_px as f32 / 64.0, mirrored));
    let side = px as f32 / ppp;
    let min = (square.center() - vec2(side, side) / 2.0).round_to_pixels(ppp);
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    crate::runtime::egui::textures::paint(painter, texture, Rect::from_min_size(min, vec2(side, side)), uv, color);
}
