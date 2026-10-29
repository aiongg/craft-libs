use egui::{ColorImage, Context, RawInput};

use super::raster::{rasterise, scale_stroke_widths};
use super::*;
use crate::direction::{self, Direction};

fn alpha(image: &ColorImage) -> impl Iterator<Item = u8> + '_ {
    image.pixels.iter().map(|p| p.a())
}

/// The suite default's size classes in points, from a bare context.
fn size_points() -> Vec<(IconSize, f32)> {
    let mut points = Vec::new();
    Context::default()
        .run_ui(RawInput::default(), |ui| {
            let tokens = Tokens::of(ui);
            points = IconSize::ALL.iter().map(|size| (*size, size.points(&tokens))).collect();
        })
        .drop_without_applying_deltas();
    points
}

#[test]
fn every_icon_rasterises_to_a_partial_mask_at_every_size_class() {
    for set in IconSet::ALL {
        for icon in Icon::ALL {
            for (size, points) in size_points() {
                for ppp in [1.0, 2.0] {
                    let px = pixels(points, ppp);
                    let image = rasterise(icon, set, px, size.stroke() * ppp, false);
                    assert_eq!(image.size, [px as usize; 2]);
                    let drawn = alpha(&image).filter(|a| *a > 0).count();
                    // Not empty, and not a filled square (Carbon's files hide a full-size rect with CSS).
                    assert!(drawn > 0, "{} in {} at {px} px is empty", icon.name(), set.id());
                    assert!(drawn < image.pixels.len() * 9 / 10, "{} in {} at {px} px fills its square", icon.name(), set.id());
                }
            }
        }
    }
}

#[test]
fn strokes_are_drawn_at_the_suite_width() {
    // Lucide's minus is one horizontal stroke: a column across it sums to the stroke width.
    for (px, stroke) in [(16, 1.5), (12, 1.25), (32, 3.0)] {
        let image = rasterise(Icon::Minus, IconSet::Lucide, px, stroke, false);
        let x = px as usize / 2;
        let coverage: f32 = (0..px as usize).map(|y| image.pixels[y * px as usize + x].a() as f32 / 255.0).sum();
        assert!((coverage - stroke).abs() < 0.1, "{px} px: stroke {coverage}, expected {stroke}");
    }
    assert_eq!(
        scale_stroke_widths(r#"<svg stroke-width="2"><path stroke-width="1"/></svg>"#, 1.125),
        r#"<svg stroke-width="2.250"><path stroke-width="1.125"/></svg>"#
    );
}

#[test]
fn the_second_request_hits_the_cache() {
    let ctx = Context::default();
    let first = texture(&ctx, Icon::Search, 16);
    let allocated = ctx.tex_manager().read().num_allocated();
    assert_eq!(texture(&ctx, Icon::Search, 16), first);
    assert_eq!(ctx.tex_manager().read().num_allocated(), allocated);
    assert_ne!(texture(&ctx, Icon::Search, 20), first);
    set_active(&ctx, IconSet::Carbon);
    assert_eq!(active(&ctx), IconSet::Carbon);
    assert_ne!(texture(&ctx, Icon::Search, 16), first);
}

#[test]
fn a_directional_icon_is_mirrored_right_to_left() {
    let plain = rasterise(Icon::ChevronLeft, IconSet::Lucide, 16, 1.5, false);
    let mirrored = rasterise(Icon::ChevronLeft, IconSet::Lucide, 16, 1.5, true);
    let flipped: Vec<u8> = (0..16).flat_map(|y| (0..16).rev().map(move |x| (y, x))).map(|(y, x)| plain.pixels[y * 16 + x].a()).collect();
    assert_eq!(alpha(&mirrored).collect::<Vec<_>>(), flipped);
    assert_ne!(alpha(&mirrored).collect::<Vec<_>>(), alpha(&plain).collect::<Vec<_>>());

    let ctx = Context::default();
    let (ltr_left, ltr_close) = (texture(&ctx, Icon::ChevronLeft, 16), texture(&ctx, Icon::Close, 16));
    direction::apply(&ctx, Direction::Rtl);
    assert_ne!(texture(&ctx, Icon::ChevronLeft, 16), ltr_left);
    assert_eq!(texture(&ctx, Icon::Close, 16), ltr_close);
}

/// The names the PhotoCraft button call sites need (DS-43 A8) that every set has a file for. The
/// variants make a renamed or dropped manifest entry a compile error; the names pin the spelling.
#[test]
fn every_photocraft_button_icon_resolves_in_every_set() {
    let wanted = [
        (Icon::Note, "note"),
        (Icon::Ban, "ban"),
        (Icon::SwapHorizontal, "swap_horizontal"),
        (Icon::NewLayer, "new_layer"),
        (Icon::NewFolder, "new_folder"),
        (Icon::Folder, "folder"),
        (Icon::Target, "target"),
        (Icon::Adjustments, "adjustments"),
        (Icon::Grid, "grid"),
        (Icon::GridSmall, "grid_small"),
        (Icon::Play, "play"),
        (Icon::AlignJustify, "align_justify"),
        (Icon::MarqueeRect, "marquee_rect"),
        (Icon::MarqueeEllipse, "marquee_ellipse"),
        (Icon::Home, "home"),
        (Icon::Scan, "scan"),
        (Icon::Ruler, "ruler"),
        (Icon::Stamp, "stamp"),
        (Icon::Spline, "spline"),
    ];
    for (icon, name) in wanted {
        assert_eq!(icon.name(), name);
        for set in IconSet::ALL {
            assert!(alpha(&rasterise(icon, set, 16, 1.5, false)).any(|a| a > 0), "{name} is blank in {set:?}");
        }
    }
}
