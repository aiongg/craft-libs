//! Tests of the brush cursor and the tip pose.

use egui::{Context, Rect as ScreenRect, Vec2, pos2};
use kurbo::{Point, Rect};

use super::{BrushCursor, BrushStyle, BrushTip, BrushTipOutline, TipPose, TipRings};
use crate::canvas::{CanvasView, CursorOptions, PaintingCursor, ToolCursor};
use crate::icons::Icon;
use crate::theme::{Base, current, install};

fn style() -> BrushStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    BrushStyle::of(&current(&ctx))
}

fn view(zoom: f64) -> CanvasView {
    CanvasView::new(ScreenRect::from_min_size(pos2(0.0, 0.0), Vec2::splat(400.0)), Point::ZERO, zoom)
}

fn prefs() -> CursorOptions {
    CursorOptions::default()
}

fn rings_of(cursor: &ToolCursor) -> &[Vec<Vec2>] {
    match cursor {
        ToolCursor::Outline { rings, .. } => rings,
        other => panic!("expected an outline, got {other:?}"),
    }
}

fn is_finite(cursor: &ToolCursor) -> bool {
    match cursor {
        ToolCursor::Circle { radius, .. } => radius.is_finite(),
        ToolCursor::Outline { rings, .. } => rings.iter().flatten().all(|p| p.x.is_finite() && p.y.is_finite()),
        ToolCursor::Crosshair { length, gap } => length.is_finite() && gap.is_finite(),
        _ => true,
    }
}

fn star() -> TipRings {
    TipRings::new(vec![vec![[1.0, 0.0], [0.0, 1.0], [-1.0, 0.0], [0.0, -1.0]]])
}

fn traced(diameter: f64) -> BrushTip {
    BrushTip { diameter, outline: BrushTipOutline::Traced { normal: star(), full: star() } }
}

// With `caps_lock_precise` on and Caps Lock down, every painting preference gives the 6 pt crosshair.
#[test]
fn caps_lock_gives_the_precise_crosshair() {
    let (style, view, tip) = (style(), view(1.0), BrushTip::round(100.0, 40.0));
    let painting = [PaintingCursor::Standard, PaintingCursor::Precise, PaintingCursor::NormalBrushTip, PaintingCursor::FullSizeBrushTip];
    for painting in painting {
        let prefs = CursorOptions { painting, show_only_crosshair_while_painting: true, ..prefs() };
        let cursor = BrushCursor::new(&tip).centre(true).painting(true).cursor(&style, &prefs, &view, true);
        assert!(matches!(cursor, ToolCursor::Crosshair { length, gap } if length == style.cursor.precise_reach && gap == 0.0), "{painting:?}");
    }
    let off = CursorOptions { caps_lock_precise: false, ..prefs() };
    assert!(matches!(BrushCursor::new(&tip).cursor(&style, &off, &view, true), ToolCursor::Circle { .. }));
}

// Standard returns the tool icon at its centre; Precise the crosshair.
#[test]
fn standard_gives_the_tool_icon_and_precise_the_crosshair() {
    let (style, view, tip) = (style(), view(1.0), BrushTip::round(100.0, 40.0));
    let standard = CursorOptions { painting: PaintingCursor::Standard, ..prefs() };
    let cursor = BrushCursor::new(&tip).icon(Icon::Move).cursor(&style, &standard, &view, false);
    assert!(matches!(cursor, ToolCursor::Tool { icon: Icon::Move, hotspot, .. } if hotspot == Vec2::splat(12.0)));
    let precise = CursorOptions { painting: PaintingCursor::Precise, ..prefs() };
    assert!(matches!(BrushCursor::new(&tip).cursor(&style, &precise, &view, false), ToolCursor::Crosshair { .. }));
}

// Show Only Crosshair While Painting swaps the outline for the crosshair while painting, not before.
#[test]
fn crosshair_only_while_painting() {
    let (style, view, tip) = (style(), view(1.0), BrushTip::round(100.0, 40.0));
    let prefs = CursorOptions { show_only_crosshair_while_painting: true, ..prefs() };
    assert!(matches!(BrushCursor::new(&tip).cursor(&style, &prefs, &view, false), ToolCursor::Circle { .. }));
    let cursor = BrushCursor::new(&tip).painting(true).cursor(&style, &prefs, &view, false);
    assert!(matches!(cursor, ToolCursor::Crosshair { length, gap } if length == style.cursor.precise_reach && gap == 0.0));
    // The preference off keeps the outline while painting.
    assert!(matches!(BrushCursor::new(&tip).painting(true).cursor(&style, &self::prefs(), &view, false), ToolCursor::Circle { .. }));
}

// A round tip gives `normal_radius` or `full_radius` times the zoom as the circle's radius.
#[test]
fn normal_and_full_size_pick_the_two_radii() {
    let (style, view, tip) = (style(), view(2.0), BrushTip::round(100.0, 30.0));
    let radius = |painting| match BrushCursor::new(&tip).cursor(&style, &CursorOptions { painting, ..prefs() }, &view, false) {
        ToolCursor::Circle { radius, .. } => radius,
        other => panic!("{other:?}"),
    };
    assert_eq!(radius(PaintingCursor::NormalBrushTip), 60.0);
    assert_eq!(radius(PaintingCursor::FullSizeBrushTip), 100.0);
}

// Traced rings map through the view like `to_screen` (PhotoCraft's `tip_axes_follow_the_view_like_to_screen`, `the_outline_turns_and_flips_with_the_view`).
#[test]
fn traced_rings_follow_zoom_rotation_flip_and_aspect() {
    let (style, tip) = (style(), traced(20.0));
    let mut view = view(3.0);
    view.rotation = 30.0;
    view.flip_x = true;
    view.aspect = 1.5;
    let cursor = BrushCursor::new(&tip).cursor(&style, &prefs(), &view, false);
    let origin = view.to_screen(Point::ZERO);
    let expected: Vec<Vec2> =
        [[1.0, 0.0], [0.0, 1.0], [-1.0, 0.0], [0.0, -1.0]].iter().map(|p| view.to_screen(Point::new(p[0] * 10.0, p[1] * 10.0)) - origin).collect();
    let ring = &rings_of(&cursor)[0];
    assert_eq!(ring.len(), expected.len());
    for (got, want) in ring.iter().zip(&expected) {
        assert!((*got - *want).length() < 1e-3, "{got:?} vs {want:?}");
    }
}

// The pose turns the rings before the view and the shared rings are not rebuilt (PhotoCraft's `pen_pose_rotation_precedes_the_view_and_reuses_the_tip_cache`).
#[test]
fn pen_pose_precedes_the_view() {
    let style = style();
    let tip = traced(20.0);
    let BrushTipOutline::Traced { normal, .. } = &tip.outline else { panic!("a traced tip") };
    let shared = normal.clone();
    let mut view = view(2.0);
    view.rotation = 90.0;
    let pose = TipPose::pen(90.0, None);
    let cursor = BrushCursor::new(&tip).pose(pose).cursor(&style, &prefs(), &view, false);
    // The ring point (1, 0) turns 90 degrees clockwise to document (0, 1)... in document y-down
    // terms: x = (cos, -sin) gives (0, -1) for the x axis; the view then turns it 90 degrees.
    let doc = Vec2::new(0.0, -10.0);
    let want = view.map_vec(kurbo::Vec2::new(f64::from(doc.x), f64::from(doc.y)));
    let got = rings_of(&cursor)[0][0];
    assert!((got - want).length() < 1e-3, "{got:?} vs {want:?}");
    assert!(normal.same(&shared), "the app's rings stay shared");
}

// `TipPose::pen` with a tilt shortens the axis along the tilt by its cosine (PhotoCraft's `brush_projection_shortens_the_tilt_axis`).
#[test]
fn projection_shortens_the_tilt_axis() {
    let upright = TipPose::pen(0.0, None);
    assert_eq!(upright, TipPose::upright());
    let tilted = TipPose::pen(0.0, Some([60.0, 0.0]));
    assert!((tilted.x_axis.x - 0.5).abs() < 1e-9, "{:?}", tilted.x_axis);
    assert!((tilted.y_axis.y - 1.0).abs() < 1e-9);
    // A near-flat pen keeps at least 5 %; a zero or non-finite tilt leaves the step out.
    assert!((TipPose::pen(0.0, Some([90.0, 0.0])).x_axis.x - 0.05).abs() < 1e-9);
    assert_eq!(TipPose::pen(0.0, Some([0.0, 0.0])), TipPose::upright());
    assert_eq!(TipPose::pen(0.0, Some([f32::NAN, 5.0])), TipPose::upright());
}

// Unequal or skewed axes turn a round tip into an outline ring with the axes' lengths.
#[test]
fn round_tip_becomes_an_ellipse_under_a_skewed_pose() {
    let (style, tip) = (style(), BrushTip::round(100.0, 40.0));
    let pose = TipPose::pen(0.0, Some([60.0, 0.0]));
    let cursor = BrushCursor::new(&tip).pose(pose).cursor(&style, &prefs(), &view(1.0), false);
    let ring = &rings_of(&cursor)[0];
    // Semi-axes 40 * 0.5 = 20 along x and 40 along y.
    let max_x = ring.iter().map(|p| p.x.abs()).fold(0.0, f32::max);
    let max_y = ring.iter().map(|p| p.y.abs()).fold(0.0, f32::max);
    assert!((max_x - 20.0).abs() < 1e-3 && (max_y - 40.0).abs() < 1e-3, "{max_x} {max_y}");
    // A pixel aspect does the same to an upright pose.
    let mut wide = view(1.0);
    wide.aspect = 2.0;
    assert!(matches!(BrushCursor::new(&tip).cursor(&style, &prefs(), &wide, false), ToolCursor::Outline { .. }));
}

// An outline under `brush.tip_min_radius` on screen gives the 4 pt crosshair with no gap.
#[test]
fn tiny_tips_show_a_crosshair() {
    let style = style();
    let small = BrushTip::round(100.0, 40.0);
    let cursor = BrushCursor::new(&small).cursor(&style, &prefs(), &view(0.05), false);
    assert!(matches!(cursor, ToolCursor::Crosshair { length, gap } if length == style.tip_min_crosshair && gap == 0.0), "{cursor:?}");
    let cursor = BrushCursor::new(&traced(2.0)).cursor(&style, &prefs(), &view(1.0), false);
    assert!(matches!(cursor, ToolCursor::Crosshair { .. }));
    // At the threshold the outline shows.
    let at = BrushCursor::new(&BrushTip::round(100.0, 3.5)).cursor(&style, &prefs(), &view(1.0), false);
    assert!(matches!(at, ToolCursor::Circle { .. }));
}

// NaN, infinite, negative and zero sizes never panic and never give a non-finite ring.
#[test]
fn hostile_sizes_give_finite_cursors() {
    let style = style();
    let bad = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -5.0, 0.0];
    let mut views = vec![view(1.0), view(f64::NAN), view(0.0)];
    views[1].aspect = f64::INFINITY;
    for v in &views {
        for &size in &bad {
            let tips = [
                BrushTip::round(size, size),
                BrushTip { diameter: size, outline: BrushTipOutline::Round { normal_radius: size, full_radius: 10.0 } },
                traced(size),
                BrushTip { diameter: 10.0, outline: BrushTipOutline::Pixels { cell: Rect::new(size, size, size, size), round: false } },
            ];
            for tip in &tips {
                for painting in [PaintingCursor::NormalBrushTip, PaintingCursor::FullSizeBrushTip] {
                    let options = CursorOptions { painting, ..prefs() };
                    for inner in [None, Some(size)] {
                        let mut cursor = BrushCursor::new(tip).pose(TipPose::pen(size, Some([size as f32, 1.0])));
                        if let Some(r) = inner {
                            cursor = cursor.inner_ring(r);
                        }
                        assert!(is_finite(&cursor.cursor(&style, &options, v, false)), "{tip:?}");
                    }
                }
            }
        }
    }
}

// `inner_ring` adds a second mapped ring (LightCraft's feather core).
#[test]
fn inner_ring_adds_a_second_ring() {
    let (style, tip) = (style(), BrushTip::round(100.0, 40.0));
    let cursor = BrushCursor::new(&tip).inner_ring(20.0).cursor(&style, &prefs(), &view(2.0), false);
    let rings = rings_of(&cursor);
    assert_eq!(rings.len(), 2);
    let reach = |ring: &Vec<Vec2>| ring.iter().map(|p| p.length()).fold(0.0, f32::max);
    assert!((reach(&rings[0]) - 80.0).abs() < 1e-3 && (reach(&rings[1]) - 40.0).abs() < 1e-3);
}

// A Pixels tip gives `Hidden`, or the 4 pt crosshair under 6 pt or with `centre`.
#[test]
fn pixel_tips_hide_the_pointer_and_small_cells_show_a_crosshair() {
    let style = style();
    let pixels = |side: f64| BrushTip { diameter: side, outline: BrushTipOutline::Pixels { cell: Rect::new(0.0, 0.0, side, side), round: false } };
    let cursor = |tip: &BrushTip, centre: bool, zoom: f64| BrushCursor::new(tip).centre(centre).cursor(&style, &prefs(), &view(zoom), false);
    assert!(matches!(cursor(&pixels(10.0), false, 1.0), ToolCursor::Hidden));
    assert!(matches!(cursor(&pixels(10.0), true, 1.0), ToolCursor::Crosshair { length, gap } if length == style.tip_min_crosshair && gap == 0.0));
    assert!(matches!(cursor(&pixels(2.0), false, 1.0), ToolCursor::Crosshair { .. }));
    assert!(matches!(cursor(&pixels(2.0), false, 4.0), ToolCursor::Hidden));
}
