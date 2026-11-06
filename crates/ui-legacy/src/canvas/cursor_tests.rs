//! The canvas cursor.

use std::cell::Cell;

use egui::{Context, CursorIcon, LayerId, Pos2, Shape, Vec2};
use egui_kittest::Harness;

use super::{CursorBadge, CursorHost, CursorOptions, CursorStyle, PaintingCursor, ToolCursor};
use crate::icons::Icon;
use crate::theme::{Base, current, install};

fn style(ctx: &Context) -> CursorStyle {
    CursorHost::style(&current(ctx))
}

/// What one pass left: the shapes it painted, nested ones flattened, and its bitmap pointer.
struct Pass {
    shapes: Vec<Shape>,
    image: Option<egui::CustomCursorImage>,
}

fn flatten(shape: &Shape, out: &mut Vec<Shape>) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten(s, out)),
        other => out.push(other.clone()),
    }
}

/// Runs `add` in a 200 point frame at `pixels_per_point`.
fn run(pixels_per_point: f32, add: impl Fn(&Context)) -> Pass {
    let harness = Harness::builder()
        .with_size([200.0, 200.0])
        .with_pixels_per_point(pixels_per_point)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui(|ui| add(ui.ctx()));
    let output = harness.output();
    let mut shapes = Vec::new();
    output.shapes.iter().for_each(|clipped| flatten(&clipped.shape, &mut shapes));
    Pass { shapes, image: output.platform_output.cursor_image.clone() }
}

fn segments(shapes: &[Shape]) -> usize {
    shapes.iter().filter(|s| matches!(s, Shape::LineSegment { .. })).count()
}

// With Caps Lock a brush tip cursor becomes the precise crosshair.
#[test]
fn caps_lock_makes_painting_cursors_precise() {
    let prefs = CursorOptions::default();
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let style = &style(&ctx);
    let tip = ToolCursor::brush(style, &prefs, 10.0, 12.0, Icon::Brush, false);
    assert!(matches!(tip, ToolCursor::Circle { radius, centre: false } if radius == 10.0));
    let precise = ToolCursor::brush(style, &prefs, 10.0, 12.0, Icon::Brush, true);
    assert!(matches!(precise, ToolCursor::Crosshair { .. }));
    // The preference off leaves the tip alone.
    let off = CursorOptions { caps_lock_precise: false, ..prefs };
    assert!(matches!(ToolCursor::brush(style, &off, 10.0, 12.0, Icon::Brush, true), ToolCursor::Circle { .. }));
    // The full-size choice uses the full reach.
    let full = CursorOptions { painting: PaintingCursor::FullSizeBrushTip, ..prefs };
    assert!(matches!(ToolCursor::brush(style, &full, 10.0, 12.0, Icon::Brush, false), ToolCursor::Circle { radius, .. } if radius == 12.0));
    // Other tools: Caps Lock swaps the icon for the crosshair, and Precise swaps it back.
    let icon = |prefs: &CursorOptions, caps| ToolCursor::tool(style, prefs, Icon::Move, Vec2::ZERO, None, caps);
    assert!(matches!(icon(&prefs, false), ToolCursor::Tool { .. }));
    assert!(matches!(icon(&prefs, true), ToolCursor::Crosshair { .. }));
    let other = CursorOptions { other_precise: true, ..prefs };
    assert!(matches!(icon(&other, false), ToolCursor::Crosshair { .. }));
    assert!(matches!(icon(&other, true), ToolCursor::Tool { .. }));
}

// A 10 px brush at 200 % paints a 20-point outline; the OS pointer is hidden.
#[test]
fn the_brush_tip_draws_at_its_screen_size() {
    let cursor = ToolCursor::Circle { radius: 10.0, centre: false };
    let icon = Cell::new(None);
    let pass =
        run(1.0, |ctx| icon.set(Some(CursorHost::set(ctx, &ctx.layer_painter(LayerId::background()), Pos2::new(50.0, 50.0), &cursor, &style(ctx)))));
    assert_eq!(icon.get(), Some(CursorIcon::None));
    let image = pass.image.expect("a bitmap");
    // The ring and its halo fit the bitmap; the hotspot is its middle pixel.
    let [width, _] = image.size;
    let hot = usize::from(image.hotspot[0]);
    assert_eq!(usize::from(width), 2 * hot + 1);
    let alpha = |dx: isize, dy: isize| image.rgba[((hot as isize + dy) as usize * usize::from(width) + (hot as isize + dx) as usize) * 4 + 3];
    assert!(alpha(10, 0) > 200, "the ring is 10 points from the hotspot");
    assert!(alpha(0, -10) > 200);
    assert_eq!(alpha(5, 0), 0, "inside the ring is clear");
    assert_eq!(alpha(0, 0), 0);
    assert!(hot < 16, "the bitmap is no bigger than the ring needs");
}

// A badge (plus, minus) is painted beside the tool icon.
#[test]
fn a_tool_cursor_paints_its_badge() {
    let count = |badge: Option<CursorBadge>| {
        let cursor = ToolCursor::Tool { icon: Icon::Move, hotspot: Vec2::ZERO, badge };
        let pass = run(1.0, |ctx| {
            CursorHost::set(ctx, &ctx.layer_painter(LayerId::background()), Pos2::new(50.0, 50.0), &cursor, &style(ctx));
        });
        segments(&pass.shapes)
    };
    // Each glyph line is a halo line under an ink line.
    assert_eq!(count(None), 0);
    assert_eq!(count(Some(CursorBadge::Subtract)), 2);
    assert_eq!(count(Some(CursorBadge::Add)), 4);
}

// A badged circle paints its glyph centred on the pointer, over the circle bitmap.
#[test]
fn a_badged_circle_paints_its_badge_at_the_pointer() {
    let pointer = Pos2::new(50.0, 50.0);
    let pass = |badge: CursorBadge| {
        let cursor = ToolCursor::Badged { cursor: Box::new(ToolCursor::Circle { radius: 10.0, centre: false }), badge };
        run(1.0, |ctx| {
            CursorHost::set(ctx, &ctx.layer_painter(LayerId::background()), pointer, &cursor, &style(ctx));
        })
    };
    let add = pass(CursorBadge::Add);
    assert!(add.image.is_some(), "the circle is still a bitmap pointer");
    assert_eq!(segments(&add.shapes), 4);
    assert_eq!(segments(&pass(CursorBadge::Subtract).shapes), 2);
    for shape in &add.shapes {
        if let Shape::LineSegment { points, .. } = shape {
            let middle = points[0].to_vec2() / 2.0 + points[1].to_vec2() / 2.0;
            assert!((middle - pointer.to_vec2()).length() < 1e-3, "{points:?}");
        }
    }
}

// On Wayland the host falls back to the system cursor for a bitmap one.
#[test]
fn without_bitmap_cursors_the_system_cursor_stands_in() {
    let (hand, zoom, circle, supported) = (Cell::new(None), Cell::new(None), Cell::new(None), Cell::new(true));
    let pass = run(1.25, |ctx| {
        CursorHost::set_wayland(ctx, true);
        supported.set(CursorHost::bitmaps_supported(ctx));
        let painter = ctx.layer_painter(LayerId::background());
        let (at, style) = (Pos2::new(50.0, 50.0), style(ctx));
        hand.set(Some(CursorHost::set(ctx, &painter, at, &ToolCursor::Hand { closed: true }, &style)));
        zoom.set(Some(CursorHost::set(ctx, &painter, at, &ToolCursor::Zoom { out: true }, &style)));
        circle.set(Some(CursorHost::set(ctx, &painter, at, &ToolCursor::Circle { radius: 10.0, centre: false }, &style)));
    });
    assert!(!supported.get());
    // Windows draws these two itself; elsewhere the system's pointers are right.
    if !cfg!(target_os = "windows") {
        assert_eq!(hand.get(), Some(CursorIcon::Grabbing));
        assert_eq!(zoom.get(), Some(CursorIcon::ZoomOut));
    }
    // The outline is painted at its full size instead of uploaded.
    assert_eq!(circle.get(), Some(CursorIcon::None));
    assert!(pass.image.is_none());
    assert!(pass.shapes.iter().any(|s| matches!(s, Shape::Circle(c) if c.radius == 10.0)));
}
