use std::sync::Arc;

use craft_theme::FontSlot;
use craft_ui_rt::{
    Clip, CornerRadii, Dash, Fill, FontSpec, ImageKey, Item, Mesh, Overflow, Rect, Renderer, RgbaImage, SizeU, State, Stroke, StrokeAlign,
    TextEngine, TextSpec, Vec2, Vertex, pos2, vec2,
};

use craft_ui_render_epaint::{EpaintOptions, EpaintRenderer};

use crate::common::{Painted, Scene, Stage, fonts, paint, px, renderer, rgba};

/// A scene with fixed content and a chip at a fractional offset whose colour follows `shade`.
fn demo(size: SizeU, scale: f32, shade: u8) -> (Scene, State<u8>) {
    let mut cell = None;
    let scene = Scene::new(renderer(), size, scale, |rt, r| {
        let shade = rt.store_mut().create(shade);
        cell = Some(shade);
        let label = r.text().layout(
            &TextSpec {
                text: "Layer 12".into(),
                font: FontSpec { slot: FontSlot::Ui, size: 13.0 },
                line_height: None,
                color: rgba(240, 240, 240, 255),
                max_width: None,
                overflow: Overflow::Wrap,
            },
            scale,
        );
        let backdrop = Painted {
            id: 1,
            size: vec2(200.0, 120.0),
            paint: paint(move |cx| {
                cx.vgradient(Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 120.0)), rgba(30, 40, 90, 255), rgba(90, 40, 30, 255));
                cx.rect(
                    Rect::from_min_size(pos2(8.2, 8.7), vec2(70.0, 40.0)),
                    6.0,
                    rgba(200, 200, 60, 255),
                    Some(Stroke::new(2.0, rgba(0, 0, 0, 255)).aligned(StrokeAlign::Outside)),
                );
                cx.circle(pos2(120.0, 30.0), 18.0, rgba(60, 200, 120, 255), Some(Stroke::new(1.5, rgba(255, 255, 255, 255))));
                cx.line(pos2(10.0, 110.0), pos2(190.0, 100.5), Stroke::new(1.0, rgba(255, 255, 255, 255)));
            }),
        };
        let chip = Painted {
            id: 2,
            size: vec2(60.0, 24.0),
            paint: paint(move |cx| {
                let v = shade.get(cx);
                cx.rect(Rect::from_min_size(pos2(0.0, 0.0), vec2(60.0, 24.0)), 4.0, rgba(v, 100, 255 - v, 255), None);
                cx.text(&label, pos2(5.3, 4.2), None);
            }),
        };
        let shadowed = Painted {
            id: 3,
            size: vec2(60.0, 30.0),
            paint: paint(|cx| {
                cx.shadow(Rect::from_min_size(pos2(0.0, 0.0), vec2(60.0, 30.0)), 5.0, vec2(0.0, 3.0), 8.0, 1.0, rgba(0, 0, 0, 160));
                cx.rect(Rect::from_min_size(pos2(0.0, 0.0), vec2(60.0, 30.0)), 5.0, rgba(235, 235, 240, 255), None);
            }),
        };
        Stage { kids: vec![(Vec2::ZERO, backdrop), (vec2(10.3, 70.7), chip), (vec2(110.5, 60.25), shadowed)] }
    });
    (scene, cell.expect("the state was made"))
}

fn area(rects: &[Rect]) -> f32 {
    rects.iter().map(|r| r.width() * r.height()).sum()
}

fn damage_only_equals_full_redraw(scale: f32) {
    let size = SizeU::new((200.0 * scale) as u32, (120.0 * scale) as u32);
    let (mut scene, shade) = demo(size, scale, 40);
    scene.frame();
    shade.set(&mut scene.rt, 210);
    let stats = scene.frame();
    assert!(area(&scene.damage) < 200.0 * 120.0 * 0.15, "a chip changed, the damage is {:?}", scene.damage);
    assert_eq!(stats.prepared, 1, "only the chip's list is tessellated again");
    let partial = scene.pixels();

    let (mut full, _) = demo(size, scale, 210);
    full.frame();
    assert_eq!(partial, full.pixels(), "damage-only drawing differs from a full redraw at scale {scale}");
    // The change itself shows: the chip's colour is the new one.
    let at = |s: f32| (((10.3 + 30.0) * s) as u32, ((70.7 + 2.0) * s) as u32);
    let (x, y) = at(scale);
    assert_eq!(px(&partial, x, y)[0], 210);
}

#[test]
fn damage_only_drawing_equals_a_full_redraw_byte_for_byte() {
    damage_only_equals_full_redraw(1.0);
    damage_only_equals_full_redraw(1.5);
}

/// Text drawn after the glyph atlas was cleared is shaped again and draws the same pixels.
#[test]
fn text_laid_out_before_an_atlas_reset_draws_after_it() {
    let small = || {
        let options =
            EpaintOptions { text: epaint::text::TextOptions { max_texture_side: 1024, ..Default::default() }, ..EpaintOptions::predictable() };
        EpaintRenderer::create_blocking(options, fonts(), None).expect("a wgpu adapter")
    };
    let size = SizeU::new(120, 40);
    let spec = |size: f32| TextSpec {
        text: "Layer 12 gjpqy".into(),
        font: FontSpec { slot: FontSlot::Ui, size },
        line_height: None,
        color: rgba(255, 255, 255, 255),
        max_width: None,
        overflow: Overflow::Wrap,
    };
    let make = |r: &mut EpaintRenderer| {
        let label = r.text().layout(&spec(14.0), 1.0);
        Stage { kids: vec![(Vec2::ZERO, Painted { id: 1, size: vec2(120.0, 40.0), paint: paint(move |cx| cx.text(&label, pos2(4.0, 4.0), None)) })] }
    };
    let mut fresh = Scene::new(small(), size, 1.0, |_, r| make(r));
    fresh.frame();
    let expected = fresh.pixels();
    assert!(expected.pixels.chunks_exact(4).any(|p| p[3] > 0), "the label draws");

    let mut churned = Scene::new(small(), size, 1.0, |_, r| make(r));
    let before = churned.r.atlas_generation();
    // Fill the atlas with glyphs of many sizes until it is cleared at the frame's pass.
    for s in 8..90 {
        churned.r.text().layout(&spec(s as f32 + 0.5), 1.0);
    }
    churned.frame();
    assert_ne!(churned.r.atlas_generation(), before, "the atlas was cleared");
    assert_eq!(churned.pixels(), expected, "the text laid out before the reset draws as a fresh one");
}

/// Items with a known place on a 200 x 120 window.
fn items_scene() -> Scene {
    Scene::new(renderer(), SizeU::new(200, 120), 1.0, |rt, _| {
        let mask = ImageKey(7);
        let mut pixels = RgbaImage::new(SizeU::new(8, 8));
        for y in 0..8 {
            for x in 0..4 {
                pixels.write(craft_ui_rt::RectU::new(x, y, x + 1, y + 1), &[10, 10, 10, 255]);
            }
        }
        rt.images_mut().insert(mask, Arc::new(pixels));
        let tri = Arc::new(Mesh {
            vertices: vec![
                Vertex { pos: pos2(100.0, 70.0), uv: pos2(0.0, 0.0), color: rgba(255, 0, 0, 255) },
                Vertex { pos: pos2(130.0, 70.0), uv: pos2(0.0, 0.0), color: rgba(0, 255, 0, 255) },
                Vertex { pos: pos2(115.0, 100.0), uv: pos2(0.0, 0.0), color: rgba(0, 0, 255, 255) },
            ],
            indices: vec![0, 1, 2],
            texture: None,
        });
        Stage {
            kids: vec![(
                Vec2::ZERO,
                Painted {
                    id: 1,
                    size: vec2(200.0, 120.0),
                    paint: paint(move |cx| {
                        cx.fill_rect(Rect::from_min_size(pos2(10.0, 10.0), vec2(30.0, 30.0)), rgba(255, 0, 0, 255));
                        // A rounded clip cuts the corners of a fill that is square.
                        cx.push_clip(Clip { rect: Rect::from_min_size(pos2(60.0, 10.0), vec2(40.0, 40.0)), radii: CornerRadii::same(18.0) });
                        cx.fill_rect(Rect::from_min_size(pos2(50.0, 0.0), vec2(60.0, 60.0)), rgba(0, 0, 255, 255));
                        cx.pop_clip();
                        cx.shadow(Rect::from_min_size(pos2(130.0, 15.0), vec2(40.0, 25.0)), 4.0, vec2(0.0, 4.0), 8.0, 0.0, rgba(0, 0, 0, 255));
                        cx.mesh(tri.clone());
                        cx.push_opacity(0.5, Rect::from_min_size(pos2(10.0, 50.0), vec2(30.0, 20.0)));
                        cx.fill_rect(Rect::from_min_size(pos2(10.0, 50.0), vec2(30.0, 20.0)), rgba(255, 255, 255, 255));
                        cx.pop_opacity();
                        cx.line(
                            pos2(10.0, 90.0),
                            pos2(90.0, 90.0),
                            Stroke::new(4.0, rgba(255, 255, 0, 255)).dashed(Dash { on: 10.0, off: 10.0, offset: 0.0 }),
                        );
                        cx.image_mask(mask, Rect::from_min_size(pos2(10.0, 100.0), vec2(8.0, 8.0)), rgba(0, 255, 0, 255));
                    }),
                },
            )],
        }
    })
}

#[test]
fn every_new_display_item_draws_where_it_belongs() {
    let mut scene = items_scene();
    scene.frame();
    let img = scene.pixels();
    assert_eq!(px(&img, 20, 20), [255, 0, 0, 255], "rect");
    // Rounded clip: the clip's corner is empty, its middle is filled, its straight edge is kept.
    assert_eq!(px(&img, 61, 11)[3], 0, "the clipped corner stays empty");
    assert_eq!(px(&img, 80, 30), [0, 0, 255, 255], "inside the rounded clip");
    assert_eq!(px(&img, 80, 11), [0, 0, 255, 255], "the top edge between the corners is kept");
    assert_eq!(px(&img, 80, 52)[3], 0, "below the clip");
    // Shadow: soft below the rect, nothing far from it, nothing drawn where the rect would be solid.
    assert!(px(&img, 150, 44)[3] > 0, "the shadow reaches under the rect");
    assert_eq!(px(&img, 150, 100)[3], 0, "and not far from it");
    // Mesh: the centre mixes the three vertex colours.
    let c = px(&img, 115, 80);
    assert_eq!(c[3], 255);
    assert!(c[0] > 40 && c[1] > 40 && c[2] > 40, "{c:?}");
    // Opacity.
    let o = px(&img, 20, 60);
    assert!((120..=136).contains(&o[3]), "half transparent: {o:?}");
    // Dashes: on from 10 to 20, off from 20 to 30.
    assert!(px(&img, 15, 90)[3] > 200 && px(&img, 25, 90)[3] == 0 && px(&img, 35, 90)[3] > 200);
    // Image mask: the opaque left half takes the colour, the clear right half stays empty.
    assert_eq!(px(&img, 12, 104), [0, 255, 0, 255]);
    assert_eq!(px(&img, 16, 104)[3], 0);
    let _ = (Item::PopClip, Fill::new(rgba(0, 0, 0, 0)));
}
