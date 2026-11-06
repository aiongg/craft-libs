//! Drawing at several interface scales: strokes, lines, icons and text land on the device grid.

use craft_theme::scale::Stroke as ThemeStroke;
use craft_ui::canvas::crisp_shape;
use craft_ui::controls::Button;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::theme::{Base, StrokeExt as _, Tokens, TokensOf as _, install};
use egui::{Color32, Rect, Shape, Stroke, pos2, vec2};
use egui_kittest::Harness;

const SCALES: [f32; 4] = [1.0, 1.25, 1.5, 2.0];

fn harness(ppp: f32) -> egui_kittest::Harness<'static> {
    Harness::builder()
        .with_size([200.0, 100.0])
        .with_pixels_per_point(ppp)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui(|_| {})
}

fn on_grid(points: f32, ppp: f32) -> bool {
    let px = points * ppp;
    (px - px.round()).abs() < 1e-3
}

fn flatten<'a>(shape: &'a Shape, out: &mut Vec<&'a Shape>) {
    match shape {
        Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
        other => out.push(other),
    }
}

fn shapes_of(h: &Harness<'_>) -> Vec<Shape> {
    h.output().shapes.iter().map(|s| s.shape.clone()).collect()
}

fn leaves(shapes: &[Shape]) -> Vec<&Shape> {
    let mut out = Vec::new();
    shapes.iter().for_each(|s| flatten(s, &mut out));
    out
}

#[test]
fn a_hairline_is_one_device_pixel_at_every_scale() {
    for ppp in SCALES {
        let h = harness(ppp);
        let width = ThemeStroke::Hairline.egui(&h.ctx, Color32::WHITE).width;
        assert!((width * ppp - 1.0).abs() < 1e-5, "{ppp}");
    }
}

#[test]
fn a_crisp_horizontal_line_covers_whole_device_pixels_on_the_grid() {
    for ppp in SCALES {
        for y in [10.0_f32, 10.3, 10.5] {
            let Shape::Mesh(mesh) = crisp_shape(pos2(5.3, y), pos2(60.7, y), Stroke::new(1.0, Color32::WHITE), ppp) else {
                panic!("{ppp}: not a mesh")
            };
            let bounds = mesh.calc_bounds();
            for v in [bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y] {
                assert!(on_grid(v, ppp), "{ppp} y={y}: {v}");
            }
            assert!(bounds.height() * ppp >= 1.0 - 1e-3, "{ppp}");
        }
    }
}

#[test]
fn an_icon_texture_is_the_points_times_the_scale_rounded() {
    for ppp in SCALES {
        let points = std::cell::Cell::new(0.0_f32);
        let mut h = Harness::builder()
            .with_size([100.0, 100.0])
            .with_pixels_per_point(ppp)
            .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
            .build_ui(|ui| {
                points.set(IconSize::Md.points(&Tokens::of(ui)));
                icons::paint(ui, Rect::from_min_size(pos2(10.0, 10.0), vec2(20.0, 20.0)), Icon::Search, IconSize::Md, Color32::WHITE);
            });
        h.run();
        let expected = (points.get() * ppp).round() as usize;
        let shapes = shapes_of(&h);
        let textured: Vec<_> = leaves(&shapes)
            .into_iter()
            .filter_map(|s| match s {
                Shape::Rect(r) if r.fill_texture_id() != egui::TextureId::default() => Some(r.fill_texture_id()),
                Shape::Mesh(m) if m.texture_id != egui::TextureId::default() => Some(m.texture_id),
                _ => None,
            })
            .collect();
        assert_eq!(textured.len(), 1, "{ppp}");
        let size = h.ctx.tex_manager().read().meta(textured[0]).map(|m| m.size);
        assert_eq!(size, Some([expected, expected]), "{ppp}");
    }
}

#[test]
fn a_button_label_sits_on_the_device_grid() {
    for ppp in SCALES {
        let mut h = Harness::builder()
            .with_size([200.0, 100.0])
            .with_pixels_per_point(ppp)
            .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
            .build_ui(|ui| {
                ui.add(Button::new("Apply"));
            });
        h.run();
        let shapes = shapes_of(&h);
        let texts: Vec<_> = leaves(&shapes).into_iter().filter_map(|s| if let Shape::Text(t) = s { Some(t.pos) } else { None }).collect();
        assert!(!texts.is_empty(), "{ppp}");
        for pos in texts {
            assert!(on_grid(pos.x, ppp) && on_grid(pos.y, ppp), "{ppp}: {pos:?}");
        }
    }
}
