//! Derived values for code with a context and no node of its own (a body, a service): data computed from inputs and
//! kept across builds. [`derived`] keeps one value per id and value type in the window's `Context`, and builds it
//! again only when its key changes. The key implements [`MemoProps`], so it can only be made of checked inputs
//! (`Tracked` revisions, framework revisions, plain values), and a kept value is never stale while the key compiles.
//! Code that builds in a node keeps its value in the node instead (`Ui::derived`), so it goes with the node.

use crate::contract::{Context, MemoKey, MemoProps};
use crate::primitives::Id;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// How many values a window keeps for code with no node; past it the least recently asked for goes.
const KEPT: usize = 256;

/// A kept value: the key it was built from and when it was last asked for.
struct Entry {
    key: u128,
    value: Arc<dyn Any + Send + Sync>,
    used: u64,
}

/// The kept values of one window.
#[derive(Default)]
struct Store {
    entries: HashMap<Id, Entry>,
    /// Counts the asks, to order the entries by their last.
    clock: u64,
}

/// The value under `id` derived from `key`: the kept one while `key` is the one it was built from, else `build`'s,
/// which is kept in its place. One value is kept per id and value type `V`.
///
/// While the key is unchanged and the value is kept, every call returns the same `Arc` (`Arc::ptr_eq` holds).
pub fn derived<K: MemoProps, V: Send + Sync + 'static>(ctx: &Context, id: Id, key: &K, build: impl FnOnce() -> V) -> Arc<V> {
    let mut memo = MemoKey::new();
    memo.feed_hash(&K::identity());
    key.key(&mut memo);
    let key = memo.finish();
    let slot = id.with(TypeId::of::<V>());
    let kept = ctx.service::<Store, _>(|store| {
        store.clock += 1;
        let clock = store.clock;
        let entry = store.entries.get_mut(&slot).filter(|entry| entry.key == key)?;
        entry.used = clock;
        Arc::clone(&entry.value).downcast::<V>().ok()
    });
    if let Some(value) = kept {
        return value;
    }
    // Built outside the store, so a build may derive values of its own.
    let value = Arc::new(build());
    let dropped = ctx.service::<Store, _>(|store| {
        let used = store.clock;
        let dropped = store.entries.insert(slot, Entry { key, value: Arc::clone(&value) as Arc<dyn Any + Send + Sync>, used });
        let oldest = (store.entries.len() > KEPT).then(|| store.entries.iter().min_by_key(|(_, entry)| entry.used).map(|(id, _)| *id)).flatten();
        (dropped, oldest.and_then(|id| store.entries.remove(&id)))
    });
    // The replaced values drop outside the store: a value's drop may reach the window's services.
    drop(dropped);
    value
}
