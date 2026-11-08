//! The cursor service: the highest priority wins and reaches the record and the viewport output.

use craft_behavior::{Cursor, CursorBadge, CursorIcon, CursorPriority, IconId, PaintedCursor, Sense};
use craft_ui::runtime::cursor::current;
use craft_ui::testing::pointer;
use craft_ui::testing::sensing::{ProbePart, ProbeRoot, show};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Shape, Ui, Vec2};

/// Skeleton: `frames_pointer::painted_cursor_request_wins_by_priority_and_reaches_the_cursor_record`, over the
/// spec's priority order.
fn brush_over(priority: CursorPriority) -> (Option<Cursor>, egui::CursorIcon) {
    let area = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(200.0, 100.0));
    let brush = ProbeRoot::new("brush", area, Sense::HOVER)
        .part(ProbePart::new(craft_behavior::PartKey::new("grip"), area, Sense::HOVER).cursor(CursorIcon::Grab))
        .requests(PaintedCursor::circle(8.0), priority);
    let mut show_frame = |ui: &mut Ui| {
        show(ui, std::slice::from_ref(&brush));
    };
    let mut frames = Frames::new();
    frames.idle(2, &mut show_frame);
    assert_eq!(current(&frames.ctx), None, "no request while the pointer is elsewhere");
    frames.frame(vec![pointer::moved(area.center())], &mut show_frame);
    frames.idle(1, &mut show_frame);
    (current(&frames.ctx), frames.cursor_icon())
}

#[trial]
fn the_cursor_request_with_the_higher_priority_wins() {
    let (record, system) = brush_over(CursorPriority::DropTarget);
    assert_eq!(record, Some(Cursor::Painted(PaintedCursor::circle(8.0))), "a drop-target cursor outranks the part's");
    assert_eq!(system, egui::CursorIcon::None, "a painted cursor hides the system one");
    let (record, system) = brush_over(CursorPriority::Tool);
    assert_eq!(record, Some(Cursor::System(CursorIcon::Grab)), "a tool cursor loses to the hovered part's");
    assert_eq!(system, egui::CursorIcon::Grab, "a system cursor goes out through the viewport output");
}

/// The shapes the ghost layer gains while the pointer rests in the area with `cursor` requested, the pointer, and
/// the system cursor that goes out.
fn painted(cursor: PaintedCursor) -> (Vec<Shape>, Pos2, egui::CursorIcon) {
    let area = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(200.0, 100.0));
    let run = |cursor: PaintedCursor| {
        let probe = ProbeRoot::new("tool", area, Sense::HOVER).requests(cursor, CursorPriority::Tool);
        let mut show_frame = |ui: &mut Ui| {
            show(ui, std::slice::from_ref(&probe));
        };
        let mut frames = Frames::new();
        frames.idle(2, &mut show_frame);
        frames.frame(vec![pointer::moved(area.center())], &mut show_frame);
        frames.idle(1, &mut show_frame);
        let shapes = frames.frame_shapes(&mut show_frame);
        (shapes, frames.cursor_icon())
    };
    let (baseline, _) = run(PaintedCursor::Hidden);
    let (shapes, system) = run(cursor);
    (shapes.into_iter().filter(|shape| !baseline.contains(shape)).collect(), area.center(), system)
}

fn bounds(shapes: &[Shape]) -> Rect {
    shapes.iter().map(Shape::visual_bounding_rect).fold(Rect::NOTHING, Rect::union)
}

#[trial]
fn every_painted_cursor_but_hidden_paints_at_the_pointer_and_hides_the_system_cursor() {
    let icon = IconId::new(craft_ui::primitives::icon_names()[0]);
    let cursors = [
        ("hand", PaintedCursor::Hand),
        ("zoom in", PaintedCursor::zoom(false)),
        ("zoom out", PaintedCursor::zoom(true)),
        ("outline", PaintedCursor::outline(vec![vec![Pos2::new(-6.0, -6.0), Pos2::new(6.0, -6.0), Pos2::new(0.0, 6.0)]])),
        ("tool", PaintedCursor::tool(icon.clone(), Vec2::new(4.0, 20.0), Some(CursorBadge::Add))),
        ("turn", PaintedCursor::turn(Vec2::new(1.0, 1.0))),
        ("distort", PaintedCursor::distort(Vec2::new(1.0, -1.0))),
        ("skew", PaintedCursor::skew(Vec2::new(1.0, 0.0))),
    ];
    for (name, cursor) in cursors {
        let (shapes, pointer, system) = painted(cursor);
        assert!(!shapes.is_empty(), "{name} paints shapes");
        assert!(bounds(&shapes).expand(1.0).contains(pointer), "{name} is painted at the pointer: {:?} {pointer:?}", bounds(&shapes));
        assert_eq!(system, egui::CursorIcon::None, "{name} hides the system cursor");
    }
    let (shapes, _, system) = painted(PaintedCursor::Hidden);
    assert!(shapes.is_empty(), "a hidden cursor paints nothing");
    assert_eq!(system, egui::CursorIcon::None, "and hides the system one");
}

#[trial]
fn a_badged_system_cursor_keeps_its_base_and_paints_only_the_badge() {
    let (shapes, pointer, system) = painted(PaintedCursor::badged(CursorIcon::Copy, CursorBadge::Copy));
    assert_eq!(system, egui::CursorIcon::Copy, "the system cursor is the base");
    assert!(!shapes.is_empty(), "the badge is painted");
    let badge = bounds(&shapes);
    assert!(badge.min.x > pointer.x && badge.min.y > pointer.y, "the badge sits below and after the pointer: {badge:?} {pointer:?}");
}

#[trial]
fn the_pen_badges_paint_beside_the_pointer_and_differ_from_each_other() {
    let painted_badge = |badge| {
        let (shapes, pointer, _) = painted(PaintedCursor::badged(CursorIcon::Default, badge));
        assert!(!shapes.is_empty(), "{badge:?} is painted");
        let area = bounds(&shapes);
        assert!(area.min.x > pointer.x && area.min.y > pointer.y, "{badge:?} sits below and after the pointer: {area:?} {pointer:?}");
        shapes.len()
    };
    assert_ne!(painted_badge(CursorBadge::Scissors), painted_badge(CursorBadge::Close), "a cut point and a close point look different");
}
