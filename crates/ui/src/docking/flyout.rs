//! `Flyout`: the panel an icon of an iconic column opens beside the column.

use crate::contract::overlay::Dismiss;
use crate::contract::{BodyCx, Overlay, OverlayUi, PopupHandle};
use craft_behavior::dock::DockEdge;
use egui::{Pos2, Rect, Ui, Vec2};

#[cfg(feature = "testing")]
mod stories;

/// A popup beside an iconic column, on the side of the canvas, level with the icon that opened it. Escape and a
/// press outside close it; with Auto-Collapse Iconic Panels the focus leaving it closes it too. Its body gets the
/// rect of the flyout, to show the group of the icon's panel in.
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

    /// The rect of the flyout of the icon `cell`: level with the icon, on the side of the column away from its
    /// window edge (`rtl` mirrors the edge).
    pub fn rect(&self, cell: Rect, rtl: bool) -> Rect {
        let beyond_end = (self.edge == DockEdge::Start) != rtl;
        let left = if beyond_end { cell.right() } else { cell.left() - self.size.x };
        Rect::from_min_size(Pos2::new(left, cell.top()), self.size)
    }

    /// The layer the flyout shows in.
    pub fn overlay(&self, cell: Rect, rtl: bool) -> Overlay {
        Overlay::popover().placed(self.rect(cell, rtl)).dismiss(Dismiss::new().focus_left(self.auto_collapse))
    }

    /// Shows the flyout of `popup` beside the icon `cell` while the popup is open; `None` while it is closed.
    pub fn show<R>(
        self,
        ui: &mut OverlayUi<'_>,
        popup: PopupHandle,
        cell: Rect,
        rtl: bool,
        mut body: impl FnMut(&mut Ui, &mut BodyCx<'_>, Rect) -> R,
    ) -> Option<R> {
        ui.show(popup, self.overlay(cell, rtl), |ui, cx| {
            let rect = cx.rect;
            body(ui, cx, rect)
        })
    }
}
