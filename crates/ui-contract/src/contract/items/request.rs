//! Requests to a collection's item step from outside it, taken at the collection's next run, and the
//! lead position the item step leaves for readers outside it. Both live in a service of the window's
//! `Context`, keyed by the collection's `Id`.

use crate::contract::Context;
use crate::primitives::Id;
use std::collections::HashMap;

/// The rename requests not yet taken: the collection and the key to rename.
struct RenameRequests<K>(Vec<(Id, K)>);

impl<K> Default for RenameRequests<K> {
    fn default() -> Self {
        RenameRequests(Vec::new())
    }
}

/// Asks the collection `collection` (the id it is built with, `Props::id`) to start renaming the item `key`. At its
/// next item step the lead moves to the item, the item is revealed and its rename field opens, as for the rename
/// key. A request for a key the source lacks, or for a collection that cannot rename the item, is dropped. A second
/// request for the same collection replaces the first.
pub fn request_rename<K: Clone + Send + Sync + 'static>(ctx: &Context, collection: Id, key: &K) {
    ctx.service::<RenameRequests<K>, _>(|requests| {
        requests.0.retain(|(id, _)| *id != collection);
        requests.0.push((collection, key.clone()));
    });
    ctx.request_build_of(collection);
}

/// The rename request addressed to `collection`, once.
pub(super) fn take_rename<K: Send + Sync + 'static>(ctx: &Context, collection: Id) -> Option<K> {
    ctx.service::<RenameRequests<K>, _>(|requests| {
        let at = requests.0.iter().position(|(id, _)| *id == collection)?;
        Some(requests.0.remove(at).1)
    })
}

/// The rename request addressed to `collection`, left in place for the run's own step to take. L-154's kept row
/// covers a requested rename too, so a row the filter excludes is still listed while its rename starts.
pub(super) fn peek_rename<K: Clone + Send + Sync + 'static>(ctx: &Context, collection: Id) -> Option<K> {
    ctx.service::<RenameRequests<K>, _>(|requests| requests.0.iter().find(|(id, _)| *id == collection).map(|(_, key)| key.clone()))
}

/// The lead's position of each collection as its item step left it at the end of its last run.
#[derive(Default)]
struct LeadPositions(HashMap<Id, Option<usize>>);

pub(super) fn record_lead(ctx: &Context, collection: Id, at: Option<usize>) {
    ctx.service::<LeadPositions, _>(|leads| leads.0.insert(collection, at));
}

/// The position of the lead of `collection` after its last item step, `None` while it has none.
#[doc(hidden)]
pub fn lead_position(ctx: &Context, collection: Id) -> Option<usize> {
    ctx.service::<LeadPositions, _>(|leads| leads.0.get(&collection).copied().flatten())
}
