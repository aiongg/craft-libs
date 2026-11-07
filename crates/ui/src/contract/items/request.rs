//! Requests to a collection's item step from outside it, taken at the collection's next pass.

use crate::runtime::egui::memory as store;
use egui::{Context, Id};

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
    store::Service::<RenameRequests<K>>::run(ctx, |requests| {
        requests.0.retain(|(id, _)| *id != collection);
        requests.0.push((collection, key.clone()));
    });
    crate::runtime::egui::repaint::request_of(ctx, crate::runtime::egui::pass::current_viewport(ctx));
}

/// The rename request addressed to `collection`, once.
pub(super) fn take_rename<K: Send + Sync + 'static>(ctx: &Context, collection: Id) -> Option<K> {
    store::Service::<RenameRequests<K>>::run(ctx, |requests| {
        let at = requests.0.iter().position(|(id, _)| *id == collection)?;
        Some(requests.0.remove(at).1)
    })
}
