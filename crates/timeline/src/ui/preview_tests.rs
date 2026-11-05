//! The frame preview.

use egui::epaint::Shape;
use egui::{Context, LayerId, RawInput, Rect, TextureId, pos2, vec2};

use super::preview::{FrameFit, paint_frame};

/// The rect and texture coordinates `paint_frame` draws the texture with.
fn painted(rect: Rect, size: egui::Vec2, fit: FrameFit) -> (Rect, Rect) {
    let ctx = Context::default();
    let mut output = ctx.run_ui(RawInput::default(), |ctx| {
        paint_frame(&ctx.layer_painter(LayerId::background()), rect, TextureId::default(), size, fit);
    });
    output.textures_delta.clear();
    let mesh = output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Mesh(mesh) => Some(mesh.clone()),
            _ => None,
        })
        .expect("the frame is drawn as an image");
    let (first, last) =
        (mesh.vertices.first().unwrap(), mesh.vertices.iter().max_by(|a, b| (a.pos.x + a.pos.y).total_cmp(&(b.pos.x + b.pos.y))).unwrap());
    let corners = mesh.vertices.iter().fold((first.pos, first.uv), |(min, uv), v| (min.min(v.pos), uv.min(v.uv)));
    (Rect::from_min_max(corners.0, last.pos), Rect::from_min_max(corners.1, last.uv))
}

// a 4:3 frame in a 16:9 box is centred at full height.
#[test]
fn letterbox_keeps_the_aspect() {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1600.0, 900.0));
    let (target, uv) = painted(rect, vec2(640.0, 480.0), FrameFit::Letterbox);
    assert_eq!(target, Rect::from_min_size(pos2(200.0, 0.0), vec2(1200.0, 900.0)));
    assert_eq!(uv, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)));
}

// a cropped frame's UVs are the crop, clamped.
#[test]
fn crop_fills_the_box() {
    let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(160.0, 90.0));
    let (target, uv) = painted(rect, vec2(640.0, 480.0), FrameFit::Crop([0.25, 0.1, f32::NAN, 5.0]));
    assert_eq!(target, rect);
    assert_eq!(uv, Rect::from_min_max(pos2(0.25, 0.1), pos2(1.0, 0.11)));
}
