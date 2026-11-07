//! Context data: `IdTypeMap` temp entries, the per-service state behind one lock each, and id minting.

use egui::util::IdTypeMap;
use egui::util::id_type_map::RawKey;
use egui::{Context, Id};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

/// Context data: the map the runtime's slots live in.
pub(crate) type Map = IdTypeMap;

/// A boxed entry moved out of context data. The caller drops it later, outside every lock.
pub(crate) type Removed = Box<dyn Any + Send + Sync>;

/// Moves one typed slot out of the map as the box it is stored in. `IdTypeMap::remove_temp::<T>` would run
/// `T::default()` and drop the placeholder under the data lock; the raw removal runs no app code.
pub(crate) fn take_raw<T: Any + Send + Sync>(data: &mut Map, id: Id) -> Option<Removed> {
    data.remove_temp_raw(RawKey::new::<T>(id))
}

/// Moves the slot of type `T` under `id` out of context data.
pub(crate) fn take<T: Any + Send + Sync>(ctx: &Context, id: Id) -> Option<T> {
    let boxed = ctx.data_mut(|data| take_raw::<T>(data, id))?;
    boxed.downcast::<T>().ok().map(|value| *value)
}

/// Stores `value` under `id`. The entry it replaces is dropped after the data lock is released.
pub(crate) fn put<T: Any + Clone + Send + Sync>(ctx: &Context, id: Id, value: T) {
    let replaced = ctx.data_mut(|data| {
        let old = take_raw::<T>(data, id);
        data.insert_temp(id, value);
        old
    });
    drop(replaced);
}

/// A copy of the slot under `id` without running slot code under the data lock: the entry moves out, is
/// cloned and moves back.
pub(crate) fn peek_clone<T: Any + Clone + Send + Sync>(ctx: &Context, id: Id) -> Option<T> {
    let moved = take::<T>(ctx, id)?;
    let copy = moved.clone();
    ctx.data_mut(|data| data.insert_temp(id, moved));
    Some(copy)
}

/// Whether a slot of type `T` is stored under `id`.
pub(crate) fn exists<T: Any + Send + Sync>(ctx: &Context, id: Id) -> bool {
    ctx.data(|data| data.get_temp_raw(RawKey::new::<T>(id)).is_some())
}

/// Runs `f` on the context's data under its lock; `f` runs no app code and makes no call that reaches the context.
pub(crate) fn locked<R>(ctx: &Context, f: impl FnOnce(&mut Map) -> R) -> R {
    ctx.data_mut(f)
}

/// The id derived from `source` alone, with no parent.
pub(crate) fn unique(source: impl Hash + std::fmt::Debug) -> Id {
    Id::unique(source)
}

/// A service's state in context data. Cloning shares the state.
///
/// The context's data lock is held only to fetch the handle. The service's own lock is held while `with`
/// runs its closure, so the closure runs no app code and makes no call that reaches the context (7.2, the
/// lock rule).
#[derive(Default)]
pub(crate) struct Service<S>(Arc<Mutex<S>>);

impl<S> Clone for Service<S> {
    fn clone(&self) -> Self {
        Service(Arc::clone(&self.0))
    }
}

/// Folds the bytes of a `TypeId`, which is already a hash.
#[derive(Default)]
struct TypeIdHasher(u64);

impl Hasher for TypeIdHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.0 ^= u64::from_le_bytes(word);
        }
    }
}

type HandleCache = HashMap<TypeId, Box<dyn Any>, BuildHasherDefault<TypeIdHasher>>;

thread_local! {
    /// The service handles of the passes between their begin and end hooks on this thread, innermost last: a
    /// lookup in the context's data happens once per service type and pass. A pass is the only user of its
    /// thread's context while it runs.
    static HANDLES: RefCell<Vec<HandleCache>> = const { RefCell::new(Vec::new()) };
}

/// Opens the handle cache of a pass; the begin hook calls it.
pub(crate) fn open_handles() {
    HANDLES.with(|caches| caches.borrow_mut().push(HandleCache::default()));
}

/// Closes the handle cache of the innermost pass; the end hook calls it.
pub(crate) fn close_handles() {
    HANDLES.with(|caches| caches.borrow_mut().pop());
}

impl<S: Default + Send + 'static> Service<S> {
    /// The context's service of type `S`, created on first use.
    pub(crate) fn of(ctx: &Context) -> Service<S> {
        let cached = HANDLES.with(|caches| {
            caches.borrow().last().and_then(|cache| cache.get(&TypeId::of::<S>())).and_then(|handle| handle.downcast_ref::<Service<S>>()).cloned()
        });
        if let Some(handle) = cached {
            return handle;
        }
        static SLOT: OnceLock<Id> = OnceLock::new();
        let slot = *SLOT.get_or_init(|| unique("craft_ui.service"));
        let handle = locked(ctx, |data| data.get_temp_mut_or_default::<Service<S>>(slot).clone());
        HANDLES.with(|caches| {
            if let Some(cache) = caches.borrow_mut().last_mut() {
                cache.insert(TypeId::of::<S>(), Box::new(handle.clone()));
            }
        });
        handle
    }

    /// Runs `f` on the state under the service's lock.
    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut S) -> R) -> R {
        f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// Runs `f` on the context's service of type `S`.
pub(crate) fn with<S: Default + Send + 'static, R>(ctx: &Context, f: impl FnOnce(&mut S) -> R) -> R {
    Service::<S>::of(ctx).with(f)
}

/// Runs `f` on a child `Ui` of `ui` whose auto ids start from `salt`: equal widgets under different salts never meet.
#[cfg(feature = "testing")]
pub(crate) fn scoped_by<R>(ui: &mut egui::Ui, salt: impl std::hash::Hash + std::fmt::Debug, f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.push_id(salt, f).inner
}
