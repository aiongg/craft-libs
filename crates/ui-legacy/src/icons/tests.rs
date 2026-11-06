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

/// The names the PhotoCraft button call sites need (DS-43 A8) (the base set draws all of them; a pack that lacks one falls back to it). The
/// variants make a renamed or dropped manifest entry a compile error; the names pin the spelling.
#[test]
fn every_photocraft_button_icon_resolves_in_the_base_set() {
    let wanted = [
        (Icon::Note, "note"),
        (Icon::Ban, "ban"),
        (Icon::SwapHorizontal, "swap_horizontal"),
        (Icon::SwapVertical, "swap_vertical"),
        (Icon::SplitCross, "split_cross"),
        (Icon::SplitVertical, "split_vertical"),
        (Icon::SplitHorizontal, "split_horizontal"),
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
        (Icon::Sparkles, "sparkles"),
        (Icon::TextCursor, "text_cursor"),
        (Icon::OrientationPortrait, "orientation_portrait"),
        (Icon::OrientationLandscape, "orientation_landscape"),
    ];
    for (icon, name) in wanted {
        assert_eq!(icon.name(), name);
        assert!(icon.drawn_in(IconSet::BASE));
        assert!(alpha(&rasterise(icon, IconSet::BASE, 16, 1.5, false)).any(|a| a > 0), "{name} is blank in the base set");
    }
}

/// The preset browser's view switch and new-group button (DS-44).
#[test]
fn the_preset_browser_icons_are_drawn_in_every_set() {
    for (icon, name) in [(Icon::LayoutGrid, "layout_grid"), (Icon::List, "list"), (Icon::NewFolder, "new_folder"), (Icon::Folder, "folder")] {
        assert_eq!(icon.name(), name);
        for set in IconSet::ALL {
            assert!(icon.drawn_in(set), "{name} in {}", set.id());
        }
    }
}

#[test]
fn a_pack_without_an_icon_draws_the_base_set_file() {
    let missing: Vec<(Icon, IconSet)> = Icon::ALL.iter().flat_map(|i| IconSet::ALL.map(|s| (*i, s))).filter(|(i, s)| !i.drawn_in(*s)).collect();
    assert!(!missing.is_empty(), "the manifest has no name a pack lacks");
    for (icon, set) in missing {
        assert_ne!(set, IconSet::BASE);
        assert_eq!(icon.svg(set), icon.svg(IconSet::BASE), "{} in {}", icon.name(), set.id());
        // The base file is a fill drawing, so a stroke pack does not rewrite its widths.
        assert_eq!(rasterise(icon, set, 16, 3.0, false), rasterise(icon, IconSet::BASE, 16, 1.5, false), "{} in {}", icon.name(), set.id());
    }
}

/// The tile grid's Picked flag and the relink dialog's Update (LC45).
#[test]
fn the_flag_and_refresh_icons_are_drawn_in_every_set() {
    for (icon, name) in [(Icon::Flag, "flag"), (Icon::Refresh, "refresh")] {
        assert_eq!(icon.name(), name);
        assert!(!icon.directional(), "{name}");
        for set in IconSet::ALL {
            assert!(icon.drawn_in(set), "{name} in {}", set.id());
        }
    }
}

/// The stroke editor's align, cap and join segments, the connector bar's routing, the Brush Settings footer (LC92) and Liquify's Twirl
/// Counterclockwise and Push Left (LC93). Only the glyphs that stay put in right-to-left are non-directional: the canvas does not mirror.
#[test]
fn the_stroke_routing_footer_and_liquify_icons_resolve() {
    let fixed = [
        (Icon::StrokeAlignInside, "stroke_align_inside"),
        (Icon::StrokeAlignCenter, "stroke_align_center"),
        (Icon::StrokeAlignOutside, "stroke_align_outside"),
        (Icon::StrokeCapButt, "stroke_cap_butt"),
        (Icon::StrokeCapRound, "stroke_cap_round"),
        (Icon::StrokeCapSquare, "stroke_cap_square"),
        (Icon::StrokeJoinMiter, "stroke_join_miter"),
        (Icon::StrokeJoinRound, "stroke_join_round"),
        (Icon::StrokeJoinBevel, "stroke_join_bevel"),
        (Icon::RouteStraight, "route_straight"),
        (Icon::RouteElbow, "route_elbow"),
        (Icon::RouteCurved, "route_curved"),
        (Icon::SquarePlus, "square_plus"),
        (Icon::RotateCcw, "rotate_ccw"),
        (Icon::ArrowLeftFixed, "arrow_left_fixed"),
    ];
    for (icon, name) in fixed {
        assert_eq!(icon.name(), name);
        assert!(!icon.directional(), "{name}");
        assert!(icon.drawn_in(IconSet::BASE), "{name}");
    }
    assert_eq!(Icon::Undo2.name(), "undo_2");
    assert!(Icon::Undo2.directional());
}

#[test]
fn liquify_and_footer_icons_are_drawn_in_every_set() {
    for icon in [Icon::SquarePlus, Icon::Undo2, Icon::RotateCcw, Icon::ArrowLeftFixed] {
        for set in IconSet::ALL {
            assert!(icon.drawn_in(set), "{} in {}", icon.name(), set.id());
        }
    }
}

/// Step 6b's timeline, review, sheet and hyperlink glyphs: each resolves, is drawn in the base set, and stays put in right-to-left.
#[test]
fn the_step_6b_icons_resolve_and_the_packs_that_have_them_draw_their_own() {
    let wanted = [
        (Icon::TrackMute, "track_mute"),
        (Icon::Solo, "solo"),
        (Icon::SyncLock, "sync_lock"),
        (Icon::Snap, "snap"),
        (Icon::LinkedSelection, "linked_selection"),
        (Icon::MarkIn, "mark_in"),
        (Icon::MarkOut, "mark_out"),
        (Icon::StepBack, "step_back"),
        (Icon::StepForward, "step_forward"),
        (Icon::InsertClip, "insert_clip"),
        (Icon::OverwriteClip, "overwrite_clip"),
        (Icon::LiftClip, "lift_clip"),
        (Icon::ExtractClip, "extract_clip"),
        (Icon::KeyframeLinear, "keyframe_linear"),
        (Icon::KeyframeBezier, "keyframe_bezier"),
        (Icon::KeyframeAutoBezier, "keyframe_auto_bezier"),
        (Icon::KeyframeHold, "keyframe_hold"),
        (Icon::KeyframeEaseIn, "keyframe_ease_in"),
        (Icon::KeyframeEaseOut, "keyframe_ease_out"),
        (Icon::KeyLinear, "key_linear"),
        (Icon::KeyLinearFilled, "key_linear_filled"),
        (Icon::KeyHold, "key_hold"),
        (Icon::KeyBezier, "key_bezier"),
        (Icon::KeyAutoBezier, "key_auto_bezier"),
        (Icon::KeyContinuousBezier, "key_continuous_bezier"),
        (Icon::KeyEaseIn, "key_ease_in"),
        (Icon::KeyEaseOut, "key_ease_out"),
        (Icon::KeyframeEasyEase, "keyframe_easy_ease"),
        (Icon::ThumbsUp, "thumbs_up"),
        (Icon::ThumbsDown, "thumbs_down"),
        (Icon::XCircle, "x_circle"),
        (Icon::WarningDiamond, "warning_diamond"),
        (Icon::SortAscending, "sort_ascending"),
        (Icon::SortDescending, "sort_descending"),
        (Icon::Globe, "globe"),
        (Icon::Bookmark, "bookmark"),
        (Icon::Mail, "mail"),
    ];
    for (icon, name) in wanted {
        assert_eq!(icon.name(), name);
        assert!(!icon.directional(), "{name}");
        for set in IconSet::ALL {
            assert!(alpha(&rasterise(icon, set, 16, 1.5, false)).any(|a| a > 0), "{name} is blank in {}", set.id());
        }
    }
    // Lucide, Carbon and Tabler have their own mute speaker and thumbs.
    for icon in [Icon::TrackMute, Icon::ThumbsUp, Icon::ThumbsDown, Icon::SortAscending] {
        for set in [IconSet::Lucide, IconSet::Carbon, IconSet::Tabler] {
            assert!(icon.drawn_in(set), "{} in {}", icon.name(), set.id());
        }
    }
    // The sort glyphs differ in every set.
    for set in IconSet::ALL {
        assert_ne!(Icon::SortAscending.svg(set), Icon::SortDescending.svg(set), "{}", set.id());
    }
}
