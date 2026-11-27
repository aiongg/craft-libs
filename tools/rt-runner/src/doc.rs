//! The document cell and the edits components apply to it.

use std::sync::Arc;

use crate::fixture::{Doc, Op};
use craft_ui_rt::{State, Writes};

/// The document as the components read it: one cell, edited in place.
pub type DocCell = State<Arc<Doc>>;

/// Applies `op`; readers of the cell rebuild.
pub fn edit(doc: DocCell, cx: &mut impl Writes, op: Op) {
    doc.update(cx, |d| Arc::make_mut(d).apply(op));
}

/// A layer whose name may change (PhotoCraft: not the Background layer).
pub fn renamable(layer: &crate::fixture::Layer) -> bool {
    layer.id != 0
}
