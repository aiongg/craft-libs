//! The cursor service (docs/contract-rt.md section 10, "Cursor"). A component names the cursor over
//! its own shape in `Component::cursor` and over its parts in `PartBuilder::cursor`; the node keeps
//! them and records them with its hit shapes. ui-rt picks the window's cursor from the hovered (or
//! pressed) shape and sends it to the platform only when it differs from the last one, so hovering
//! a node with another cursor runs no hook and no per-frame cursor pass.

use craft_ui_rt::{HitShape, PaintCx, Sense};

use super::node::Kept;

/// Records the node's own hit shape: its sense and, when the component names one, its cursor. A node
/// that only names a cursor senses hover, so the pointer over it finds the shape.
pub(crate) fn record_own_shape(kept: &Kept, cx: &mut PaintCx<'_>) {
    let bounds = cx.bounds();
    match kept.cursor {
        Some(cursor) => cx.hit_cursor(HitShape::Rect(bounds), kept.sense | Sense::HOVER, 0, cursor),
        None if !kept.sense.is_empty() => cx.hit(HitShape::Rect(bounds), kept.sense, 0),
        None => {}
    }
}

/// The platform cursor for a component's cursor. A system cursor maps one to one. ui-rt has no painted
/// cursors yet (a pointer layer that draws them over the window), so a painted one shows the system
/// cursor nearest to it and a hidden one hides the pointer.
pub(crate) fn platform(cursor: &craft_behavior::Cursor) -> craft_ui_rt::Cursor {
    use craft_behavior::{Cursor, CursorIcon, PaintedCursor};
    let icon = match cursor {
        Cursor::System(icon) => *icon,
        Cursor::Painted(painted) => match painted {
            PaintedCursor::Hidden => return craft_ui_rt::Cursor::Hidden,
            PaintedCursor::Hand => CursorIcon::Grab,
            PaintedCursor::Zoom { out: true, .. } => CursorIcon::ZoomOut,
            PaintedCursor::Zoom { .. } => CursorIcon::ZoomIn,
            PaintedCursor::Badged { base, .. } => *base,
            _ => CursorIcon::Crosshair,
        },
    };
    craft_ui_rt::Cursor::Icon(icon)
}
