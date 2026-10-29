use egui::epaint::PathShape;
use egui::{Context, CursorIcon, Pos2, Shape, pos2, vec2};
use egui_kittest::Harness;
use kurbo::Point;

use super::{
    CanvasView, PointerInk, SelectionBadge, distort_cursor, selection_badge_beside_pointer, selection_cursor, skew_cursor, turn_cursor,
    turn_cursor_with,
};
use crate::theme::{Base, install};

/// Runs one frame of `add` and returns the cursor icon it left and the shapes it painted.
fn paint(add: impl Fn(&Context)) -> (CursorIcon, Vec<Shape>) {
    let h = Harness::builder().with_size([200.0, 200.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(|ui| add(ui.ctx()));
    let out = h.output();
    (out.platform_output.cursor_icon, out.shapes.iter().map(|s| s.shape.clone()).collect())
}

fn pass(add: impl Fn(&Context)) -> (CursorIcon, usize) {
    let (icon, shapes) = paint(add);
    (icon, shapes.iter().map(count).sum())
}

fn count(shape: &Shape) -> usize {
    match shape {
        Shape::Vec(v) => v.iter().map(count).sum(),
        _ => 1,
    }
}

fn paths(shape: &Shape, out: &mut Vec<PathShape>) {
    match shape {
        Shape::Vec(v) => v.iter().for_each(|s| paths(s, out)),
        Shape::Path(p) => out.push(p.clone()),
        _ => {}
    }
}

fn path_shapes(shapes: &[Shape]) -> Vec<PathShape> {
    let mut out = Vec::new();
    shapes.iter().for_each(|s| paths(s, &mut out));
    out
}

const AT: Pos2 = pos2(100.0, 100.0);

fn view() -> CanvasView {
    CanvasView::new(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 200.0)), Point::new(0.0, 0.0), 1.0)
}

#[test]
fn system_ink_leaves_the_pointer_and_painted_ink_hides_it() {
    let base = pass(|_| {}).1;
    let system = pass(|ctx| selection_cursor(ctx, AT, SelectionBadge::Add, PointerInk::System));
    let painted = pass(|ctx| selection_cursor(ctx, AT, SelectionBadge::Add, PointerInk::Painted));
    assert_eq!(system.0, CursorIcon::Crosshair);
    assert_eq!(painted.0, CursorIcon::None);
    // The painted crosshair is on top of the badge both draw.
    assert!(painted.1 > system.1 && system.1 > base, "{base} {system:?} {painted:?}");
    // New has no badge: the system pointer is the bare icon.
    assert_eq!(pass(|ctx| selection_cursor(ctx, AT, SelectionBadge::New, PointerInk::System)), (CursorIcon::Crosshair, base));

    // The rotate pointer hides the system one unless the host keeps it.
    let toward = vec2(1.0, 1.0);
    let old = pass(|ctx| turn_cursor(ctx, AT, toward));
    assert!(old.1 > base);
    assert_eq!(old, pass(|ctx| turn_cursor_with(ctx, AT, toward, PointerInk::Painted)));
    assert_eq!(old.0, CursorIcon::None);
    let kept = pass(|ctx| turn_cursor_with(ctx, AT, toward, PointerInk::System));
    assert_eq!((kept.0, kept.1), (CursorIcon::Default, old.1));

    // So do the distort and skew pointers.
    let dir = kurbo::Vec2::new(1.0, 1.0);
    for ink in [PointerInk::System, PointerInk::Painted] {
        let want = if ink == PointerInk::Painted { CursorIcon::None } else { CursorIcon::Default };
        assert_eq!(pass(|ctx| distort_cursor(ctx, AT, &view(), dir, ink)).0, want);
        assert_eq!(pass(|ctx| skew_cursor(ctx, AT, &view(), dir, ink)).0, want);
    }
}

#[test]
fn the_distort_halo_is_one_outline_over_the_unstroked_fills() {
    let (_, shapes) = paint(|ctx| distort_cursor(ctx, AT, &view(), kurbo::Vec2::new(1.0, 0.0), PointerInk::Painted));
    let all = path_shapes(&shapes);
    let transparent = egui::Color32::TRANSPARENT;
    let fills: Vec<_> = all.iter().filter(|p| p.fill != transparent).collect();
    assert_eq!(fills.len(), 2);
    assert!(fills.iter().all(|p| p.stroke.width == 0.0), "{fills:?}");
    let outlines: Vec<_> = all.iter().filter(|p| p.stroke.width > 0.0).collect();
    assert_eq!(outlines.len(), 1, "{all:?}");
    assert!(outlines[0].closed && outlines[0].points.len() == 4 && outlines[0].fill == transparent);
    assert_eq!(outlines[0].stroke.width, 1.5, "PhotoCraft's width");
    // Drawn after both fills, so it covers their edges as PhotoCraft's does.
    assert_eq!(all.iter().position(|p| p.stroke.width > 0.0), Some(all.len() - 1));
}

#[test]
fn the_skew_arrowheads_are_stroked_one_point_in_the_halo_and_the_shafts_are_halo_then_ink() {
    let (_, shapes) = paint(|ctx| skew_cursor(ctx, AT, &view(), kurbo::Vec2::new(1.0, 0.0), PointerInk::Painted));
    let all = path_shapes(&shapes);
    let heads: Vec<_> = all.iter().filter(|p| p.fill != egui::Color32::TRANSPARENT).collect();
    assert_eq!(heads.len(), 2);
    assert!(heads.iter().all(|p| p.stroke.width == 1.0 && p.points.len() == 3), "{heads:?}");
}

#[test]
fn the_badge_beside_the_system_pointer_sits_clear_of_the_arrow_and_leaves_the_icon() {
    let (icon, shapes) = paint(|ctx| selection_badge_beside_pointer(ctx, AT, SelectionBadge::Add));
    assert_eq!(icon, CursorIcon::Default);
    let centre = AT + vec2(30.0, 11.0);
    let lines: Vec<_> = shapes
        .iter()
        .flat_map(|s| match s {
            Shape::LineSegment { points, .. } => vec![*points],
            Shape::Vec(v) => v.iter().filter_map(|s| if let Shape::LineSegment { points, .. } = s { Some(*points) } else { None }).collect(),
            _ => Vec::new(),
        })
        .collect();
    assert_eq!(lines.len(), 4, "a halo and an ink line for each of the plus's two bars");
    assert!(lines.iter().all(|[a, b]| (a.to_vec2() + b.to_vec2()) / 2.0 == centre.to_vec2()));
}
