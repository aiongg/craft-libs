//! `PaintSlot`: a place in a node's paint order, reserved while the component paints and filled later by code that
//! holds no painter (docs/contract-rt.md, "Decided (PaintSlot)"). The node's display list keeps an `Item::Slot`
//! there; ui-rt keeps the slot's items beside the node's lists, so a fill repaints the node and builds nothing.
//!
//! Slots are for display items (pages, imaging previews, the canvas picture when its content is shapes). A picture
//! that is a GPU texture is an `Item::Surface`.

use std::cell::RefCell;

use craft_ui_rt::{DisplayList, Item, Pos2, Rect, Vec2};

use super::canvas::Canvas;
use super::id::Id;
use super::record::PaintRecord;

/// A reserved place in one node's paint order: the node and the slot's key among that node's slots.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintSlot {
    node: Id,
    key: u64,
    /// The node's top-left corner in window coordinates when the slot was reserved: fills come in window
    /// coordinates and the node keeps its items in its own.
    origin: Vec2,
}

impl PaintSlot {
    /// Reserves the next slot of the look `canvas` records, clipped to `clip` (window coordinates). The looks of one
    /// run reserve their slots in the same order, so slot `n` names the same place in every look.
    pub(crate) fn reserve_in(canvas: &Canvas<'_>, record: &RefCell<PaintRecord>, node: Id, origin: Pos2, clip: Rect) -> PaintSlot {
        let key = {
            let mut record = record.borrow_mut();
            let key = record.slots;
            record.slots += 1;
            key
        };
        canvas.clipped(clip, || canvas.push(Item::Slot(key)));
        PaintSlot { node, key, origin: origin.to_vec2() }
    }

    /// Puts `items` (window coordinates) in the slot in place of what it held. The node repaints with them at the
    /// slot's place in its paint order once the calling build ends; nothing builds. Called from a build (a paint,
    /// an update or a body); outside one there is no runtime to reach and the fill is dropped.
    pub fn fill(self, items: Vec<Item>) {
        let (node, key, origin) = (self.node, self.key, self.origin);
        let mut list = DisplayList { items };
        for item in &mut list.items {
            super::items::translate(item, -origin);
        }
        crate::contract::context::log_build_read(move |cx| {
            if let Some(node) = crate::contract::rt::ids::node_of(cx, node) {
                cx.fill_slot(node, key, list);
            }
        });
    }
}
