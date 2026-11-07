//! The cursor service (8.5): requests of a pass collect here, the end hook picks the winner and records it.

use crate::runtime::egui::{output, pass};
use crate::runtime::frame::PerViewport;
use craft_behavior::cursor::pick;
use craft_behavior::{Cursor, CursorIcon, CursorPriority, PaintedCursor};
use egui::Context;

#[derive(Default)]
struct State {
    requests: Vec<(Cursor, CursorPriority)>,
    record: Option<Cursor>,
}

/// Requests `cursor` for the running pass of this viewport.
pub(crate) fn request(ctx: &Context, cursor: Cursor, priority: CursorPriority) {
    PerViewport::<State>::with(ctx, pass::current_viewport(ctx), |state| state.requests.push((cursor, priority)));
}

/// The end hook: records the pass's winner. A system cursor goes out through the viewport's output; a painted
/// cursor hides the system one and is left for the overlay host to paint, except a badged system cursor, which
/// keeps its base and has only the badge painted.
pub(crate) fn end_pass(ctx: &Context) {
    let won = PerViewport::<State>::with(ctx, pass::current_viewport(ctx), |state| {
        let won = pick(&state.requests).cloned();
        state.requests.clear();
        state.record = won.clone();
        won
    });
    match won {
        Some(Cursor::System(icon)) => output::set_cursor(ctx, icon),
        Some(Cursor::Painted(PaintedCursor::Badged { base, .. })) => output::set_cursor(ctx, base),
        Some(_) => output::set_cursor(ctx, CursorIcon::None),
        None => {}
    }
}

/// The cursor the last pass of the running viewport settled on; `None` is the default arrow.
pub fn current(ctx: &Context) -> Option<Cursor> {
    PerViewport::<State>::with(ctx, pass::current_viewport(ctx), |state| state.record.clone())
}
