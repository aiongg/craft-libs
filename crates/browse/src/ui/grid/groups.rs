//! `TileGroup` headers, pinned while their group scrolls.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Rect, Response, Ui};

use super::{TileGroup, TileStyle};

/// Draws a group header in `rect`: title `BodyStrong` `tile.group.text` at the start, detail
/// `Caption` `tile.group.count` at the end, hover wash `state.hover` × 0.5; pinned copies on
/// `tile.group.pinned` with a `line.divider` hairline below. A `Heading` node named title and
/// detail. Senses clicks; not focusable.
pub(super) fn group_header(ui: &mut Ui, rect: Rect, group: &TileGroup, pinned: bool, style: &TileStyle) -> Response {
    // lightcraft: crates/ui-egui/src/panels/grid.rs:656 (`group_header`).
    todo!()
}

/// The rect of the pinned header: the header of the group whose rows cross the viewport's top,
/// pinned there, pushed up by the next group's header as it arrives. `None` when the group's own
/// header is in view.
pub(super) fn pinned_rect(headers: &[Rect], viewport: Rect, group_at_top: Option<usize>) -> Option<(usize, Rect)> {
    // lightcraft: crates/ui-egui/src/panels/grid.rs:436 (the current header sticks; the next pushes it up).
    todo!()
}
