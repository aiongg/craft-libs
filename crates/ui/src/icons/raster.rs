//! SVG to alpha mask, with the suite's stroke width.

use egui::{Color32, ColorImage};
use resvg::{tiny_skia, usvg};

use super::{Icon, IconSet};

/// The icon in white at `px` × `px` physical pixels, its alpha from the drawing; tinted when drawn.
///
/// Stroke sets are drawn `stroke_px` physical pixels wide; fill sets ignore it. `mirrored` flips
/// the drawing horizontally. A document that fails to parse gives a transparent image (and a log
/// line): the embedded sets are checked by the tests.
pub(super) fn rasterise(icon: Icon, set: IconSet, px: u32, stroke_px: f32, mirrored: bool) -> ColorImage {
    let size = [px as usize; 2];
    match draw(icon.svg(set), set, px, stroke_px, mirrored) {
        Ok(pixmap) => ColorImage::new(size, pixmap.pixels().iter().map(|p| Color32::from_white_alpha(p.alpha())).collect()),
        Err(error) => {
            log::error!("icon {} in {}: {error}", icon.name(), set.id());
            ColorImage::filled(size, Color32::TRANSPARENT)
        }
    }
}

fn draw(svg: &[u8], set: IconSet, px: u32, stroke_px: f32, mirrored: bool) -> Result<tiny_skia::Pixmap, String> {
    let options = usvg::Options::default();
    let mut tree = usvg::Tree::from_data(svg, &options).map_err(|e| e.to_string())?;
    let view = tree.size();
    if let Some(own) = set.stroke_width() {
        // Every `stroke-width` scales by the same factor, so a set's thinner details stay thinner.
        let units = stroke_px * view.width() / px as f32;
        let text = std::str::from_utf8(svg).map_err(|e| e.to_string())?;
        tree = usvg::Tree::from_str(&scale_stroke_widths(text, units / own), &options).map_err(|e| e.to_string())?;
    }
    let mut pixmap = tiny_skia::Pixmap::new(px, px).ok_or("zero size")?;
    let (sx, sy) = (px as f32 / view.width(), px as f32 / view.height());
    let transform =
        if mirrored { tiny_skia::Transform::from_row(-sx, 0.0, 0.0, sy, px as f32, 0.0) } else { tiny_skia::Transform::from_scale(sx, sy) };
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Ok(pixmap)
}

/// `svg` with each `stroke-width="…"` attribute multiplied by `factor`.
pub(super) fn scale_stroke_widths(svg: &str, factor: f32) -> String {
    const ATTR: &str = "stroke-width=\"";
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(start) = rest.find(ATTR) {
        let value_start = start + ATTR.len();
        out.push_str(&rest[..value_start]);
        rest = &rest[value_start..];
        let end = rest.find('"').unwrap_or(rest.len());
        match rest[..end].parse::<f32>() {
            Ok(width) => out.push_str(&format!("{:.3}", width * factor)),
            Err(_) => out.push_str(&rest[..end]),
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}
