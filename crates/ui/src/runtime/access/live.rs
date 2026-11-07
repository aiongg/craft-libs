//! Announcements: transient text goes through `announce`, flushed at the end of each pass as one status node and
//! returned in `FrameOutput` for tests.

use super::Politeness;
use crate::runtime::egui::{access as adapter, pass};
use crate::runtime::frame::{self, PerViewport};
use crate::runtime::memory::global;
use egui::{Context, ViewportId};

/// What the frame has announced so far in one viewport.
#[derive(Default)]
struct Announced(Vec<(String, Politeness)>);

/// Announces `text`; for code without a context. Components call `cx.announce`.
pub fn announce(ctx: &Context, text: impl Into<String>, politeness: Politeness) {
    let text = text.into();
    frame::announce(ctx, text.clone());
    let viewport = pass::current_viewport(ctx);
    PerViewport::<Announced>::with(ctx, viewport, |announced| announced.0.push((text, politeness)));
}

/// The begin hook: the first pass of a frame starts with no announcements; a discarded pass's replay keeps them,
/// so the pass that is shown still carries the status node.
pub(crate) fn begin_pass(ctx: &Context) {
    let viewport = pass::current_viewport(ctx);
    if pass::pass_nr(ctx, viewport) == 0 {
        PerViewport::<Announced>::with(ctx, viewport, |announced| announced.0.clear());
    }
}

/// The end hook: the status node of the viewport, one live node holding the frame's announcements.
pub(crate) fn end_pass(ctx: &Context) {
    let viewport = pass::current_viewport(ctx);
    let texts = PerViewport::<Announced>::with(ctx, viewport, |announced| announced.0.clone());
    if texts.is_empty() {
        return;
    }
    let live = if texts.iter().any(|(_, politeness)| *politeness == Politeness::Assertive) { accesskit::Live::Assertive } else { accesskit::Live::Polite };
    let text = texts.iter().map(|(text, _)| text.as_str()).collect::<Vec<_>>().join(". ");
    let id = status_id(viewport);
    adapter::with_node(ctx, id, |node| {
        node.set_role(accesskit::Role::Status);
        node.set_live(live);
        node.set_live_atomic();
        node.set_label(text);
    });
}

fn status_id(viewport: ViewportId) -> egui::Id {
    global("access.status", viewport)
}

