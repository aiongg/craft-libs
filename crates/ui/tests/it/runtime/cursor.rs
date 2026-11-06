//! The cursor service: the highest priority wins and reaches the record and the viewport output.

use craft_behavior::{Cursor, CursorIcon, CursorPriority, PaintedCursor, Sense};
use craft_ui::runtime::cursor::current;
use craft_ui::testing::pointer;
use craft_ui::testing::sensing::{ProbePart, ProbeRoot, show};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Ui, Vec2};

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
