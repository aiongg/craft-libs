//! `IdTypeMap` temp entries (`insert_temp`, `remove_temp_raw`).

use egui::util::IdTypeMap;
use egui::util::id_type_map::RawKey;
use egui::{Context, Id};
use std::any::Any;

/// A boxed entry moved out of context data. The caller drops it later, outside every lock.
pub(crate) type Removed = Box<dyn Any + Send + Sync>;

/// Moves one typed slot out of the map as the box it is stored in. `IdTypeMap::remove_temp::<T>` would run
/// `T::default()` and drop the placeholder under the data lock; the raw removal runs no app code.
pub(crate) fn take_raw<T: Any + Send + Sync>(data: &mut IdTypeMap, id: Id) -> Option<Removed> {
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

/// Whether a value of type `T` is stored under `id`. For plain-data markers: it clones the value under the lock.
pub(crate) fn contains<T: Any + Clone + Send + Sync>(ctx: &Context, id: Id) -> bool {
    ctx.data(|data| data.get_temp::<T>(id).is_some())
}
