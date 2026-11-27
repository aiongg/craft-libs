//! Inner controls folded into their owner's access node (`PaintUi::inner`): the owner's node stands
//! for the control, carries the control's value text when the owner's `access` sets none, and the
//! control reports no node of its own (`Role::Merged`).
//!
//! The owner asks for the fold in its paint, before the control runs; the control's run offers its
//! value text before the owner reports. Both live in the window's `Folds` service, keyed by craft
//! `Id`, so a kept control's text still reaches an owner that runs again without it. A folded control's
//! node holds a [`FoldEntry`], which takes the control's entries out when the node goes.

use std::collections::HashMap;

use crate::contract::Context;
use crate::primitives::Id;

/// The window's folds: which owner each inner control folds into, and the value text each folded
/// control last offered its owner.
#[derive(Default)]
struct Folds {
    owners: HashMap<Id, Id>,
    offered: HashMap<Id, Folded>,
    /// Fold requests made since the window opened (`testing::access::fold_requests`).
    requests: usize,
}

/// What a folded control hands its owner's node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Folded {
    /// The control.
    pub inner: Id,
    /// Its value text.
    pub value: Option<String>,
}

/// The control `inner` folds into `owner`'s node.
pub(crate) fn request(ctx: &Context, inner: Id, owner: Id) {
    ctx.service::<Folds, _>(|folds| {
        folds.owners.insert(inner, owner);
        folds.requests += 1;
    });
}

/// The fold requests made in the window of `ctx` since it opened.
pub(crate) fn requests(ctx: &Context) -> usize {
    ctx.service::<Folds, _>(|folds| folds.requests)
}

/// The owner `inner` folds into, if any.
pub(crate) fn owner_of(ctx: &Context, inner: Id) -> Option<Id> {
    ctx.service::<Folds, _>(|folds| folds.owners.get(&inner).copied())
}

/// The folded control's report for its owner's node, replacing what it offered before.
pub(crate) fn offer(ctx: &Context, owner: Id, folded: Folded) {
    ctx.service::<Folds, _>(|folds| {
        folds.offered.insert(owner, folded);
    });
}

/// What the control folded into `owner` last offered.
pub(crate) fn offered(ctx: &Context, owner: Id) -> Option<Folded> {
    ctx.service::<Folds, _>(|folds| folds.offered.get(&owner).cloned())
}

/// A folded control's place in its window's `Folds`, held by the control's node: dropped with the node, it
/// removes the control's owner and the text it offered.
pub(crate) struct FoldEntry {
    ctx: Context,
    inner: Id,
}

/// The entry the folded control `inner` keeps on its node.
pub(crate) fn entry(ctx: &Context, inner: Id) -> FoldEntry {
    FoldEntry { ctx: ctx.clone(), inner }
}

impl Drop for FoldEntry {
    fn drop(&mut self) {
        let inner = self.inner;
        self.ctx.service::<Folds, _>(|folds| {
            folds.owners.remove(&inner);
            folds.offered.retain(|_, folded| folded.inner != inner);
        });
    }
}
