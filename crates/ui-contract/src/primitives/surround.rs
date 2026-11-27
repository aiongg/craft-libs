//! Painting the canvas surround: the fill around a document and the theme's pattern over it.

use super::{Canvas, CheckerArea, fill_polygon, items};
use crate::style::SurroundLook;
use craft_ui_rt::{Filter, ImageKey, ImageOptions, RgbaImage, SizeU, Wrap};
use egui::{Color32, Pos2};
use std::sync::Arc;

/// The side of the pattern's tile, in points: one dot per tile.
const TILE: f32 = 22.0;

/// The dot's four texels at the tile centre and their share of the pattern colour's alpha, the dot an antialiased
/// point slightly off the texel grid.
const DOT: [((usize, usize), u8); 4] = [((10, 10), 255), ((11, 10), 110), ((10, 11), 110), ((11, 11), 60)];

/// Paints the surround `look` over `area`: the fill, and with a pattern one textured shape of the dot tile over it.
/// The tile is laid in screen space from the origin, so the dots hold still while the view scrolls or turns. A rect takes its corners; a quad (a piece of a turned pasteboard) is filled as `Draw` fills a polygon.
pub fn paint_surround(canvas: &Canvas<'_>, area: CheckerArea, look: &SurroundLook) {
    match area {
        CheckerArea::Rect(rect, radius) => canvas.add(items::rect_filled(rect, radius, look.fill)),
        CheckerArea::Quad(corners) => {
            // One physical pixel of feathering, as egui's tessellator fades a convex fill.
            let feather = 1.0 / canvas.pixels_per_point();
            canvas.extend(items::mesh(fill_polygon(&corners, look.fill, feather)));
        }
    }
    let Some(pattern) = look.pattern else { return };
    let texture = dot_texture(canvas, pattern);
    let uv = |p: Pos2| (p.to_vec2() / TILE).to_pos2();
    canvas.extend(super::kit::tiled(texture, area, uv));
}

/// The repeating tile of one `pattern` dot, from the runtime's texture cache: one texture per pattern colour.
fn dot_texture(canvas: &Canvas<'_>, pattern: Color32) -> ImageKey {
    let options = ImageOptions { magnify: Filter::Linear, minify: Filter::Linear, mipmaps: false, wrap: Wrap::Repeat };
    let key = super::canvas::image_key(&("craft-ui-surround", pattern));
    canvas.services().image(key, options, &mut || {
        let side = TILE as usize;
        let [r, g, b, a] = pattern.to_srgba_unmultiplied();
        let mut pixels = vec![0u8; side * side * 4];
        for ((x, y), share) in DOT {
            let alpha = (u16::from(a) * u16::from(share) / 255) as u8;
            // Straight alpha, as egui's colour image holds it once unmultiplied.
            let texel = Color32::from_rgba_unmultiplied(r, g, b, alpha).to_srgba_unmultiplied();
            if let Some(pixel) = pixels.get_mut((y * side + x) * 4..(y * side + x) * 4 + 4) {
                pixel.copy_from_slice(&texel);
            }
        }
        Arc::new(RgbaImage { size: SizeU::new(side as u32, side as u32), pixels })
    });
    key
}
