//! The suite's icons: an [`Icon`] by meaning, rasterised once per pixel size and tinted when drawn.
//!
//! `crates/ui/icons/icons.toml` lists the icons and, for each [`IconSet`], the file
//! that draws them. [`IconSet::BASE`] (Phosphor) has a file for every icon; any other set is a
//! pack that lists only the icons it has, and [`Icon::svg`] gives the base set's file for the rest
//! ([`Icon::drawn_in`] tells which); `build.rs` embeds the files. [`paint`] draws an icon at a size class in the
//! context's set ([`set_active`], Lucide by default). Each (icon, set, pixel size, stroke,
//! mirroring) is rasterised once per context with `resvg`, in white, into an egui texture.
//!
//! Stroke sets (Lucide, Tabler) are drawn with the suite's stroke width ([`IconSize::stroke`]),
//! whatever the size; fill sets (Phosphor, Carbon) as drawn. [`Icon::directional`] icons mirror
//! in a right-to-left interface ([`crate::direction`]).
//!
//! ```
//! use craft_ui::icons::{self, Icon, IconSize};
//! # egui::__run_test_ui(|ui| {
//! let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
//! icons::paint(ui, rect, Icon::Close, IconSize::Md, ui.visuals().text_color());
//! # });
//! ```

mod compare;
mod raster;
mod svg_set;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use egui::emath::GuiRounding as _;
use egui::{Color32, Context, Id, Rect, TextureHandle, TextureId, TextureOptions, Ui, pos2, vec2};

use crate::direction;
use crate::theme::{Tokens, TokensOf};

pub use compare::stories;
#[allow(deprecated, reason = "re-export of the deprecated catalog")]
pub use svg_set::{SvgIconSet, SvgStyle};

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

/// The components of this module with stories.
pub const COMPONENTS: &[&str] = &["Icons"];

/// An icon size class; the sizes in points are the theme's `size.icon` tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconSize {
    /// Inline and small-control icons.
    Sm,
    /// Standard icons.
    Md,
    /// Rail and toolbar icons.
    Lg,
}

impl IconSize {
    /// Every size class, small to large.
    pub const ALL: [IconSize; 3] = [IconSize::Sm, IconSize::Md, IconSize::Lg];

    /// The size in points.
    pub fn points(self, tokens: &Tokens) -> f32 {
        let icon = &tokens.size.icon;
        match self {
            IconSize::Sm => icon.sm,
            IconSize::Md => icon.md,
            IconSize::Lg => icon.lg,
        }
    }

    /// The drawn stroke width of stroke sets at this size, in points.
    pub const fn stroke(self) -> f32 {
        match self {
            IconSize::Sm => 1.25,
            IconSize::Md | IconSize::Lg => 1.5,
        }
    }
}

/// The pixel size of an icon `points` wide: rounded, at least 1.
pub fn pixels(points: f32, pixels_per_point: f32) -> u32 {
    (points * pixels_per_point).round().max(1.0) as u32
}

/// Draws `icon` from the context's set, centred in `rect` and tinted `color`.
pub fn paint(ui: &Ui, rect: Rect, icon: Icon, size: IconSize, color: Color32) {
    let points = size.points(&Tokens::of(ui));
    paint_in(ui, rect, icon, active(ui.ctx()), points, size.stroke(), color);
}

/// Draws `icon` from `set`, `points` wide with strokes `stroke` points wide, centred in `rect`.
pub fn paint_in(ui: &Ui, rect: Rect, icon: Icon, set: IconSet, points: f32, stroke: f32, color: Color32) {
    let ctx = ui.ctx();
    let ppp = ctx.pixels_per_point();
    let px = pixels(points, ppp);
    let texture = texture_in(ctx, icon, set, px, stroke * ppp);
    let side = px as f32 / ppp;
    let min = (rect.center() - vec2(side, side) / 2.0).round_to_pixels(ppp);
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    ui.painter().image(texture, Rect::from_min_size(min, vec2(side, side)), uv, color);
}

/// The texture of `icon` from the context's set at `px` × `px` pixels, with the standard stroke.
pub fn texture(ctx: &Context, icon: Icon, px: u32) -> TextureId {
    texture_in(ctx, icon, active(ctx), px, IconSize::Md.stroke() * ctx.pixels_per_point())
}

/// The texture of `icon` from `set` at `px` × `px` pixels, strokes `stroke_px` pixels wide.
///
/// Mirrored when the icon is directional and the context's interface runs right to left.
pub fn texture_in(ctx: &Context, icon: Icon, set: IconSet, px: u32, stroke_px: f32) -> TextureId {
    let key = Key {
        icon,
        set,
        px,
        // Fill sets ignore the stroke, so all their requests share one entry.
        stroke: if set.stroke_width().is_some() { (stroke_px * 64.0).round() as u32 } else { 0 },
        mirrored: icon.directional() && direction::is_rtl(ctx),
    };
    if let Some(id) = ctx.data_mut(|d| d.get_temp_mut_or_default::<Cache>(cache_id()).textures.get(&key).map(TextureHandle::id)) {
        return id;
    }
    let image = raster::rasterise(icon, set, px, key.stroke as f32 / 64.0, key.mirrored);
    let name = format!("craft_ui::icons/{}/{}/{px}{}", set.id(), icon.name(), if key.mirrored { "/rtl" } else { "" });
    let handle = ctx.load_texture(name, image, TextureOptions::LINEAR);
    let id = handle.id();
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Cache>(cache_id()).textures.insert(key, handle));
    id
}

/// The set [`paint`] and [`texture`] draw from in this context.
pub fn active(ctx: &Context) -> IconSet {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Cache>(cache_id()).active).unwrap_or(IconSet::Lucide)
}

/// Makes [`paint`] and [`texture`] draw from `set` in this context.
pub fn set_active(ctx: &Context, set: IconSet) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Cache>(cache_id()).active = Some(set));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    icon: Icon,
    set: IconSet,
    px: u32,
    /// Stroke width in 1/64 px; 0 for fill sets.
    stroke: u32,
    mirrored: bool,
}

#[derive(Clone, Default)]
struct Cache {
    textures: HashMap<Key, TextureHandle>,
    active: Option<IconSet>,
}

fn cache_id() -> Id {
    Id::unique("craft_ui::icons")
}
