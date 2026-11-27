//! The window's open layers as the layer queries read them (docs/contract-rt.md section 15.9): each layer child's
//! `OpenEntry` (`LayerKept.entry`), bottom first, found in the ui-rt tree. A query is tracked through the window's
//! open-layer cell, which the layer child's build writes when its entry changes and its owner writes when it stops
//! showing it, so a reader runs again exactly when the stack changes.

use craft_ui_rt::{BuildCx, NodeId, Runtime, WindowId, Writes as _};

use super::overlay::LayerScope;
use crate::contract::context::Context;
use crate::primitives::Id;
use crate::runtime::overlay::{LayerRole, OpenEntry, OpenFilter};

/// One open layer child: its node and its entry.
pub(crate) struct Open {
    pub(crate) node: NodeId,
    pub(crate) entry: OpenEntry,
    /// The frame time it opened at.
    pub(crate) since: Option<f64>,
}

/// The layer children of `window` that carry an entry, bottom first (layers in their order, each layer's children in
/// theirs). A layer child whose own build is running has no kept facts to give and is left out.
pub(crate) fn open(rt: &Runtime, window: WindowId) -> Vec<Open> {
    let Some(root) = rt.window_node(window) else { return Vec::new() };
    let mut open = Vec::new();
    for &layer in rt.children(root).iter().skip(1) {
        for &node in rt.children(layer) {
            if let Some(lk) = rt.local::<LayerScope>(node)
                && let Some(entry) = &lk.entry
            {
                open.push(Open { node, entry: entry.clone(), since: lk.since });
            }
        }
    }
    open
}

/// The layer child `node` lies in: its ancestor whose parent is one of the window's layers other than the content
/// layer. `None` for a node on the content layer.
pub(crate) fn layer_child_of(rt: &Runtime, node: NodeId) -> Option<NodeId> {
    let window = rt.window_of(node)?;
    let root = rt.window_node(window)?;
    let content = *rt.children(root).first()?;
    let mut at = node;
    loop {
        let parent = rt.parent(at)?;
        if rt.parent(parent) == Some(root) {
            return (parent != content).then_some(at);
        }
        at = parent;
    }
}

/// Whether `entry` counts for `filter`. A layer fading out counts for none.
pub(crate) fn counts(entry: &OpenEntry, filter: OpenFilter) -> bool {
    !entry.exiting
        && match filter {
            OpenFilter::Any => true,
            OpenFilter::Modal => entry.rank.is_modal(),
            OpenFilter::Keys => entry.takes_keys(),
            OpenFilter::Asks => entry.rank.is_modal() && entry.role != LayerRole::Progress,
        }
}

/// The top modal layer child of `window`, if a modal layer is open there.
pub(crate) fn top_modal(rt: &Runtime, window: WindowId) -> Option<Open> {
    open(rt, window).into_iter().rev().find(|o| counts(&o.entry, OpenFilter::Modal))
}

/// Whether `node` lies in the top modal layer child of its window.
pub(crate) fn in_top_modal(rt: &Runtime, node: NodeId) -> bool {
    let Some(window) = rt.window_of(node) else { return false };
    let Some(top) = top_modal(rt, window) else { return false };
    layer_child_of(rt, node) == Some(top.node)
}

/// Registers a read of the window's open-layer cell for the node `cx` builds.
pub(crate) fn track(cx: &mut BuildCx<'_>) {
    if let Some(ctx) = cx.provided::<Context>() {
        let cell = ctx.layers_cell(cx.store_mut());
        cell.get(cx);
    }
}

/// The open layers of the window changed (a layer child's entry was set, changed or left): every layer query reader
/// runs again.
pub(crate) fn note_changed(cx: &mut BuildCx<'_>) {
    if let Some(ctx) = cx.provided::<Context>() {
        let cell = ctx.layers_cell(cx.store_mut());
        let revision = *cx.store_mut().peek(cell) + 1;
        cell.set(cx, revision);
    }
}

/// The entry of the layer `id` names, among `open`.
pub(crate) fn find(open: &[Open], id: Id) -> Option<&Open> {
    open.iter().rev().find(|o| o.entry.id == id)
}
