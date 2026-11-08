//! `Flyout`: the panel an icon of an iconic column opens beside the column.

use crate::contract::overlay::{Anchor, Dismiss, Place};
use crate::contract::{BodyCx, Overlay, OverlayUi, PopupHandle};
use craft_behavior::dock::DockEdge;
use craft_behavior::overlay::{Align, Side};
use egui::{Rect, Ui, Vec2};

#[cfg(feature = "testing")]
mod stories;

/// A popup beside an iconic column, on the side of the canvas, level with the icon that opened it. Escape and a
/// press outside close it; with Auto-Collapse Iconic Panels the focus leaving it closes it too. Its body gets the
/// rect the flyout's `size` takes, to show the group of the icon's panel in.
#[derive(crate::contract::Pattern)]
pub struct Flyout {
    edge: DockEdge,
    size: Vec2,
    auto_collapse: bool,
}

impl Flyout {
    /// A flyout of `size` for the column on the window edge `edge`, closing when the focus leaves it if
    /// `auto_collapse`.
    pub fn new(edge: DockEdge, size: Vec2, auto_collapse: bool) -> Self {
        Flyout { edge, size, auto_collapse }
    }

    /// The layer the flyout shows in: hung from `anchor` (the icon's part) on the side away from the window edge.
    /// The anchored placement mirrors the side in a right-to-left layout.
    pub fn overlay(&self, anchor: Anchor) -> Overlay {
        let side = match self.edge {
            DockEdge::End => Side::Start,
            _ => Side::End,
        };
        Overlay::popover().place(Place::anchored(anchor, side, Align::Start)).dismiss(Dismiss::new().focus_left(self.auto_collapse))
    }

    /// Shows the flyout of `popup` while it is open, hung from `anchor`; `None` while it is closed.
    pub fn show<R>(
        self,
        ui: &mut OverlayUi<'_>,
        popup: PopupHandle,
        anchor: Anchor,
        mut body: impl FnMut(&mut Ui, &mut BodyCx<'_>, Rect) -> R,
    ) -> Option<R> {
        let size = self.size;
        ui.show(popup, self.overlay(anchor), |ui, cx| {
            let rect = crate::layout::place_in_flow(ui, size);
            body(ui, cx, rect)
        })
    }
}

impl OverlayUi<'_> {
    /// Whether the layer of `popup` was dismissed in this pass (Escape, a press outside, the focus leaving it).
    pub(crate) fn dismissed(&self, popup: PopupHandle) -> bool {
        self.closed.iter().any(|(id, _)| *id == popup.id())
    }
}
