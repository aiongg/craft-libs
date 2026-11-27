//! The document cell the components read and the outbox they send edits through.

use std::sync::Arc;

use crate::model::{Doc, Layer, Op};
use craft_ui_rt::{Reads, State, Store, Writes};

/// The panel's two cells: the document as the host last gave it, and the edits the user made since the host
/// last took them. The panel never writes the document; the host applies each `Op` and sets the result.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DocCell {
    doc: State<Arc<Doc>>,
    outbox: State<Vec<Op>>,
}

impl DocCell {
    pub fn create(store: &mut Store, doc: Doc) -> DocCell {
        DocCell { doc: store.create(Arc::new(doc)), outbox: store.create(Vec::new()) }
    }

    pub fn get(self, cx: &mut impl Reads) -> Arc<Doc> {
        self.doc.get(cx)
    }

    pub fn with<R>(self, cx: &mut impl Reads, f: impl FnOnce(&Arc<Doc>) -> R) -> R {
        self.doc.with(cx, f)
    }

    /// The document without recording a read.
    pub fn peek(self, store: &Store) -> &Arc<Doc> {
        store.peek(self.doc)
    }

    /// The host's new document; readers rebuild.
    pub fn set_doc(self, cx: &mut impl Writes, doc: Doc) {
        self.doc.set(cx, Arc::new(doc));
    }

    /// Changes the document in place (a host applying edits, or setting thumbnails).
    pub fn update_doc(self, cx: &mut impl Writes, f: impl FnOnce(&mut Doc)) {
        self.doc.update(cx, |d| f(Arc::make_mut(d)));
    }

    /// Reads the outbox, so the reader runs again when an edit arrives.
    pub fn has_edits(self, cx: &mut impl Reads) -> bool {
        self.outbox.with(cx, |q| !q.is_empty())
    }

    /// The edits since the last call, oldest first. Writes the outbox only when it holds edits.
    pub fn take_edits(self, cx: &mut impl Writes) -> Vec<Op> {
        let store = cx.store_mut();
        if store.peek(self.outbox).is_empty() {
            return Vec::new();
        }
        let mut taken = Vec::new();
        self.outbox.update(store, |q| std::mem::swap(q, &mut taken));
        taken
    }
}

/// Sends `op` to the host; readers of the outbox run.
pub fn edit(doc: DocCell, cx: &mut impl Writes, op: Op) {
    doc.outbox.update(cx, |q| q.push(op));
}

/// A layer whose name may change (PhotoCraft: not the Background layer).
pub fn renamable(layer: &Layer) -> bool {
    layer.id != 0
}
