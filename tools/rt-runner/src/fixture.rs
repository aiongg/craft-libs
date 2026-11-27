//! The runner's fake layer document: the 20-layer pattern the checks and benches share, and the thumbnail
//! pictures the runner generates as a host would.

use craft_rt_layers::{Blend, Doc, Kind, Layer, LayerId};
use craft_ui_rt::{ImageKey, hash_key};

/// Panel size in logical points (plan section 2).
pub const PANEL_WIDTH: f32 = 300.0;
pub const PANEL_HEIGHT: f32 = 640.0;

/// `n` layers: "Layer 1" (top) .. "Layer n-1", then "Background" (locked, bottom, id 0).
/// Ids 1.. follow a 20-layer pattern. "Layer 1" is selected and active.
pub fn doc(n: usize) -> Doc {
    let n = n.max(1);
    let mut layers: Vec<Layer> = (1..n as u64).map(pattern_layer).collect();
    layers.push(Layer {
        id: 0,
        name: "Background".into(),
        kind: Kind::Pixel,
        visible: true,
        locked: true,
        blend: Blend::Normal,
        thumb: None,
        thumb_uv: Doc::FULL_UV,
    });
    let first = layers.first().map(|l| l.id);
    Doc::from_rows(layers, first, first)
}

fn pattern_layer(id: LayerId) -> Layer {
    let p = id % 20;
    Layer {
        id,
        name: format!("Layer {id}"),
        kind: match p {
            5 => Kind::Type,
            9 => Kind::Fill,
            _ => Kind::Pixel,
        },
        visible: !matches!(p, 3 | 13),
        locked: p == 11,
        blend: if p == 7 { Blend::Multiply } else { Blend::Normal },
        thumb: None,
        thumb_uv: Doc::FULL_UV,
    }
}

/// What colours a layer's picture: its id, as the pictures were once seeded.
fn seed(layer: &Layer) -> u32 {
    (layer.id as u32).wrapping_mul(2654435761)
}

fn kind_code(kind: Kind) -> u8 {
    match kind {
        Kind::Pixel => 0,
        Kind::Type => 1,
        Kind::Fill => 2,
    }
}

/// The picture's key: one per (layer look, side in pixels).
pub fn thumb_key(layer: &Layer, side: u32) -> ImageKey {
    ImageKey(hash_key(&("thumb", seed(layer), kind_code(layer.kind), side)))
}

/// Straight RGBA8, `px` x `px`, row-major. Deterministic: every variant uploads the same pixels.
pub fn thumbnail(layer: &Layer, px: u32) -> Vec<u8> {
    let seed = seed(layer);
    let mut out = Vec::with_capacity((px * px * 4) as usize);
    let base = [(seed >> 16) as u8, (seed >> 8) as u8, seed as u8];
    for y in 0..px {
        for x in 0..px {
            let rgb = match layer.kind {
                Kind::Pixel => {
                    let t = ((x + y) * 255 / (2 * px.max(2) - 2)) as u8;
                    [base[0] ^ t, base[1].wrapping_add(t / 2), base[2] ^ (255 - t)]
                }
                Kind::Fill => base,
                Kind::Type => {
                    // Ink glyph block on paper.
                    let ink = x > px / 4 && x < px * 3 / 4 && (y > px / 4 && y < px / 2 || (x > px * 7 / 16 && x < px * 9 / 16 && y < px * 3 / 4));
                    if ink { [24, 24, 28] } else { [244, 242, 236] }
                }
            };
            out.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    out
}
