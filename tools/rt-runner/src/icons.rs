//! The suite's Lucide icons as white masks, rasterised the way craft-ui rasterises them
//! (`svg_set::rasterise`): a stroke icon is drawn `stroke.icon` of its side wide, at whole
//! physical pixels, and the display list tints the mask with the ink.

use std::sync::Arc;

use craft_svg::tree::draw::{self as svg_draw, tiny_skia};
use craft_svg::tree::{self, Limits, usvg};
use craft_ui_rt::{BuildCx, ImageKey, RgbaImage, SizeU, hash_key};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Eye,
    Lock,
    LockOpen,
    Text,
    Trash,
    NewLayer,
}

fn svg(icon: Icon) -> &'static [u8] {
    match icon {
        Icon::Eye => include_bytes!("../assets/icons/eye.svg"),
        Icon::Lock => include_bytes!("../assets/icons/lock.svg"),
        Icon::LockOpen => include_bytes!("../assets/icons/lock-open.svg"),
        Icon::Text => include_bytes!("../assets/icons/type.svg"),
        Icon::Trash => include_bytes!("../assets/icons/trash.svg"),
        Icon::NewLayer => include_bytes!("../assets/icons/square-plus.svg"),
    }
}

/// The set's own `stroke-width` in view-box units (Lucide).
const OWN_STROKE: f32 = 2.0;
/// Bytes drawing one icon may allocate, its target included.
const DRAW_BUDGET: u64 = 64 << 20;

fn limits() -> Limits {
    Limits {
        bytes: 1 << 20,
        depth: 32,
        tree_depth: 32,
        xml_nodes: 10_000,
        allow_dtd: false,
        refuse_entity_markup: true,
        embedded_bytes: 0,
        embedded_depth: 0,
        embedded_raster_pixels: 0,
        ..Limits::default()
    }
}

fn parse(svg: &[u8]) -> Result<usvg::Tree, String> {
    let options = usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver { resolve_data: Box::new(|_, _, _| None), resolve_string: Box::new(|_, _| None) },
        ..usvg::Options::default()
    };
    tree::parse(svg, &limits(), &options).map_err(|e| e.to_string())
}

/// `svg` with each `stroke-width="..."` attribute multiplied by `factor`.
fn scale_stroke_widths(svg: &str, factor: f32) -> String {
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

fn draw(svg: &[u8], px: u32, stroke_px: f32) -> Result<tiny_skia::Pixmap, String> {
    let mut tree = parse(svg)?;
    let view = tree.size();
    let units = stroke_px * view.width() / px as f32;
    let text = std::str::from_utf8(svg).map_err(|e| e.to_string())?;
    tree = parse(scale_stroke_widths(text, units / OWN_STROKE).as_bytes())?;
    let (sx, sy) = (px as f32 / view.width(), px as f32 / view.height());
    svg_draw::render(&tree, tiny_skia::Transform::from_scale(sx, sy), px, px, 0, DRAW_BUDGET).map_err(|refusal| refusal.message)
}

/// The icon in white at `px` x `px` physical pixels, its alpha from the drawing; transparent for
/// a document that fails to draw.
fn rasterise(icon: Icon, px: u32, stroke64: u32) -> RgbaImage {
    let size = SizeU::new(px, px);
    let mut image = RgbaImage::new(size);
    match draw(svg(icon), px, stroke64 as f32 / 64.0) {
        Ok(pixmap) => {
            for (out, p) in image.pixels.chunks_exact_mut(4).zip(pixmap.pixels()) {
                out.copy_from_slice(&[255, 255, 255, p.alpha()]);
            }
        }
        Err(error) => eprintln!("icon {icon:?}: {error}"),
    }
    image
}

/// The side in pixels and the stroke in 64ths of a pixel of an icon `side` points wide at
/// `scale` (craft-ui's texture key).
pub fn pixels(side: f32, scale: f32, stroke_share: f32) -> (u32, u32) {
    // In f32 and in craft's order (`SvgIcon::pixels`), so a stroke on a 64th's tie rounds as craft's does.
    let side = side as f32;
    let px = (side * scale).round().max(1.0) as u32;
    let stroke = (stroke_share * side * scale * 64.0).round() as u32;
    (px, stroke)
}

/// The key of `icon`'s mask at that pixel size, registered on first use.
pub fn mask(cx: &mut BuildCx<'_>, icon: Icon, px: u32, stroke64: u32) -> ImageKey {
    let key = ImageKey(hash_key(&("icon", icon, px, stroke64)));
    if cx.images().get(key).is_none() {
        cx.images().insert(key, Arc::new(rasterise(icon, px, stroke64)));
    }
    key
}
