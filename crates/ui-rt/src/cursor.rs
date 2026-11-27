//! The pointer's look per window. Nodes name cursors in paint: for a hit shape
//! ([`crate::PaintCx::hit_cursor`]) or for themselves and their descendants
//! ([`crate::PaintCx::cursor`]). A window picks its cursor again only when what decides it
//! changed (the shape under the pointer, a press starting or ending, a drag's target, a repaint of
//! a node that names cursors), and the platform hears only a cursor that differs from the last one
//! sent.
//!
//! Order: a drag over the window shows its operation; a press shows the cursor of the shape it went
//! down on, else of the nearest node above that shape that names one, wherever the pointer goes;
//! otherwise the hovered shape's cursor, else the nearest hovered node's; else the default.

use crate::id::{NodeId, WindowId};
use crate::platform::Cursor;
use crate::runtime::Runtime;

impl Runtime {
    /// Picks `window`'s cursor when something that decides it changed, and queues it for the
    /// platform when it differs from the last one sent.
    pub(crate) fn refresh_cursor(&mut self, window: WindowId) {
        let Some(w) = self.windows.get_mut(&window) else { return };
        if !std::mem::take(&mut w.input.cursor_check) {
            return;
        }
        let cursor = self.pick_cursor(window);
        let Some(w) = self.windows.get_mut(&window) else { return };
        if w.input.cursor != cursor {
            w.input.cursor = cursor;
            self.cursor_requests.push((window, cursor));
        }
    }

    fn pick_cursor(&self, window: WindowId) -> Cursor {
        if let Some(c) = self.drag_cursor(window) {
            return c;
        }
        let Some(input) = self.windows.get(&window).map(|w| &w.input) else { return Cursor::Default };
        if let Some((node, shape)) = input.press_cursor_source() {
            return shape.or_else(|| self.node_cursor(node)).unwrap_or_default();
        }
        if let Some((node, shape)) = input.hover_cursor_source() {
            return shape.or_else(|| self.node_cursor(node)).unwrap_or_default();
        }
        Cursor::Default
    }

    /// The cursor of the nearest node from `node` up that names one.
    fn node_cursor(&self, node: NodeId) -> Option<Cursor> {
        let mut cur = Some(node);
        while let Some(id) = cur {
            let n = self.tree.nodes.get(&id)?;
            if n.cursor.is_some() {
                return n.cursor;
            }
            cur = n.parent;
        }
        None
    }

    /// Cursor changes since the last call; the driver hands them to the platform.
    pub(crate) fn take_cursor_requests(&mut self) -> Vec<(WindowId, Cursor)> {
        std::mem::take(&mut self.cursor_requests)
    }
}
